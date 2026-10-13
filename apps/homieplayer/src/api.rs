//! Talking to the media server.
//!
//! Two kinds of servers are supported:
//! * **API servers** answer `GET /api/info` with JSON and serve the catalog at
//!   `GET /api/library`, optionally behind `POST /api/login` (see README.md).
//! * **Plain file servers** (nginx/Apache/Caddy directory listings): the
//!   listing is crawled and movies/series are guessed from folder and file names.
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::Read;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;

use crate::util::percent_decode;

pub const VIDEO_EXTS: &[&str] = &[
    "mp4", "mkv", "webm", "avi", "mov", "m4v", "flv", "wmv", "mpg", "mpeg", "ogv", "ts", "3gp",
];
const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png", "webp"];

// ------------------------------------------------------------------ model

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Kind {
    #[default]
    Movie,
    Series,
}

#[derive(Clone, Default, Deserialize)]
#[serde(default)]
pub struct Item {
    pub id: String,
    pub title: String,
    pub year: Option<u32>,
    pub overview: String,
    pub poster: Option<String>,
    pub backdrop: Option<String>,
    pub genres: Vec<String>,
    /// Age rating such as "PG-13".
    pub rating: Option<String>,
    /// Seconds.
    pub duration: Option<f64>,
    /// Unix time the item was added to the library.
    pub added: Option<i64>,
    /// Movies only.
    pub stream: Option<String>,
    /// Series only.
    pub seasons: Vec<Season>,
    #[serde(skip)]
    pub kind: Kind,
}

#[derive(Clone, Default, Deserialize)]
#[serde(default)]
pub struct Season {
    pub number: u32,
    pub title: Option<String>,
    pub episodes: Vec<Episode>,
}

impl Season {
    pub fn label(&self) -> String {
        self.title.clone().unwrap_or_else(|| {
            if self.number == 0 {
                "Specials".into()
            } else {
                format!("Season {}", self.number)
            }
        })
    }
}

#[derive(Clone, Default, Deserialize)]
#[serde(default)]
pub struct Episode {
    pub id: String,
    pub number: u32,
    pub title: String,
    pub overview: String,
    pub still: Option<String>,
    pub duration: Option<f64>,
    pub stream: String,
}

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct Library {
    pub movies: Vec<Item>,
    pub series: Vec<Item>,
}

impl Library {
    pub fn all(&self) -> impl Iterator<Item = &Item> {
        self.movies.iter().chain(&self.series)
    }

    pub fn find(&self, id: &str) -> Option<&Item> {
        self.all().find(|i| i.id == id)
    }

    /// Series, season index and episode index for an episode id.
    pub fn find_episode(&self, ep_id: &str) -> Option<(&Item, usize, usize)> {
        self.series.iter().find_map(|s| {
            s.seasons.iter().enumerate().find_map(|(si, season)| {
                let ei = season.episodes.iter().position(|e| e.id == ep_id)?;
                Some((s, si, ei))
            })
        })
    }

    /// Genres ordered by how many titles have them.
    pub fn genres(&self) -> Vec<String> {
        let mut count: HashMap<&str, usize> = HashMap::new();
        for g in self.all().flat_map(|i| &i.genres) {
            *count.entry(g.as_str()).or_default() += 1;
        }
        let mut v: Vec<_> = count.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        v.into_iter().map(|(g, _)| g.to_owned()).collect()
    }

    fn finish(&mut self, base: &str) {
        let fix = |u: &mut Option<String>| {
            if let Some(s) = u.as_mut() {
                *s = join(base, s);
            }
        };
        for (items, kind) in [
            (&mut self.movies, Kind::Movie),
            (&mut self.series, Kind::Series),
        ] {
            for it in items.iter_mut() {
                it.kind = kind;
                fix(&mut it.poster);
                fix(&mut it.backdrop);
                fix(&mut it.stream);
                if it.id.is_empty() {
                    it.id = it.stream.clone().unwrap_or_else(|| it.title.clone());
                }
                // servers often number movies, series and episodes separately
                let tag = if kind == Kind::Movie { "movie" } else { "series" };
                it.id = format!("{tag}:{}", it.id);
                it.seasons.sort_by_key(|s| s.number);
                for s in &mut it.seasons {
                    s.episodes.sort_by_key(|e| e.number);
                    for e in &mut s.episodes {
                        fix(&mut e.still);
                        e.stream = join(base, &e.stream);
                        if e.id.is_empty() {
                            e.id = e.stream.clone();
                        }
                        e.id = format!("episode:{}", e.id);
                    }
                }
            }
        }
        self.movies.retain(|m| m.stream.is_some());
        self.series
            .retain(|s| s.seasons.iter().any(|x| !x.episodes.is_empty()));
        let by_title = |a: &Item, b: &Item| a.title.to_lowercase().cmp(&b.title.to_lowercase());
        self.movies.sort_by(by_title);
        self.series.sort_by(by_title);
    }
}

// ----------------------------------------------------------------- client

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Api,
    Directory,
}

#[derive(Clone, Debug)]
pub struct ServerInfo {
    pub name: String,
    pub auth: bool,
    pub mode: Mode,
}

#[derive(Deserialize)]
struct InfoJson {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    auth: bool,
}

#[derive(Deserialize)]
pub struct Session {
    pub token: String,
    #[serde(default)]
    pub name: Option<String>,
}

/// The server rejected our token; the user has to sign in again.
#[derive(Debug)]
pub struct Unauthorized;

impl std::fmt::Display for Unauthorized {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("session expired, please sign in again")
    }
}
impl std::error::Error for Unauthorized {}

#[derive(Clone)]
pub struct Client {
    pub base: String,
    pub info: ServerInfo,
    pub token: Option<String>,
    agent: ureq::Agent,
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(5))
        .timeout_read(Duration::from_secs(30))
        .user_agent(concat!("HomiePlayer/", env!("CARGO_PKG_VERSION")))
        .build()
}

/// "192.168.1.5:8080/" -> "http://192.168.1.5:8080"
pub fn normalize(input: &str) -> String {
    let s = input.trim().trim_end_matches('/');
    if s.contains("://") {
        s.to_owned()
    } else {
        format!("http://{s}")
    }
}

fn origin(url: &str) -> &str {
    let start = url.find("://").map_or(0, |i| i + 3);
    url[start..].find('/').map_or(url, |i| &url[..start + i])
}

/// Resolve a possibly-relative URL against the server base.
pub fn join(base: &str, rel: &str) -> String {
    if rel.contains("://") {
        rel.to_owned()
    } else if let Some(r) = rel.strip_prefix('/') {
        format!("{}/{r}", origin(base))
    } else {
        format!("{}/{rel}", base.trim_end_matches('/'))
    }
}

fn status_err(e: ureq::Error, what: &str) -> anyhow::Error {
    match e {
        ureq::Error::Status(401 | 403, _) => anyhow!(Unauthorized),
        ureq::Error::Status(code, r) => {
            anyhow!("{what}: server answered {code} {}", r.status_text())
        }
        ureq::Error::Transport(t) => anyhow!("{what}: {t}"),
    }
}

impl Client {
    /// Find out what kind of server lives at `address`.
    pub fn connect(address: &str) -> Result<Self> {
        let base = normalize(address);
        if base.len() <= "http://".len() {
            bail!("enter the server's address, e.g. 192.168.1.20:8080");
        }
        let agent = agent();
        let api = agent
            .get(&format!("{base}/api/info"))
            .call()
            .ok()
            .and_then(|r| r.into_json::<InfoJson>().ok());
        let host = base.split("://").nth(1).unwrap_or(&base).to_owned();
        let info = match api {
            Some(i) => ServerInfo {
                name: i.name.unwrap_or(host),
                auth: i.auth,
                mode: Mode::Api,
            },
            None => {
                // not an API server: is it at least a file listing?
                agent
                    .get(&format!("{base}/"))
                    .call()
                    .map_err(|e| status_err(e, "can't reach the server"))?;
                ServerInfo {
                    name: host,
                    auth: false,
                    mode: Mode::Directory,
                }
            }
        };
        Ok(Self {
            base,
            info,
            token: None,
            agent,
        })
    }

    pub fn auth_header(&self) -> Option<String> {
        self.token.as_ref().map(|t| format!("Bearer {t}"))
    }

    fn get(&self, url: &str) -> ureq::Request {
        let r = self.agent.get(url);
        match self.auth_header() {
            Some(h) => r.set("Authorization", &h),
            None => r,
        }
    }

    pub fn login(&self, username: &str, password: &str) -> Result<Session> {
        let r = self
            .agent
            .post(&format!("{}/api/login", self.base))
            .send_json(serde_json::json!({ "username": username, "password": password }));
        match r {
            Ok(r) => r.into_json().context("unexpected login reply"),
            Err(ureq::Error::Status(400 | 401 | 403, _)) => bail!("Wrong username or password"),
            Err(e) => Err(status_err(e, "sign-in failed")),
        }
    }

    /// Best-effort; the session is forgotten locally either way.
    pub fn logout(&self) {
        if self.info.mode == Mode::Api && self.token.is_some() {
            let url = format!("{}/api/logout", self.base);
            let mut r = self.agent.post(&url).timeout(Duration::from_secs(3));
            if let Some(h) = self.auth_header() {
                r = r.set("Authorization", &h);
            }
            let _ = r.call();
        }
    }

    pub fn library(&self) -> Result<Library> {
        let mut lib = match self.info.mode {
            Mode::Api => self
                .get(&format!("{}/api/library", self.base))
                .call()
                .map_err(|e| status_err(e, "can't load the library"))?
                .into_json::<Library>()
                .context("the server sent a library this app doesn't understand")?,
            Mode::Directory => self.crawl()?,
        };
        lib.finish(&self.base);
        Ok(lib)
    }

    /// Download an image (poster, backdrop) with our credentials.
    pub fn fetch(&self, url: &str) -> Result<Vec<u8>> {
        let mut r = self.agent.get(url);
        if url.starts_with(&self.base) {
            if let Some(h) = self.auth_header() {
                r = r.set("Authorization", &h);
            }
        }
        let r = r.call().map_err(|e| status_err(e, "image"))?;
        let mut buf = Vec::new();
        r.into_reader().take(25 << 20).read_to_end(&mut buf)?;
        Ok(buf)
    }

    // ------------------------------------------------------- directory mode

    fn crawl(&self) -> Result<Library> {
        let root = format!("{}/", self.base);
        let mut queue = VecDeque::from([(root.clone(), 0usize)]);
        let mut seen = HashSet::new();
        let mut videos = Vec::new();
        let mut images: HashMap<String, Vec<(String, String)>> = HashMap::new();
        while let Some((dir, depth)) = queue.pop_front() {
            if seen.len() > 5000 || !seen.insert(dir.clone()) {
                continue;
            }
            let html = match self.get(&dir).call() {
                Ok(r) => r.into_string().unwrap_or_default(),
                Err(e) if depth == 0 => return Err(status_err(e, "can't list the server")),
                Err(_) => continue,
            };
            for href in hrefs(&html) {
                let url = resolve_href(&dir, &href);
                if url.len() <= dir.len() || !url.starts_with(&dir) {
                    continue; // parent, sorting links, other hosts
                }
                if url.ends_with('/') {
                    if depth < 6 {
                        queue.push_back((url, depth + 1));
                    }
                    continue;
                }
                let name = percent_decode(url.rsplit('/').next().unwrap_or_default());
                let (stem, ext) = split_ext(&name);
                if VIDEO_EXTS.contains(&ext.as_str()) {
                    let rel = percent_decode(&dir[root.len()..]);
                    let dirs = rel
                        .split('/')
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned)
                        .collect();
                    videos.push(Found {
                        url,
                        dir: dir.clone(),
                        dirs,
                        stem: stem.to_owned(),
                    });
                } else if IMAGE_EXTS.contains(&ext.as_str()) {
                    images
                        .entry(dir.clone())
                        .or_default()
                        .push((stem.to_lowercase(), url));
                }
            }
        }
        if videos.is_empty() {
            bail!(
                "this server is a website, not a media library: it has no /api/info endpoint \
                 and isn't a plain file listing either (see README.md for the API to add)"
            );
        }
        Ok(classify(&root, videos, &images))
    }
}

struct Found {
    url: String,
    dir: String,
    /// decoded folder names below the server root
    dirs: Vec<String>,
    stem: String,
}

fn hrefs(html: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(i) = rest.find("href=") {
        rest = &rest[i + 5..];
        let Some(q) = rest.chars().next().filter(|c| *c == '"' || *c == '\'') else {
            continue;
        };
        let Some(end) = rest[1..].find(q) else { break };
        out.push(rest[1..1 + end].replace("&amp;", "&"));
        rest = &rest[1 + end..];
    }
    out
}

fn resolve_href(dir: &str, href: &str) -> String {
    let href = href.split(['?', '#']).next().unwrap_or_default();
    if href.contains("://") {
        href.to_owned()
    } else if let Some(r) = href.strip_prefix("//") {
        format!("{}://{r}", dir.split("://").next().unwrap_or("http"))
    } else if href.starts_with('/') {
        format!("{}{href}", origin(dir))
    } else {
        format!("{dir}{}", href.trim_start_matches("./"))
    }
}

fn split_ext(name: &str) -> (&str, String) {
    match name.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s, e.to_lowercase()),
        _ => (name, String::new()),
    }
}

const SHOW_DIRS: &[&str] = &["tv", "tv shows", "tvshows", "shows", "series", "tv series"];
const MOVIE_DIRS: &[&str] = &["movies", "movie", "films", "film"];
const JUNK: &[&str] = &[
    "2160p", "1080p", "1080i", "720p", "576p", "480p", "4k", "uhd", "bluray", "blu-ray", "bdrip",
    "brrip", "webrip", "web-dl", "webdl", "web", "hdtv", "dvdrip", "hdrip", "remux", "x264",
    "x265", "h264", "h265", "hevc", "avc", "xvid", "hdr", "hdr10", "dv", "10bit", "proper",
    "repack", "extended", "unrated", "aac", "ac3", "dts", "ddp5", "atmos", "imax",
];

fn is_season_dir(name: &str) -> Option<u32> {
    let l = name.to_lowercase();
    if l == "specials" {
        return Some(0);
    }
    let n = l.strip_prefix("season").or_else(|| l.strip_prefix('s'))?;
    n.trim_matches(|c: char| c == ' ' || c == '_' || c == '.')
        .parse()
        .ok()
}

/// "Show.Name.S02E05.Title.1080p" -> (2, 5, "Show.Name.", ".Title.1080p")
fn parse_episode(stem: &str) -> Option<(u32, u32, &str, &str)> {
    let b = stem.as_bytes();
    let digits = |from: usize| b[from..].iter().take_while(|c| c.is_ascii_digit()).count();
    let num = |from: usize, n: usize| stem[from..from + n].parse::<u32>().ok();
    for i in 0..b.len() {
        let boundary = i == 0 || !b[i - 1].is_ascii_alphanumeric();
        if !boundary {
            continue;
        }
        // S01E02 / s1e2 / S01.E02
        if b[i].eq_ignore_ascii_case(&b's') {
            let sn = digits(i + 1);
            if (1..=2).contains(&sn) {
                let mut j = i + 1 + sn;
                if j < b.len() && matches!(b[j], b'.' | b' ' | b'_' | b'-') {
                    j += 1;
                }
                if j < b.len() && b[j].eq_ignore_ascii_case(&b'e') {
                    let en = digits(j + 1);
                    if (1..=3).contains(&en) {
                        let end = j + 1 + en;
                        return Some((num(i + 1, sn)?, num(j + 1, en)?, &stem[..i], &stem[end..]));
                    }
                }
            }
        }
        // 1x02
        let sn = digits(i);
        if (1..=2).contains(&sn) && b.get(i + sn).is_some_and(|c| c.eq_ignore_ascii_case(&b'x')) {
            let en = digits(i + sn + 1);
            if (2..=3).contains(&en) {
                let end = i + sn + 1 + en;
                return Some((num(i, sn)?, num(i + sn + 1, en)?, &stem[..i], &stem[end..]));
            }
        }
    }
    None
}

/// "2.Fast.2.Furious.2003.2160p" -> ("2 Fast 2 Furious", Some(2003))
pub fn clean_title(raw: &str) -> (String, Option<u32>) {
    let mut s = String::with_capacity(raw.len());
    let mut depth = 0;
    for c in raw.chars() {
        match c {
            '[' | '{' => depth += 1,
            ']' | '}' => depth = (depth - 1).max(0),
            '.' | '_' | '(' | ')' if depth == 0 => s.push(' '),
            c if depth == 0 => s.push(c),
            _ => {}
        }
    }
    let mut words = Vec::new();
    let mut year = None;
    for (i, w) in s.split_whitespace().enumerate() {
        let lw = w.to_lowercase();
        if i > 0 && w.len() == 4 {
            if let Ok(y @ 1900..=2099) = w.parse::<u32>() {
                year = Some(y);
                break;
            }
        }
        if i > 0 && JUNK.contains(&lw.as_str()) {
            break;
        }
        words.push(w);
    }
    while words.last().is_some_and(|w| *w == "-") {
        words.pop();
    }
    while words.first().is_some_and(|w| *w == "-") {
        words.remove(0);
    }
    (words.join(" "), year)
}

fn pick_image(images: Option<&Vec<(String, String)>>, names: &[&str]) -> Option<String> {
    let imgs = images?;
    names
        .iter()
        .find_map(|n| imgs.iter().find(|(s, _)| s == n).map(|(_, u)| u.clone()))
}

fn classify(
    root: &str,
    videos: Vec<Found>,
    images: &HashMap<String, Vec<(String, String)>>,
) -> Library {
    let mut lib = Library::default();
    let mut shows: HashMap<String, Item> = HashMap::new();
    for v in videos {
        let category = v.dirs.first().map(|d| d.to_lowercase()).unwrap_or_default();
        let in_movies = MOVIE_DIRS.contains(&category.as_str());
        let in_shows = SHOW_DIRS.contains(&category.as_str());
        let season_dir = v.dirs.iter().rposition(|d| is_season_dir(d).is_some());
        let ep = parse_episode(&v.stem);
        let dir_url = |upto: usize| {
            // URL of v.dirs[..upto]; v.dir is the URL of all of them
            let mut u = v.dir.clone();
            for _ in upto..v.dirs.len() {
                u.truncate(u[..u.len() - 1].rfind('/').map_or(u.len(), |i| i + 1));
            }
            u
        };

        if in_movies || !(ep.is_some() || in_shows || season_dir.is_some()) {
            let (mut title, mut year) = clean_title(&v.stem);
            let own_folder = !v.dirs.is_empty() && v.dirs.len() > usize::from(in_movies);
            if own_folder {
                let (t, y) = clean_title(v.dirs.last().unwrap());
                year = year.or(y);
                if title.is_empty() {
                    title = t;
                }
            }
            if title.is_empty() {
                title = v.stem.clone();
            }
            let stem = v.stem.to_lowercase();
            let imgs = images.get(&v.dir);
            let (poster_names, backdrop_names): (Vec<&str>, Vec<&str>) = if own_folder {
                (
                    vec![&stem, "poster", "folder", "cover"],
                    vec!["fanart", "backdrop", "background"],
                )
            } else {
                (vec![&stem], vec![])
            };
            let fanart = format!("{stem}-fanart");
            let poster2 = format!("{stem}-poster");
            lib.movies.push(Item {
                id: v.url.clone(),
                title,
                year,
                poster: pick_image(imgs, &poster_names).or_else(|| pick_image(imgs, &[&poster2])),
                backdrop: pick_image(imgs, &backdrop_names)
                    .or_else(|| pick_image(imgs, &[&fanart])),
                stream: Some(v.url),
                ..Default::default()
            });
            continue;
        }

        let (ep_s, ep_e, before, after) = match ep {
            Some((s, e, b, a)) => (Some(s), Some(e), b, a),
            None => (None, None, v.stem.as_str(), ""),
        };
        let show_idx = if in_shows && v.dirs.len() >= 2 {
            Some(1)
        } else if let Some(i) = season_dir.filter(|i| *i > 0) {
            Some(i - 1)
        } else if !v.dirs.is_empty() && !in_shows {
            Some(v.dirs.len() - 1)
        } else {
            None
        };
        let (show_name, show_url) = match show_idx {
            Some(i) => (clean_title(&v.dirs[i]).0, Some(dir_url(i + 1))),
            None => (clean_title(before).0, None),
        };
        let show_name = if show_name.is_empty() {
            "Unknown show".to_owned()
        } else {
            show_name
        };
        let season_no = ep_s
            .or_else(|| season_dir.and_then(|i| is_season_dir(&v.dirs[i])))
            .unwrap_or(1);
        let ep_title = clean_title(after).0;

        let key = show_name.to_lowercase();
        let show = shows.entry(key.clone()).or_insert_with(|| {
            let imgs = show_url.as_ref().and_then(|u| images.get(u));
            Item {
                id: format!("{root}#show:{key}"),
                title: show_name.clone(),
                kind: Kind::Series,
                poster: pick_image(imgs, &["poster", "folder", "cover", "show"]),
                backdrop: pick_image(imgs, &["fanart", "backdrop", "background"]),
                ..Default::default()
            }
        });
        if show.poster.is_none() {
            show.poster = pick_image(images.get(&v.dir), &["poster", "folder", "cover"]);
        }
        let season = match show.seasons.iter_mut().position(|s| s.number == season_no) {
            Some(i) => &mut show.seasons[i],
            None => {
                show.seasons.push(Season {
                    number: season_no,
                    ..Default::default()
                });
                show.seasons.last_mut().unwrap()
            }
        };
        let still_name = v.stem.to_lowercase();
        season.episodes.push(Episode {
            id: v.url.clone(),
            number: ep_e.unwrap_or(0),
            title: ep_title,
            still: pick_image(
                images.get(&v.dir),
                &[&still_name, &format!("{still_name}-thumb")],
            ),
            stream: v.url,
            ..Default::default()
        });
    }
    for mut show in shows.into_values() {
        for s in &mut show.seasons {
            s.episodes
                .sort_by(|a, b| a.number.cmp(&b.number).then(a.stream.cmp(&b.stream)));
            if s.episodes.iter().any(|e| e.number == 0) {
                for (i, e) in s.episodes.iter_mut().enumerate() {
                    e.number = i as u32 + 1;
                }
            }
            for e in &mut s.episodes {
                if e.title.is_empty() {
                    e.title = format!("Episode {}", e.number);
                }
            }
        }
        lib.series.push(show);
    }
    lib
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles() {
        assert_eq!(
            clean_title("2.Fast.2.Furious.2003.2160p"),
            ("2 Fast 2 Furious".into(), Some(2003))
        );
        assert_eq!(
            clean_title("1917 (2019) [1080p]"),
            ("1917".into(), Some(2019))
        );
        assert_eq!(
            clean_title("The_Matrix_1080p_BluRay"),
            ("The Matrix".into(), None)
        );
    }

    #[test]
    fn episodes() {
        let (s, e, before, after) = parse_episode("Breaking.Bad.S02E05.Breakage.720p").unwrap();
        assert_eq!((s, e), (2, 5));
        assert_eq!(clean_title(before).0, "Breaking Bad");
        assert_eq!(clean_title(after).0, "Breakage");
        assert_eq!(
            parse_episode("show 3x07 - Name").map(|x| (x.0, x.1)),
            Some((3, 7))
        );
        assert!(parse_episode("2.Fast.2.Furious.2003.2160p").is_none());
        assert!(parse_episode("Inception.2010.1080p.x264").is_none());
    }

    #[test]
    fn urls() {
        assert_eq!(normalize(" 10.0.0.2:8080/ "), "http://10.0.0.2:8080");
        assert_eq!(join("http://h:1/media", "/a/b.jpg"), "http://h:1/a/b.jpg");
        assert_eq!(join("http://h:1", "a/b.jpg"), "http://h:1/a/b.jpg");
        assert_eq!(resolve_href("http://h/Movies/", "/x/"), "http://h/x/");
        assert_eq!(
            resolve_href("http://h/Movies/", "A%20B.mkv?x"),
            "http://h/Movies/A%20B.mkv"
        );
    }

    #[test]
    fn ids_unique_across_kinds() {
        let mut lib: Library = serde_json::from_str(
            r#"{"movies":[{"id":"1","title":"M","stream":"/s/m/1"}],
                "series":[{"id":"1","title":"S","seasons":[{"number":1,"episodes":[{"id":"1","number":1,"title":"E","stream":"/s/e/1"}]}]}]}"#,
        )
        .unwrap();
        lib.finish("http://h");
        assert_eq!(lib.find("movie:1").unwrap().title, "M");
        assert_eq!(lib.find("series:1").unwrap().title, "S");
        assert_eq!(lib.find_episode("episode:1").unwrap().0.title, "S");
        assert_eq!(lib.movies[0].stream.as_deref(), Some("http://h/s/m/1"));
    }

    #[test]
    fn classify_listing() {
        let root = "http://h/";
        let f = |path: &str| {
            let (d, file) = path.rsplit_once('/').unwrap_or(("", path));
            let dirs: Vec<String> = d
                .split('/')
                .filter(|s| !s.is_empty())
                .map(Into::into)
                .collect();
            let dir = if dirs.is_empty() {
                root.to_owned()
            } else {
                format!("{root}{d}/")
            };
            Found {
                url: format!("{root}{path}"),
                dir,
                dirs,
                stem: split_ext(file).0.to_owned(),
            }
        };
        let lib = classify(
            root,
            vec![
                f("Movies/Inception (2010)/Inception.mkv"),
                f("2.Fast.2.Furious.2003.2160p.mkv"),
                f("TV/The Office/Season 2/The.Office.S02E01.The.Dundies.mkv"),
                f("TV/The Office/Season 1/The.Office.S01E01.Pilot.mkv"),
                f("Cartoons/Bluey/Bluey.S01E02.mkv"),
            ],
            &HashMap::from([(
                "http://h/TV/The Office/".to_owned(),
                vec![("poster".into(), "p.jpg".into())],
            )]),
        );
        let mut movies: Vec<_> = lib
            .movies
            .iter()
            .map(|m| (m.title.as_str(), m.year))
            .collect();
        movies.sort();
        assert_eq!(
            movies,
            [("2 Fast 2 Furious", Some(2003)), ("Inception", Some(2010))]
        );
        let office = lib.series.iter().find(|s| s.title == "The Office").unwrap();
        assert_eq!(office.poster.as_deref(), Some("p.jpg"));
        assert_eq!(office.seasons.len(), 2);
        assert!(lib.series.iter().any(|s| s.title == "Bluey"));
    }
}
