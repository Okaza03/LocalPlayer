//! Settings and watch progress, stored as JSON in ~/.config/localplayer
//! (%APPDATA%\localplayer on Windows).
use std::collections::HashMap;
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
    Some(config_home()?.join("localplayer"))
}

fn load<T: DeserializeOwned + Default>(name: &str) -> T {
    // fall back to the settings of the app's previous names
    let old = ["homieplayer", "lumen-player"].map(|n| config_home().map(|h| h.join(n).join(name)));
    dir()
        .map(|d| d.join(name))
        .into_iter()
        .chain(old.into_iter().flatten())
        .find_map(|p| fs::read_to_string(p).ok())
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

/// Resume points keyed by "<server>|<stream id>".
#[derive(Default, Serialize, Deserialize)]
pub struct Progress {
    entries: HashMap<String, Entry>,
    #[serde(skip)]
    dirty: bool,
}

impl Progress {
    pub fn load() -> Self {
        load("progress.json")
    }

    pub fn get(&self, key: &str) -> Option<&Entry> {
        self.entries.get(key)
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
        self.dirty = true;
    }

    /// Entries for one server, newest first.
    pub fn recent(&self, prefix: &str) -> Vec<(&str, &Entry)> {
        let mut v: Vec<_> = self
            .entries
            .iter()
            .filter_map(|(k, e)| k.strip_prefix(prefix).map(|id| (id, e)))
            .collect();
        v.sort_by_key(|(_, e)| std::cmp::Reverse(e.updated));
        v
    }

    /// Forget the watch history for one server.
    pub fn clear(&mut self, prefix: &str) {
        self.entries.retain(|k, _| !k.starts_with(prefix));
        self.dirty = true;
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
