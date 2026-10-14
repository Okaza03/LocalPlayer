//! Settings and watch progress, stored as JSON in ~/.config/valo
//! (%APPDATA%\valo on Windows).
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::util::now_unix;

#[cfg(windows)]
fn config_home() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(PathBuf::from)
}

#[cfg(not(windows))]
fn config_home() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
}

fn dir() -> Option<PathBuf> {
    Some(config_home()?.join("valo"))
}

fn load<T: DeserializeOwned + Default>(name: &str) -> T {
    dir()
        .and_then(|d| fs::read_to_string(d.join(name)).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save<T: Serialize>(name: &str, v: &T) {
    let Some(d) = dir() else { return };
    let write = || -> std::io::Result<()> {
        fs::create_dir_all(&d)?;
        let tmp = d.join(format!("{name}.tmp"));
        fs::write(&tmp, serde_json::to_vec_pretty(v)?)?;
        fs::rename(tmp, d.join(name))
    };
    if let Err(e) = write() {
        eprintln!("can't save {name}: {e}");
    }
}

/// Where one user's progress and list live in progress.json and
/// watchlist.json: keys are this plus the item id. Each signed-in user of a
/// server gets their own; `None` is a server without sign-in.
pub fn user_prefix(base: &str, user: Option<&str>) -> String {
    format!("{base}|@{}|", user.unwrap_or("").to_lowercase())
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Server address as typed by the user, e.g. "192.168.1.20:8080".
    pub server: String,
    pub username: String,
    /// Session token, kept only when "remember me" is ticked.
    pub token: Option<String>,
    /// Whether the saved session's user is an admin (cleared with the token).
    pub admin: bool,
    pub volume: f32,
    /// Play the next episode when one ends.
    pub autoplay_next: bool,
    /// Continue from the saved position instead of the start.
    pub resume: bool,
    pub fullscreen_on_play: bool,
    /// Arrow-key skip, seconds (Shift = ×6).
    pub skip_seconds: u32,
    /// Re-fetch the library this often; 0 = never.
    pub refresh_minutes: u32,
    pub genre_rows: bool,
    /// Interface zoom.
    pub ui_scale: f32,
    /// Keep an app-menu entry for the AppImage (see integrate.rs).
    pub app_menu: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: String::new(),
            username: String::new(),
            token: None,
            admin: false,
            volume: 0.7,
            autoplay_next: true,
            resume: true,
            fullscreen_on_play: false,
            skip_seconds: 5,
            refresh_minutes: 0,
            genre_rows: true,
            ui_scale: 1.0,
            app_menu: true,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        load("config.json")
    }
    pub fn save(&self) {
        save("config.json", self);
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Entry {
    pub pos: f64,
    pub dur: f64,
    pub updated: i64,
}

impl Entry {
    pub fn fraction(&self) -> f32 {
        if self.dur > 0.0 {
            (self.pos / self.dur).clamp(0.0, 1.0) as f32
        } else {
            0.0
        }
    }
    /// Started, but not (nearly) finished.
    pub fn in_progress(&self) -> bool {
        self.pos > 30.0 && (self.dur <= 0.0 || self.pos < self.dur - 60.0f64.min(self.dur * 0.05))
    }
    pub fn watched(&self) -> bool {
        self.dur > 0.0 && !self.in_progress() && self.pos > 30.0
    }
}

/// Resume points keyed by "<server>|<stream id>". An entry at position 0 is a
/// cleared one, kept so the clearing reaches the server (see sync.rs).
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Progress {
    entries: HashMap<String, Entry>,
    /// Keys changed here that the server hasn't been told about yet.
    unsent: HashSet<String>,
    #[serde(skip)]
    dirty: bool,
}

impl Progress {
    pub fn load() -> Self {
        load("progress.json")
    }

    pub fn get(&self, key: &str) -> Option<&Entry> {
        self.entries.get(key).filter(|e| e.pos > 0.0)
    }

    pub fn set(&mut self, key: &str, pos: f64, dur: f64) {
        self.entries.insert(
            key.to_owned(),
            Entry {
                pos,
                dur,
                updated: now_unix(),
            },
        );
        self.unsent.insert(key.to_owned());
        self.dirty = true;
    }

    /// Entries for one server, newest first.
    pub fn recent(&self, prefix: &str) -> Vec<(&str, &Entry)> {
        let mut v: Vec<_> = self
            .entries
            .iter()
            .filter(|(_, e)| e.pos > 0.0)
            .filter_map(|(k, e)| k.strip_prefix(prefix).map(|id| (id, e)))
            .collect();
        v.sort_by_key(|(_, e)| std::cmp::Reverse(e.updated));
        v
    }

    /// Take the server's copy of each entry it has a newer version of.
    /// Returns whether anything changed.
    pub fn merge(&mut self, prefix: &str, remote: Vec<(String, Entry)>) -> bool {
        let mut changed = false;
        for (id, e) in remote {
            let key = format!("{prefix}{id}");
            if self
                .entries
                .get(&key)
                .is_some_and(|l| l.updated >= e.updated)
            {
                continue;
            }
            self.unsent.remove(&key);
            self.entries.insert(key, e);
            changed = true;
        }
        self.dirty |= changed;
        changed
    }

    /// Changes for one server that still have to be sent.
    pub fn unsent(&self, prefix: &str) -> Vec<(String, Entry)> {
        self.unsent
            .iter()
            .filter_map(|k| {
                Some((
                    k.strip_prefix(prefix)?.to_owned(),
                    self.entries.get(k)?.clone(),
                ))
            })
            .collect()
    }

    /// The server has `id` as it was in `sent`. Unless it changed here in the
    /// meantime, stop sending it and take the server's timestamp, so the next
    /// pull doesn't mistake the server's copy for a newer one.
    pub fn sent(&mut self, prefix: &str, id: &str, sent: &Entry, server_updated: Option<i64>) {
        let key = format!("{prefix}{id}");
        if let Some(l) = self
            .entries
            .get_mut(&key)
            .filter(|l| l.updated == sent.updated)
        {
            if let Some(t) = server_updated {
                l.updated = t;
            }
            self.unsent.remove(&key);
            self.dirty = true;
        }
    }

    pub fn flush(&mut self) {
        if self.dirty {
            save("progress.json", self);
            self.dirty = false;
        }
    }
}

/// Titles the user bookmarked ("My list"), newest first, keyed like [`Progress`].
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Watchlist {
    items: Vec<String>,
}

impl Watchlist {
    pub fn load() -> Self {
        load("watchlist.json")
    }

    pub fn contains(&self, key: &str) -> bool {
        self.items.iter().any(|k| k == key)
    }

    pub fn toggle(&mut self, key: &str) {
        match self.items.iter().position(|k| k == key) {
            Some(i) => {
                self.items.remove(i);
            }
            None => self.items.insert(0, key.to_owned()),
        }
        save("watchlist.json", self);
    }

    /// Item ids saved for the server whose keys start with `prefix`.
    pub fn ids<'a>(&'a self, prefix: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.items
            .iter()
            .filter_map(move |k| k.strip_prefix(prefix))
    }
}

/// Titles removed from "Continue watching", keyed like [`Progress`], with when.
/// A title comes back once it's watched again after that.
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Hidden {
    items: HashMap<String, i64>,
}

impl Hidden {
    pub fn load() -> Self {
        load("hidden.json")
    }

    pub fn hide(&mut self, key: &str) {
        self.items.insert(key.to_owned(), now_unix());
        save("hidden.json", self);
    }

    /// Whether `key`, last watched at `updated`, was removed since.
    pub fn hides(&self, key: &str, updated: i64) -> bool {
        self.items.get(key).is_some_and(|t| updated <= *t)
    }
}
