//! Discord Rich Presence: "Watching Valo" on the user's Discord profile,
//! with the title and the time left. Talks to the Discord app over its
//! local IPC socket (a named pipe on Windows) from a background thread, so a
//! missing or closed Discord never slows the UI down.
use std::io::{Read, Write};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::util::now_unix;

/// The Discord application the presence belongs to; its name is what
/// Discord shows after "Watching".
const CLIENT_ID: &str = "1276156465446654074";
/// Square artwork for the presence (Discord loads it from here).
const ICON: &str = "https://raw.githubusercontent.com/Okaza03/Valo/main/packaging/windows/valo.png";
/// How often to look for Discord again when it isn't running.
const RETRY: Duration = Duration::from_secs(20);

/// What to show. `None` clears the presence.
#[derive(Clone, PartialEq, Debug)]
pub struct Activity {
    /// First line: "Inception", "The Office".
    pub details: String,
    /// Second line: "S02E03 · Office Olympics", "Paused".
    pub state: String,
    /// Unix seconds the video started and will end (Discord counts down).
    pub times: Option<(i64, i64)>,
}

impl Activity {
    /// Watching something: `pos` and `dur` in seconds.
    pub fn watching(title: &str, subtitle: &str, pos: f64, dur: f64, paused: bool) -> Self {
        let state = if paused {
            "Paused".to_owned()
        } else if subtitle.is_empty() {
            "Watching".to_owned()
        } else {
            subtitle.to_owned()
        };
        let times = (!paused && dur > 0.0).then(|| {
            let start = now_unix() - pos as i64;
            (start, start + dur as i64)
        });
        Self {
            details: title.to_owned(),
            state,
            times,
        }
    }

    pub fn browsing() -> Self {
        Self {
            details: "Browsing the library".to_owned(),
            state: "Picking something to watch".to_owned(),
            times: None,
        }
    }

    /// Worth telling Discord again: something changed, or the times moved
    /// (a seek) by more than a few seconds.
    fn differs(&self, other: &Self) -> bool {
        if self.details != other.details || self.state != other.state {
            return true;
        }
        match (self.times, other.times) {
            (Some(a), Some(b)) => (a.0 - b.0).abs() > 3 || (a.1 - b.1).abs() > 3,
            (a, b) => a.is_some() != b.is_some(),
        }
    }

    fn json(&self) -> Value {
        let version = format!("Valo {}", crate::update::VERSION);
        let mut a = json!({
            "type": 3, // "Watching"
            "details": clip(&self.details),
            // the bottom line ends with the version: "S02E03 · Office Olympics · Valo 0.8.0"
            "state": clip(&format!("{}  ·  {version}", self.state)),
            "assets": { "large_image": ICON, "large_text": version },
        });
        if let Some((start, end)) = self.times {
            a["timestamps"] = json!({ "start": start, "end": end });
        }
        // the member list shows the first line ("Inception", "Browsing the
        // library") instead of the app's name
        a["status_display_type"] = json!(2); // 2: details
        a
    }
}

/// Discord wants 2 to 128 characters per line.
fn clip(s: &str) -> String {
    let mut s: String = s.chars().take(128).collect();
    while s.chars().count() < 2 {
        s.push(' ');
    }
    s
}

/// The presence, as the UI sees it: tell it what to show, every frame.
pub struct Presence {
    tx: Sender<Option<Activity>>,
    last: Option<Activity>,
}

impl Presence {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || run(rx));
        Self { tx, last: None }
    }

    /// Show `a` (or nothing). Cheap: only real changes go to Discord.
    pub fn set(&mut self, a: Option<Activity>) {
        let changed = match (&a, &self.last) {
            (Some(a), Some(b)) => a.differs(b),
            (a, b) => a.is_some() != b.is_some(),
        };
        if changed {
            self.last = a.clone();
            let _ = self.tx.send(a);
        }
    }
}

/// The background thread: keeps a connection while there's something to
/// show, and reconnects when Discord starts later.
fn run(rx: Receiver<Option<Activity>>) {
    let mut conn: Option<Conn> = None;
    let mut want: Option<Activity> = None;
    let mut dirty = false;
    let mut tried = Instant::now() - RETRY;
    loop {
        match rx.recv_timeout(RETRY) {
            Ok(a) => {
                want = a;
                dirty = true;
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return, // the app is closing
        }
        while let Ok(a) = rx.try_recv() {
            want = a; // only the newest matters
            dirty = true;
        }
        if !dirty {
            continue;
        }
        if conn.is_none() {
            if want.is_none() {
                dirty = false; // nothing shown, nothing to clear
                continue;
            }
            if tried.elapsed() < RETRY {
                continue;
            }
            tried = Instant::now();
            conn = Conn::open();
        }
        if let Some(c) = conn.as_mut() {
            match c.set(want.as_ref()) {
                Ok(()) => dirty = false,
                Err(_) => conn = None, // Discord closed; try again later
            }
        }
    }
}

#[cfg(unix)]
type Pipe = std::os::unix::net::UnixStream;
#[cfg(windows)]
type Pipe = std::fs::File;

struct Conn {
    pipe: Pipe,
    nonce: u64,
}

impl Conn {
    /// Find Discord's socket and say hello.
    fn open() -> Option<Self> {
        for path in candidates() {
            let Some(pipe) = connect(&path) else {
                continue;
            };
            let mut c = Conn { pipe, nonce: 0 };
            let hello = json!({ "v": 1, "client_id": CLIENT_ID });
            if c.send(0, &hello).is_ok() && c.recv().is_ok() {
                return Some(c);
            }
        }
        None
    }

    fn set(&mut self, a: Option<&Activity>) -> std::io::Result<()> {
        self.set_reply(a).map(|_| ())
    }

    /// Like [`set`](Self::set), with Discord's answer.
    fn set_reply(&mut self, a: Option<&Activity>) -> std::io::Result<Value> {
        self.nonce += 1;
        let msg = json!({
            "cmd": "SET_ACTIVITY",
            "args": { "pid": std::process::id(), "activity": a.map(Activity::json) },
            "nonce": self.nonce.to_string(),
        });
        self.send(1, &msg)?;
        let reply = self.recv()?;
        if reply["evt"] == "ERROR" {
            let why = reply["data"]["message"]
                .as_str()
                .unwrap_or("rejected")
                .to_owned();
            return Err(std::io::Error::other(why));
        }
        Ok(reply)
    }

    /// One frame: opcode and length (little endian), then JSON.
    fn send(&mut self, op: u32, body: &Value) -> std::io::Result<()> {
        let body = body.to_string();
        let mut frame = Vec::with_capacity(8 + body.len());
        frame.extend_from_slice(&op.to_le_bytes());
        frame.extend_from_slice(&(body.len() as u32).to_le_bytes());
        frame.extend_from_slice(body.as_bytes());
        self.pipe.write_all(&frame)
    }

    fn recv(&mut self) -> std::io::Result<Value> {
        let mut head = [0u8; 8];
        self.pipe.read_exact(&mut head)?;
        let op = u32::from_le_bytes(head[..4].try_into().unwrap());
        let len = u32::from_le_bytes(head[4..].try_into().unwrap()) as usize;
        let mut body = vec![0u8; len.min(1 << 20)];
        self.pipe.read_exact(&mut body)?;
        if op == 2 {
            // Discord closed the connection (bad client id, …)
            return Err(std::io::Error::other("closed by Discord"));
        }
        Ok(serde_json::from_slice(&body).unwrap_or(Value::Null))
    }
}

#[cfg(unix)]
fn candidates() -> Vec<std::path::PathBuf> {
    use std::path::PathBuf;
    let mut dirs: Vec<PathBuf> = ["XDG_RUNTIME_DIR", "TMPDIR", "TMP", "TEMP"]
        .iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .collect();
    dirs.push("/tmp".into());
    // Discord installed as a Flatpak or Snap keeps its socket in a subfolder
    let subs = [
        "",
        "app/com.discordapp.Discord",
        "app/com.discordapp.DiscordCanary",
        "snap.discord",
    ];
    let mut out = Vec::new();
    for d in &dirs {
        for s in subs {
            for i in 0..10 {
                out.push(d.join(s).join(format!("discord-ipc-{i}")));
            }
        }
    }
    out
}

#[cfg(unix)]
fn connect(path: &std::path::Path) -> Option<Pipe> {
    let s = Pipe::connect(path).ok()?;
    s.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
    s.set_write_timeout(Some(Duration::from_secs(5))).ok()?;
    Some(s)
}

#[cfg(windows)]
fn candidates() -> Vec<std::path::PathBuf> {
    (0..10)
        .map(|i| format!(r"\\.\pipe\discord-ipc-{i}").into())
        .collect()
}

#[cfg(windows)]
fn connect(path: &std::path::Path) -> Option<Pipe> {
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_real_changes_count() {
        let a = Activity::watching("Heat", "1995", 100.0, 6000.0, false);
        let mut b = a.clone();
        b.times = b.times.map(|(s, e)| (s + 1, e + 1)); // a second later: same
        assert!(!a.differs(&b));
        b.times = b.times.map(|(s, e)| (s + 60, e + 60)); // a seek
        assert!(a.differs(&b));
        let paused = Activity::watching("Heat", "1995", 100.0, 6000.0, true);
        assert!(a.differs(&paused) && paused.times.is_none() && paused.state == "Paused");
    }

    /// Needs a running Discord: `cargo test -- --ignored discord_live`.
    #[test]
    #[ignore]
    fn discord_live() {
        let mut c = Conn::open().expect("couldn't reach Discord");
        let a = Activity::watching("Inception", "2010", 600.0, 8880.0, false);
        let reply = c.set_reply(Some(&a)).expect("SET_ACTIVITY failed");
        println!("{reply}");
        let secs = std::env::var("SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(4);
        std::thread::sleep(Duration::from_secs(secs));
        c.set(None).expect("clearing failed");
    }

    #[test]
    fn activity_json() {
        let a = Activity::watching("A", "", 0.0, 60.0, false);
        let j = a.json();
        assert_eq!(j["type"], 3);
        assert_eq!(j["details"], "A "); // padded to Discord's minimum
        assert_eq!(
            j["timestamps"]["end"].as_i64().unwrap() - j["timestamps"]["start"].as_i64().unwrap(),
            60
        );
    }
}
