//! The profile page's "Your devices": where this user is signed in, and
//! signing any of them out (`/api/sessions`, see docs/api/auth.md).
use anyhow::Result;
use eframe::egui::{self, vec2, Align, Layout, RichText, Sense, Stroke, Ui};

use super::admin::Remote;
use super::details::{padded, TEAL};
use super::people::{ago, section_title};
use super::Browse;
use crate::api::{self, DeviceSession};
use crate::ui::chrome;
use crate::ui::{mono, small_button, Action, ERROR, LINE, MUTED, OK, TEXT};
use crate::util::Task;

#[derive(Default)]
pub(super) struct State {
    /// `None` inside: the server doesn't list sessions.
    list: Remote<Option<Vec<DeviceSession>>>,
    op: Option<Task<Result<String>>>,
    notice: Option<(String, bool)>,
}

impl Browse {
    pub(super) fn devices_section(&mut self, ui: &mut Ui, action: &mut Action) {
        if self.client.token.is_none() {
            return; // no sign-in, no sessions
        }
        let ctx = ui.ctx().clone();
        let st = &mut self.devices;
        let mut login = false;
        st.list.poll(&mut login);
        if let Some(r) = st.op.as_mut().and_then(|t| t.poll()) {
            st.op = None;
            match r {
                Ok(msg) => st.notice = Some((msg, true)),
                Err(e) if e.is::<api::Unauthorized>() => login = true,
                Err(e) => st.notice = Some((format!("{e:#}"), false)),
            }
            st.list.retry();
        }
        if st.list.idle() || (st.list.data.is_some() && st.list.stale(60)) {
            let c = self.client.clone();
            st.list.load(&ctx, move || c.sessions());
        }
        if login {
            *action = Action::NeedLogin(self.client.clone());
            return;
        }

        let busy = st.op.is_some();
        let mut end: Option<DeviceSession> = None;
        let mut end_others = false;
        ui.add_space(30.0);
        padded(ui, |ui| {
            let list = st.list.data.clone().flatten();
            section_title(
                ui,
                "Security",
                "Your devices",
                list.as_ref().map_or(0, Vec::len),
            );
            if let Some((msg, ok)) = &st.notice {
                ui.label(RichText::new(msg).color(if *ok { OK } else { ERROR }));
                ui.add_space(8.0);
            }
            if let Some(e) = &st.list.error {
                ui.label(RichText::new(format!("Couldn't load your devices: {e}")).color(ERROR));
            }
            match (&st.list.data, list) {
                (Some(None), _) => {
                    ui.label(
                        RichText::new(format!(
                            "{} doesn't list where you're signed in. Its admin can add it; \
                             until then you can only sign out here.",
                            self.client.info.name
                        ))
                        .color(MUTED),
                    );
                    ui.add_space(6.0);
                    if ui.link("Setup guide for server admins ↗").clicked() {
                        chrome::open_url(api::SESSIONS_DOCS);
                    }
                }
                (_, Some(mut list)) => {
                    // this device first, then the most recently used
                    list.sort_by_key(|s| {
                        (!s.current, std::cmp::Reverse(s.last_used.or(s.created)))
                    });
                    for s in &list {
                        if device_row(ui, s, busy) {
                            end = Some(s.clone());
                        }
                        ui.add_space(8.0);
                    }
                    if list.iter().filter(|s| !s.current).count() > 1 {
                        ui.add_space(4.0);
                        if ui
                            .add_enabled_ui(!busy, |ui| {
                                small_button(ui, "Sign out all other devices")
                            })
                            .inner
                            .clicked()
                        {
                            end_others = true;
                        }
                    }
                }
                _ => {
                    if st.list.loading() {
                        ui.add(egui::Spinner::new().size(18.0).color(MUTED));
                    }
                }
            }
        });

        if let Some(s) = end {
            if s.current {
                *action = Action::SignOut(self.client.clone());
                return;
            }
            let c = self.client.clone();
            let name = s.device.clone().unwrap_or_else(|| "that device".into());
            st.notice = None;
            st.op = Some(Task::spawn(&ctx, move || {
                c.end_session(&s.id)?;
                Ok(format!("Signed out {name}."))
            }));
        } else if end_others {
            let others: Vec<String> = st
                .list
                .data
                .clone()
                .flatten()
                .unwrap_or_default()
                .into_iter()
                .filter(|s| !s.current)
                .map(|s| s.id)
                .collect();
            let c = self.client.clone();
            st.notice = None;
            st.op = Some(Task::spawn(&ctx, move || {
                for id in &others {
                    c.end_session(id)?;
                }
                Ok(format!("Signed out {} other devices.", others.len()))
            }));
        }
    }
}

/// One session; true when "Sign out" is clicked.
fn device_row(ui: &mut Ui, s: &DeviceSession, busy: bool) -> bool {
    let mut out = false;
    egui::Frame::new()
        .stroke(Stroke::new(1.0f32, LINE))
        .corner_radius(8)
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::hover());
                screen(ui.painter(), r, if s.current { TEAL } else { MUTED });
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.horizontal(|ui| {
                        let name = s.device.clone().unwrap_or_else(|| "Unknown device".into());
                        ui.label(RichText::new(name).size(15.0).color(TEXT));
                        if s.current {
                            ui.label(mono("This device", 10.0).color(TEAL));
                        }
                    });
                    let mut parts = Vec::new();
                    if let Some(t) = s.last_used {
                        parts.push(format!("active {}", ago(t)));
                    }
                    if let Some(t) = s.created {
                        parts.push(format!("signed in {}", ago(t)));
                    }
                    if let Some(ip) = &s.ip {
                        parts.push(ip.clone());
                    }
                    ui.label(mono(parts.join("   ·   "), 10.0).color(MUTED));
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let label = if s.current {
                        "Sign out"
                    } else {
                        "Sign out device"
                    };
                    if ui
                        .add_enabled_ui(!busy, |ui| small_button(ui, label))
                        .inner
                        .clicked()
                    {
                        out = true;
                    }
                });
            });
        });
    out
}

/// A small monitor outline.
fn screen(p: &egui::Painter, r: egui::Rect, col: egui::Color32) {
    let stroke = Stroke::new(1.5f32, col);
    let body = egui::Rect::from_center_size(r.center() - vec2(0.0, 3.0), vec2(24.0, 15.0));
    p.rect_stroke(body, 2.0, stroke, egui::StrokeKind::Inside);
    p.hline(
        r.center().x - 6.0..=r.center().x + 6.0,
        body.bottom() + 5.0,
        stroke,
    );
    p.vline(r.center().x, body.bottom()..=body.bottom() + 5.0, stroke);
}
