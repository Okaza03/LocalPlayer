//! "Is there a newer LocalPlayer?" — asks GitHub for the latest release.
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use eframe::egui::{self, Align, Align2, Layout, RichText};
use serde::Deserialize;

use crate::ui::{chrome, mono, small_button, BG, LINE, MUTED, TEXT};
use crate::util::Task;

const REPO: &str = "Okaza03/LocalPlayer";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone)]
pub struct Release {
    pub version: String,
    pub url: String,
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    html_url: String,
}

/// "v1.2.3-beta" -> [1, 2, 3]
fn parse(v: &str) -> Vec<u64> {
    let v = v.trim().trim_start_matches(['v', 'V']);
    let core = v.split(['-', '+']).next().unwrap_or(v);
    core.split('.').map(|p| p.parse().unwrap_or(0)).collect()
}

fn newer(latest: &str, current: &str) -> bool {
    let (mut a, mut b) = (parse(latest), parse(current));
    let n = a.len().max(b.len());
    a.resize(n, 0);
    b.resize(n, 0);
    a > b
}

/// The latest release if it's newer than this build, None if we're current.
fn latest() -> Result<Option<Release>> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(10))
        .user_agent(concat!("LocalPlayer/", env!("CARGO_PKG_VERSION")))
        .build();
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let r: GhRelease = match agent
        .get(&url)
        .set("Accept", "application/vnd.github+json")
        .call()
    {
        Ok(r) => r.into_json()?,
        Err(ureq::Error::Status(404, _)) => return Err(anyhow!("no releases published yet")),
        Err(ureq::Error::Status(code, _)) => return Err(anyhow!("GitHub answered {code}")),
        Err(e) => return Err(anyhow!("can't reach GitHub ({e})")),
    };
    let version = r.tag_name.trim_start_matches(['v', 'V']).to_owned();
    Ok(newer(&version, VERSION).then_some(Release {
        version,
        url: r.html_url,
    }))
}

enum Toast {
    UpToDate,
    Failed(String),
    Available,
}

#[derive(Default)]
pub struct Updates {
    task: Option<(bool, Task<Result<Option<Release>>>)>,
    pub available: Option<Release>,
    pub checked_at: Option<Instant>,
    toast: Option<(Toast, Instant)>,
}

impl Updates {
    /// `manual` checks always report back; automatic ones only when there's an update.
    pub fn check(&mut self, ctx: &egui::Context, manual: bool) {
        if self.task.is_none() {
            self.task = Some((manual, Task::spawn(ctx, latest)));
        }
    }

    pub fn checking(&self) -> bool {
        self.task.is_some()
    }

    /// "Checked 3 min ago" for the status bar.
    pub fn checked_label(&self) -> Option<String> {
        let m = self.checked_at?.elapsed().as_secs() / 60;
        Some(match m {
            0 => "Checked for updates just now".into(),
            1 => "Checked for updates 1 min ago".into(),
            m => format!("Checked for updates {m} min ago"),
        })
    }

    /// Poll the check and draw its result (a notice in the bottom-right corner).
    pub fn ui(&mut self, ctx: &egui::Context) {
        if let Some((manual, mut t)) = self.task.take() {
            match t.poll() {
                None => self.task = Some((manual, t)),
                Some(r) => {
                    self.checked_at = Some(Instant::now());
                    let toast = match r {
                        Ok(Some(rel)) => {
                            self.available = Some(rel);
                            Some(Toast::Available)
                        }
                        Ok(None) => {
                            self.available = None;
                            manual.then_some(Toast::UpToDate)
                        }
                        Err(e) => manual.then(|| Toast::Failed(format!("{e:#}"))),
                    };
                    self.toast = toast.map(|t| (t, Instant::now()));
                }
            }
        }

        let Some((toast, since)) = &self.toast else {
            return;
        };
        let keep = matches!(toast, Toast::Available) || since.elapsed() < Duration::from_secs(6);
        if !keep {
            self.toast = None;
            return;
        }
        let mut close = false;
        egui::Area::new(egui::Id::new("update_toast"))
            .anchor(Align2::RIGHT_BOTTOM, [-16.0, -46.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(BG)
                    .stroke(egui::Stroke::new(1.0f32, LINE))
                    .corner_radius(4)
                    .inner_margin(egui::Margin::symmetric(16, 12))
                    .show(ui, |ui| {
                        ui.set_max_width(380.0);
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 2.0;
                                match toast {
                                    Toast::Available => {
                                        let v = self
                                            .available
                                            .as_ref()
                                            .map_or("", |r| r.version.as_str());
                                        ui.label(mono("Update available", 11.0).color(MUTED));
                                        ui.label(
                                            RichText::new(format!(
                                                "LocalPlayer {v} is out (you have {VERSION})."
                                            ))
                                            .color(TEXT),
                                        );
                                    }
                                    Toast::UpToDate => {
                                        ui.label(mono("Up to date", 11.0).color(MUTED));
                                        ui.label(
                                            RichText::new(format!(
                                                "LocalPlayer {VERSION} is the latest version."
                                            ))
                                            .color(TEXT),
                                        );
                                    }
                                    Toast::Failed(e) => {
                                        ui.label(
                                            mono("Couldn't check for updates", 11.0).color(MUTED),
                                        );
                                        ui.label(RichText::new(e).color(TEXT));
                                    }
                                }
                            });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if small_button(ui, "✕").clicked() {
                                    close = true;
                                }
                                if let (Toast::Available, Some(r)) = (toast, &self.available) {
                                    if small_button(ui, "Download").clicked() {
                                        chrome::open_url(&r.url);
                                        close = true;
                                    }
                                }
                            });
                        });
                    });
            });
        if close {
            self.toast = None;
        }
        ctx.request_repaint_after(Duration::from_millis(500));
    }
}

#[cfg(test)]
mod tests {
    use super::newer;

    #[test]
    fn versions() {
        assert!(newer("0.2.0", "0.1.0"));
        assert!(newer("v1.0", "0.9.9"));
        assert!(newer("0.1.10", "0.1.9"));
        assert!(!newer("0.1.0", "0.1.0"));
        assert!(!newer("v0.1.0-beta", "0.1.0"));
        assert!(!newer("0.0.9", "0.1.0"));
    }
}
