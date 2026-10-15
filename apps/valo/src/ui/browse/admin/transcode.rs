//! Admin › Transcode: files that don't play smoothly everywhere (MKV,
//! HEVC, DTS), converted to MP4 by the server one at a time, with its log.
use std::time::{Duration, Instant};

use anyhow::Result;
use eframe::egui::{self, RichText, ScrollArea, Ui};
use serde_json::json;

use super::super::details::{bytes, caps, padded, panel, TEAL};
use super::super::people::columns_with;
use super::{not_set_up, status_dot, Cx, Done, Remote, Section};
use crate::api::{LogLine, Transcode};
use crate::ui::{
    mono, primary_button, segmented, small_button, ERROR, LINE, MUTED, OK, TEXT, WARN,
};
use crate::util::Task;

#[derive(Clone, Copy, PartialEq, Default)]
enum Show {
    #[default]
    Waiting,
    Done,
    All,
}

#[derive(Default)]
pub(in super::super) struct State {
    list: Remote<Transcode>,
    show: Show,
    log: Vec<LogLine>,
    log_after: u64,
    log_task: Option<Task<Result<Vec<LogLine>>>>,
    log_at: Option<Instant>,
    log_missing: bool,
}

impl State {
    pub fn refresh(&mut self) {
        self.list.retry();
        self.log_missing = false;
    }

    pub fn missing(&self) -> bool {
        self.list.missing
    }

    pub fn ui(&mut self, ui: &mut Ui, cx: &mut Cx) {
        self.list.poll(&mut cx.login);
        let running = self.list.data.as_ref().is_some_and(|t| t.running);
        // every few seconds while something converts, else now and then
        let every = if running { 3 } else { 30 };
        if self.list.idle() || (self.list.data.is_some() && self.list.stale(every)) {
            let c = cx.client.clone();
            self.list.load(&cx.ctx, move || c.admin_get("transcode"));
        }
        if self.list.missing {
            padded(ui, |ui| {
                if not_set_up(ui, Section::Transcode, &cx.client.info.name) {
                    self.list.retry();
                }
            });
            return;
        }
        self.poll_log(cx);
        cx.ctx.request_repaint_after(Duration::from_secs(every));

        let files = self
            .list
            .data
            .as_ref()
            .map(|t| t.files.clone())
            .unwrap_or_default();
        let waiting = |s: &str| !matches!(s, "done");
        let count = |f: &dyn Fn(&str) -> bool| Some(files.iter().filter(|x| f(&x.status)).count());
        let mut start: Option<Vec<String>> = None;
        let mut cancel = false;
        let mut scan = false;
        padded(ui, |ui| {
            ui.horizontal(|ui| {
                segmented(
                    ui,
                    &mut self.show,
                    &[
                        (Show::Waiting, "To convert", count(&waiting)),
                        (Show::Done, "Done", count(&|s| s == "done")),
                        (Show::All, "All", Some(files.len())),
                    ],
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let idle: Vec<String> = files
                        .iter()
                        .filter(|f| matches!(f.status.as_str(), "idle" | "failed" | ""))
                        .map(|f| f.key.clone())
                        .collect();
                    if ui
                        .add_enabled_ui(!cx.busy && !idle.is_empty(), |ui| {
                            primary_button(ui, &format!("Convert all ({})", idle.len()), true)
                        })
                        .inner
                        .clicked()
                    {
                        start = Some(Vec::new()); // empty: all
                    }
                    if files.iter().any(|f| f.status == "queued")
                        && small_button(ui, "Cancel queue").clicked()
                    {
                        cancel = true;
                    }
                    if small_button(ui, "Scan again").clicked() {
                        scan = true;
                    }
                    if self.list.loading() {
                        ui.add(egui::Spinner::new().size(16.0).color(MUTED));
                    }
                });
            });
            ui.add_space(14.0);
            if let Some(e) = &self.list.error {
                ui.label(RichText::new(e).color(ERROR));
            }
        });

        let shown: Vec<_> = files
            .iter()
            .filter(|f| match self.show {
                Show::Waiting => waiting(&f.status),
                Show::Done => f.status == "done",
                Show::All => true,
            })
            .cloned()
            .collect();
        columns_with(
            ui,
            &mut (self, &mut start),
            |ui, (_, start)| {
                if shown.is_empty() {
                    ui.label(mono("Nothing here", 12.0).color(MUTED));
                }
                for f in &shown {
                    egui::Frame::new()
                        .stroke(egui::Stroke::new(1.0f32, LINE))
                        .corner_radius(8)
                        .inner_margin(egui::Margin::symmetric(14, 10))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                let col = match f.status.as_str() {
                                    "done" => OK,
                                    "running" => TEAL,
                                    "queued" => WARN,
                                    "failed" => ERROR,
                                    _ => MUTED,
                                };
                                status_dot(ui, col);
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 2.0;
                                    ui.label(RichText::new(&f.title).color(TEXT));
                                    let mut bits: Vec<String> = Vec::new();
                                    for b in [&f.container, &f.video_codec, &f.audio_codec]
                                        .into_iter()
                                        .flatten()
                                    {
                                        bits.push(b.clone());
                                    }
                                    if let Some(s) = f.size {
                                        bits.push(bytes(s));
                                    }
                                    let state = match (f.status.as_str(), f.progress) {
                                        ("running", Some(p)) => {
                                            format!("converting {:.0}%", p * 100.0)
                                        }
                                        ("", _) => "idle".into(),
                                        (s, _) => s.to_owned(),
                                    };
                                    bits.push(state);
                                    ui.label(mono(bits.join("   ·   "), 10.0).color(MUTED));
                                    if let Some(e) = &f.error {
                                        ui.label(RichText::new(e).small().color(ERROR));
                                    }
                                });
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if matches!(f.status.as_str(), "idle" | "failed" | "")
                                            && small_button(ui, "Convert").clicked()
                                        {
                                            **start = Some(vec![f.key.clone()]);
                                        }
                                    },
                                );
                            });
                            if let (Some(p), "running") = (f.progress, f.status.as_str()) {
                                ui.add(egui::ProgressBar::new(p).desired_height(4.0));
                            }
                        });
                    ui.add_space(6.0);
                }
            },
            |ui, (st, _)| {
                panel(ui, |ui| {
                    ui.horizontal(|ui| {
                        caps(ui, "Transcode log", TEAL);
                        if running {
                            status_dot(ui, OK);
                        }
                    });
                    ui.add_space(8.0);
                    if st.log_missing {
                        ui.label(
                            RichText::new("This server doesn't share a transcode log.")
                                .small()
                                .color(MUTED),
                        );
                        return;
                    }
                    ScrollArea::vertical()
                        .id_salt("transcode-log")
                        .max_height(360.0)
                        .stick_to_bottom(true)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            if st.log.is_empty() {
                                ui.label(mono("Nothing yet", 10.5).color(MUTED));
                            }
                            for l in &st.log {
                                let col = match l.level.as_str() {
                                    "error" => ERROR,
                                    "warning" | "warn" => WARN,
                                    _ => TEXT,
                                };
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&l.message).monospace().size(10.5).color(col),
                                    )
                                    .wrap(),
                                );
                            }
                        });
                });
            },
        );

        if let Some(keys) = start {
            let all = keys.is_empty();
            cx.run(move |c| {
                let body = if all {
                    json!({ "all": true })
                } else {
                    json!({ "keys": keys })
                };
                c.admin_call("POST", "transcode", Some(&body))?;
                Ok(Done::msg(if all {
                    "Started converting every file."
                } else {
                    "Started converting."
                }))
            });
        } else if cancel {
            cx.run(|c| {
                c.admin_call("DELETE", "transcode/queue", None)?;
                Ok(Done::msg(
                    "Cancelled the queue. A file that's converting now finishes.",
                ))
            });
        } else if scan {
            cx.run(|c| {
                c.admin_call("POST", "transcode/scan", None)?;
                Ok(Done::msg("Scanning the library for files to convert."))
            });
        }
    }

    fn poll_log(&mut self, cx: &mut Cx) {
        if let Some(r) = self.log_task.as_mut().and_then(|t| t.poll()) {
            self.log_task = None;
            self.log_at = Some(Instant::now());
            match r {
                Ok(lines) => {
                    if let Some(l) = lines.last() {
                        self.log_after = l.id;
                    }
                    self.log.extend(lines);
                    let n = self.log.len();
                    if n > 500 {
                        self.log.drain(..n - 500);
                    }
                }
                Err(e) if e.is::<crate::api::NoAdminApi>() => self.log_missing = true,
                Err(_) => {}
            }
        }
        let due = self
            .log_at
            .is_none_or(|t| t.elapsed() >= Duration::from_secs(3));
        if !self.log_missing && self.log_task.is_none() && due {
            let (c, after) = (cx.client.clone(), self.log_after);
            self.log_task = Some(Task::spawn(&cx.ctx, move || {
                c.admin_get(&format!("transcode/log?after={after}"))
            }));
        }
    }
}
