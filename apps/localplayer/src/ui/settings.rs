//! The Settings and Profile dialogs.
use eframe::egui::{self, vec2, Align, Color32, Layout, RichText, Sense, Stroke, Ui};

use super::chrome::avatar;
use super::{
    bold, mono, primary_button, section, segmented, small_button, ACCENT, BG, LINE, MUTED, TEXT,
};
use crate::config::{Config, Progress};
use crate::util::fmt_runtime;

/// The connection the dialogs describe.
pub struct Conn {
    pub name: String,
    pub base: String,
    pub user: Option<String>,
    pub signed_in: bool,
    pub admin: bool,
}

impl Conn {
    pub fn prefix(&self) -> String {
        format!("{}|", self.base)
    }
}

pub enum Pick {
    Keep,
    Close,
    ChangeServer,
    Refresh,
    OpenFiles,
    SignOut,
    Quit,
    OpenSettings,
}

#[derive(Default)]
pub struct SettingsState {
    confirm_clear: bool,
}

fn dialog<R>(
    ctx: &egui::Context,
    id: &str,
    width: f32,
    add: impl FnOnce(&mut Ui) -> R,
) -> (R, bool) {
    let frame = egui::Frame::new()
        .fill(BG)
        .stroke(Stroke::new(1.0f32, LINE))
        .corner_radius(4)
        // small right margin: the scrollbar lives there, in its own lane next to the content
        .inner_margin(egui::Margin {
            left: 26,
            right: 4,
            top: 22,
            bottom: 22,
        });
    let r = egui::Modal::new(egui::Id::new(id))
        .frame(frame)
        .backdrop_color(Color32::from_black_alpha(160))
        .show(ctx, |ui| {
            ui.set_width(width.min(ctx.screen_rect().width() - 80.0) + 22.0);
            // nothing here takes typing; stray keyboard focus would scroll the dialog to it
            if let Some(f) = ctx.memory(|m| m.focused()) {
                ctx.memory_mut(|m| m.surrender_focus(f));
            }
            let h = ctx.screen_rect().height() - 120.0;
            egui::ScrollArea::vertical()
                .max_height(h)
                .min_scrolled_height(h)
                .show(ui, |ui| {
                    egui::Frame::new()
                        .inner_margin(egui::Margin {
                            left: 0,
                            right: 22,
                            top: 0,
                            bottom: 0,
                        })
                        .show(ui, add)
                        .inner
                })
                .inner
        });
    let close = r.should_close();
    (r.inner, close)
}

fn header(ui: &mut Ui, title: &str, sub: &str) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(36.0, 36.0), Sense::hover());
        let p = ui.painter();
        p.rect_filled(r, 7.0, ACCENT);
        let c = r.center() + vec2(1.5, 0.0);
        p.add(egui::Shape::convex_polygon(
            vec![
                c + vec2(-6.0, -8.0),
                c + vec2(8.0, 0.0),
                c + vec2(-6.0, 8.0),
            ],
            Color32::WHITE,
            Stroke::NONE,
        ));
        ui.add_space(4.0);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(bold(title, 24.0));
            ui.label(mono(sub, 11.0).color(MUTED));
        });
    });
    ui.add_space(8.0);
}

/// Label on the left, control on the right.
fn row(ui: &mut Ui, label: &str, control: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.set_min_height(38.0);
        ui.label(RichText::new(label).size(15.0));
        ui.with_layout(Layout::right_to_left(Align::Center), control);
    });
}

fn combo<T: PartialEq + Copy>(ui: &mut Ui, id: &str, value: &mut T, options: &[(T, &str)]) {
    let text = options.iter().find(|o| o.0 == *value).map_or("", |o| o.1);
    egui::ComboBox::from_id_salt(id)
        .width(200.0)
        .height(300.0)
        .selected_text(RichText::new(text).size(14.5))
        .show_ui(ui, |ui| {
            for (v, label) in options {
                ui.selectable_value(value, *v, *label);
            }
        });
}

fn check(ui: &mut Ui, value: &mut bool, label: &str) -> bool {
    ui.add_space(2.0);
    let changed = ui
        .checkbox(value, RichText::new(label).size(15.0))
        .changed();
    ui.add_space(4.0);
    changed
}

pub fn settings(
    ctx: &egui::Context,
    st: &mut SettingsState,
    cfg: &mut Config,
    progress: &mut Progress,
    conn: Option<&Conn>,
) -> Pick {
    let mut out = Pick::Keep;
    let mut changed = false;
    let (_, close) = dialog(ctx, "settings", 500.0, |ui| {
        header(
            ui,
            "Settings",
            concat!("LocalPlayer ", env!("CARGO_PKG_VERSION")),
        );

        section(ui, "Server");
        match conn {
            Some(c) => {
                row(ui, "Library", |ui| {
                    if small_button(ui, "Switch server").clicked() {
                        out = Pick::ChangeServer;
                    }
                    ui.add_space(6.0);
                    ui.label(RichText::new(&c.name).color(MUTED));
                });
                ui.label(mono(&c.base, 11.0).color(MUTED));
                ui.add_space(4.0);
                if c.signed_in {
                    row(ui, "Signed in", |ui| {
                        if small_button(ui, "Sign out").clicked() {
                            out = Pick::SignOut;
                        }
                        ui.add_space(6.0);
                        ui.label(RichText::new(c.user.as_deref().unwrap_or("")).color(MUTED));
                    });
                }
            }
            None => row(ui, "Not connected", |ui| {
                if small_button(ui, "Connect").clicked() {
                    out = Pick::ChangeServer;
                }
            }),
        }

        section(ui, "Playback");
        changed |= check(ui, &mut cfg.resume, "Continue where I left off");
        changed |= check(
            ui,
            &mut cfg.autoplay_next,
            "Play the next episode automatically",
        );
        changed |= check(
            ui,
            &mut cfg.fullscreen_on_play,
            "Start videos in fullscreen",
        );
        let before = cfg.skip_seconds;
        row(ui, "Arrow keys skip", |ui| {
            combo(
                ui,
                "skip",
                &mut cfg.skip_seconds,
                &[
                    (5, "5 seconds"),
                    (10, "10 seconds"),
                    (15, "15 seconds"),
                    (30, "30 seconds"),
                ],
            );
        });
        ui.label(
            RichText::new("Hold Shift to skip six times as far.")
                .small()
                .color(MUTED),
        );
        changed |= before != cfg.skip_seconds;
        row(ui, "Play a file from this computer", |ui| {
            if small_button(ui, "Open file…").clicked() {
                out = Pick::OpenFiles;
            }
        });

        section(ui, "Library");
        let before = cfg.refresh_minutes;
        row(ui, "Check for new movies and episodes", |ui| {
            combo(
                ui,
                "refresh",
                &mut cfg.refresh_minutes,
                &[
                    (0, "Only on start"),
                    (5, "Every 5 minutes"),
                    (15, "Every 15 minutes"),
                    (60, "Every hour"),
                ],
            );
        });
        changed |= before != cfg.refresh_minutes;
        changed |= check(ui, &mut cfg.genre_rows, "Show a row for each genre on Home");
        if let Some(c) = conn {
            row(ui, "Reload the library now", |ui| {
                if small_button(ui, "Refresh").clicked() {
                    out = Pick::Refresh;
                }
            });
            row(ui, "Watch history", |ui| {
                if st.confirm_clear {
                    if small_button(ui, "Yes, clear it").clicked() {
                        progress.clear(&c.prefix());
                        progress.flush();
                        st.confirm_clear = false;
                    }
                    if small_button(ui, "Cancel").clicked() {
                        st.confirm_clear = false;
                    }
                } else if small_button(ui, "Clear").clicked() {
                    st.confirm_clear = true;
                }
            });
            if st.confirm_clear {
                ui.label(
                    RichText::new("Forget every resume point and watched mark for this server?")
                        .small()
                        .color(MUTED),
                );
            }
        }

        section(ui, "Appearance");
        ui.label(RichText::new("Interface size").size(15.0));
        ui.add_space(4.0);
        changed |= segmented(
            ui,
            &mut cfg.ui_scale,
            &[
                (0.9, "Small", None),
                (1.0, "Normal", None),
                (1.15, "Large", None),
                (1.3, "Huge", None),
            ],
        );

        if crate::integrate::appimage().is_some() {
            section(ui, "Desktop");
            if check(ui, &mut cfg.app_menu, "Show LocalPlayer in the app menu") {
                crate::integrate::sync(cfg.app_menu);
                changed = true;
            }
        }

        ui.add_space(22.0);
        ui.horizontal(|ui| {
            if super::secondary_button(ui, "Quit LocalPlayer").clicked() {
                out = Pick::Quit;
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if primary_button(ui, "   Done   ", true).clicked() {
                    out = Pick::Close;
                }
            });
        });
    });
    if changed {
        cfg.save();
    }
    if close && matches!(out, Pick::Keep) {
        out = Pick::Close;
    }
    if !matches!(out, Pick::Keep) {
        st.confirm_clear = false;
    }
    out
}

pub fn profile(ctx: &egui::Context, conn: &Conn, progress: &Progress) -> Pick {
    let mut out = Pick::Keep;
    let entries = progress.recent(&conn.prefix());
    let finished = |kind: &str| {
        entries
            .iter()
            .filter(|(id, e)| id.starts_with(kind) && e.watched())
            .count()
    };
    let in_progress = entries.iter().filter(|(_, e)| e.in_progress()).count();
    let secs: f64 = entries
        .iter()
        .map(|(_, e)| if e.dur > 0.0 { e.pos.min(e.dur) } else { e.pos })
        .sum();
    let name = conn.user.clone().unwrap_or_else(|| "Guest".into());

    let (_, close) = dialog(ctx, "profile", 460.0, |ui| {
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(56.0, 56.0), Sense::hover());
            avatar(ui.painter(), r.center(), 28.0, &name);
            ui.add_space(6.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.add_space(6.0);
                ui.label(bold(&name, 24.0));
                let how = if conn.signed_in {
                    "Signed in to"
                } else {
                    "Connected to"
                };
                ui.label(mono(format!("{how} {}", conn.name), 11.0).color(MUTED));
            });
        });

        section(ui, "Watching");
        egui::Grid::new("stats")
            .num_columns(2)
            .spacing([24.0, 10.0])
            .show(ui, |ui| {
                for (label, value) in [
                    ("Movies finished", finished("movie:").to_string()),
                    ("Episodes finished", finished("episode:").to_string()),
                    ("In progress", in_progress.to_string()),
                    (
                        "Time watched",
                        if secs > 0.0 {
                            fmt_runtime(secs)
                        } else {
                            "—".into()
                        },
                    ),
                ] {
                    ui.label(RichText::new(label).size(15.0));
                    ui.label(bold(value, 16.0).color(TEXT));
                    ui.end_row();
                }
            });
        ui.add_space(4.0);
        ui.label(
            RichText::new("Counted from this computer's watch history.")
                .small()
                .color(MUTED),
        );

        section(ui, "Server");
        ui.label(mono(&conn.base, 11.0).color(MUTED));
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if conn.admin && small_button(ui, "Open admin ↗").clicked() {
                super::chrome::open_url(&format!("{}/admin", conn.base));
            }
            if small_button(ui, "Settings").clicked() {
                out = Pick::OpenSettings;
            }
            if conn.signed_in && small_button(ui, "Sign out").clicked() {
                out = Pick::SignOut;
            }
        });

        ui.add_space(22.0);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if primary_button(ui, "   Done   ", true).clicked() {
                out = Pick::Close;
            }
        });
    });
    if close && matches!(out, Pick::Keep) {
        out = Pick::Close;
    }
    out
}
