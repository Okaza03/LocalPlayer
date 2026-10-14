//! Keep watch progress in step with the server (`/api/progress`), so the
//! website and every device carry on where any of them left off. The newer
//! change wins; changes made offline are sent once the server is back.
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use eframe::egui;

use crate::api::{Client, Watched};
use crate::config::{Entry, Progress};
use crate::util::Task;

/// Look for changes made elsewhere this often (not during playback).
const PULL_EVERY: Duration = Duration::from_secs(60);
/// Send our changes at most this often; playback saves every few seconds.
const PUSH_EVERY: Duration = Duration::from_secs(15);

/// Each sent entry with the server's answer: what it stored, `None` if it
/// turned the item down, `Err` to try again later.
type Pushed = Vec<(String, Entry, Result<Option<Watched>>)>;
/// What the server has; `None` if it doesn't keep progress.
type Pulled = Result<Option<Vec<Watched>>>;

#[derive(Default)]
pub struct Sync {
    /// Server and token the state below belongs to.
    session: Option<(String, Option<String>)>,
    /// The server turned out not to keep progress.
    off: bool,
    pull: Option<(String, Task<Pulled>)>,
    push: Option<(String, Task<Pushed>)>,
    last_pull: Option<Instant>,
    last_push: Option<Instant>,
}

fn prefix(c: &Client, user: Option<&str>) -> String {
    crate::config::user_prefix(&c.base, user)
}

/// The server's entry in the app's terms. A title the server calls
/// completed counts as watched here too, whatever the position.
fn entry(w: &Watched) -> Option<(String, Entry)> {
    let key = w.key()?;
    let mut e = Entry {
        pos: w.position.max(0.0),
        dur: w.duration.max(0.0),
        updated: w.updated,
    };
    if w.completed {
        if e.dur <= 0.0 {
            e.dur = e.pos.max(60.0);
        }
        e.pos = e.dur;
    }
    Some((key, e))
}

fn send(client: &Client, items: Vec<(String, Entry)>) -> Pushed {
    items
        .into_iter()
        .map(|(id, e)| {
            let res = match Watched::from_key(&id, e.pos, e.dur) {
                Some(w) => client.save_progress(&w),
                None => Ok(None), // not a title the server knows (a local file, a guessed id)
            };
            (id, e, res)
        })
        .collect()
}

fn apply_pushed(progress: &mut Progress, prefix: &str, pushed: Pushed) {
    for (id, e, res) in pushed {
        match res {
            Ok(saved) => {
                let t = saved.map(|w| w.updated).filter(|&t| t > 0);
                progress.sent(prefix, &id, &e, t);
            }
            Err(err) => eprintln!("progress sync: {err:#}"),
        }
    }
    progress.flush();
}

impl Sync {
    /// Look for changes elsewhere on the next tick.
    pub fn pull_now(&mut self) {
        self.last_pull = None;
    }

    /// Send our changes on the next tick.
    pub fn push_now(&mut self) {
        self.last_push = None;
    }

    /// Call every frame while connected to `client` as `user`. `watching`: a
    /// video is open, so leave its resume point alone.
    pub fn tick(
        &mut self,
        ctx: &egui::Context,
        client: &Client,
        user: Option<&str>,
        progress: &mut Progress,
        watching: bool,
    ) {
        let session = Some((client.base.clone(), client.token.clone()));
        if self.session != session {
            *self = Self {
                session,
                ..Self::default()
            };
        }
        if let Some(res) = self.pull.as_mut().and_then(|(_, t)| t.poll()) {
            let (prefix, _) = self.pull.take().unwrap();
            match res {
                Ok(Some(list)) => {
                    if progress.merge(&prefix, list.iter().filter_map(entry).collect()) {
                        progress.flush();
                    }
                }
                Ok(None) => self.off = true,
                Err(e) => eprintln!("progress sync: {e:#}"),
            }
        }
        if let Some(pushed) = self.push.as_mut().and_then(|(_, t)| t.poll()) {
            let (prefix, _) = self.push.take().unwrap();
            apply_pushed(progress, &prefix, pushed);
        }
        if self.off {
            return;
        }
        let due = |t: Option<Instant>, every| t.is_none_or(|t| t.elapsed() >= every);
        if self.pull.is_none() && !watching && due(self.last_pull, PULL_EVERY) {
            self.last_pull = Some(Instant::now());
            let c = client.clone();
            self.pull = Some((prefix(client, user), Task::spawn(ctx, move || c.progress())));
        }
        let unsent = progress.unsent(&prefix(client, user));
        if self.push.is_none() && !unsent.is_empty() && due(self.last_push, PUSH_EVERY) {
            self.last_push = Some(Instant::now());
            let c = client.clone();
            self.push = Some((
                prefix(client, user),
                Task::spawn(ctx, move || send(&c, unsent)),
            ));
        }
        ctx.request_repaint_after(PUSH_EVERY);
    }

    /// On quit: send what's left, waiting a few seconds at most.
    pub fn finish(&mut self, client: &Client, user: Option<&str>, progress: &mut Progress) {
        let prefix = prefix(client, user);
        let unsent = progress.unsent(&prefix);
        if self.off || unsent.is_empty() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        let c = client.clone();
        thread::spawn(move || {
            let _ = tx.send(send(&c, unsent));
        });
        if let Ok(pushed) = rx.recv_timeout(Duration::from_secs(3)) {
            apply_pushed(progress, &prefix, pushed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(json: &str) -> Watched {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn ids_round_trip() {
        let got = w(
            r#"{"type":"episode","id":42,"position":812.5,"duration":2640,"percent":31,"completed":false,"updated":1760400000}"#,
        );
        assert_eq!(got.key().as_deref(), Some("episode:42"));
        let out = Watched::from_key("episode:42", 1.0, 2.0).unwrap();
        assert_eq!(out.id, serde_json::json!(42));
        let out = Watched::from_key("movie:m:Inception (2010)/x.mkv", 1.0, 2.0).unwrap();
        assert_eq!(out.id, serde_json::json!("m:Inception (2010)/x.mkv"));
        assert!(Watched::from_key("series:7", 1.0, 2.0).is_none());
    }

    #[test]
    fn nulls_count_as_missing() {
        let list: Vec<Watched> = serde_json::from_str(
            r#"[{"type":"movie","id":"370","position":8,"duration":null,"percent":null,"completed":false,"updated":1787403483},
                {"type":"movie","id":"455","position":2720.8,"duration":7225.7,"percent":38,"completed":null,"updated":1791944033}]"#,
        )
        .unwrap();
        assert_eq!(list[0].duration, 0.0);
        assert!(!list[1].completed);
    }

    #[test]
    fn completed_counts_as_watched() {
        let (_, e) = entry(&w(r#"{"type":"movie","id":"1","position":2400,"duration":2640,"completed":true,"updated":5}"#)).unwrap();
        assert!(e.watched());
    }

    #[test]
    fn newer_change_wins() {
        let mut p = Progress::default();
        let e = |pos, updated| Entry {
            pos,
            dur: 100.0,
            updated,
        };
        p.merge("s|", vec![("movie:1".into(), e(50.0, 10))]);
        assert!(!p.merge("s|", vec![("movie:1".into(), e(20.0, 9))]));
        assert_eq!(p.get("s|movie:1").unwrap().pos, 50.0);
        // a local change is sent, then takes the server's time
        p.set("s|movie:1", 70.0, 100.0);
        let unsent = p.unsent("s|");
        assert_eq!(unsent.len(), 1);
        p.sent("s|", "movie:1", &unsent[0].1, Some(i64::MAX - 1));
        assert!(p.unsent("s|").is_empty());
        assert!(!p.merge("s|", vec![("movie:1".into(), e(70.0, i64::MAX - 1))]));
        // clearing is a change like any other
        p.clear("s|");
        assert!(p.get("s|movie:1").is_none());
        assert_eq!(p.unsent("s|").len(), 1);
    }
}
