//! The Settings dialog.
use eframe::egui::{self, vec2, Align, Color32, Layout, RichText, Sense, Stroke, Ui};

use super::theme::{self, Mode, Palette, Skin};
use super::{
    bold, mono, primary_button, section, segmented, small_button, ACCENT, BG, LINE, LINE_HI, MUTED,
    TEXT,
};
use crate::config::Config;

/// The connection the dialogs describe.
pub struct Conn {
    pub name: String,
    pub base: String,
    pub user: Option<String>,
    pub signed_in: bool,
}

pub enum Pick {
    Keep,
    Close,
    ChangeServer,
    Refresh,
    OpenFiles,
    SignOut,
    Quit,
}

pub(super) fn dialog<R>(
    ctx: &egui::Context,
    id: &str,
    width: f32,
    add: impl FnOnce(&mut Ui) -> R,
) -> (R, bool) {
    let frame = egui::Frame::new()
        .fill(BG())
        .stroke(Stroke::new(1.0f32, LINE()))
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

pub(super) fn header(ui: &mut Ui, title: &str, sub: &str) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(36.0, 36.0), Sense::hover());
        let p = ui.painter();
        p.rect_filled(r, 7.0, ACCENT());
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
            ui.label(mono(sub, 11.0).color(MUTED()));
        });
    });
    ui.add_space(8.0);
}

/// `System` as dark or light, the way the desktop is set now.
fn resolved(ctx: &egui::Context, m: Mode) -> Mode {
    match (m, ctx.system_theme()) {
        (Mode::System, Some(egui::Theme::Light)) => Mode::Light,
        (Mode::System, _) => Mode::Dark,
        (m, _) => m,
    }
}

/// A small picture of a look: its background, a poster row, a button and
/// the accent, with the name under it.
fn swatch(ui: &mut Ui, pal: &Palette, name: &str, selected: bool) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(108.0, 94.0), Sense::click());
    let p = ui.painter();
    let pic = egui::Rect::from_min_size(r.min, vec2(r.width(), 66.0));
    p.rect_filled(pic, 6.0, pal.bg);
    // poster row
    for i in 0..3 {
        let pr = egui::Rect::from_min_size(
            pic.min + vec2(10.0 + i as f32 * 22.0, 26.0),
            vec2(18.0, 28.0),
        );
        p.rect_filled(pr, 2.0, if i == 0 { pal.accent } else { pal.surface_hi });
    }
    // title and a button
    p.rect_filled(
        egui::Rect::from_min_size(pic.min + vec2(10.0, 10.0), vec2(40.0, 5.0)),
        2.0,
        pal.text,
    );
    p.rect_filled(
        egui::Rect::from_min_size(pic.min + vec2(78.0, 8.0), vec2(20.0, 9.0)),
        3.0,
        pal.primary,
    );
    let ring = if selected {
        Stroke::new(2.0f32, ACCENT())
    } else if resp.hovered() {
        Stroke::new(1.0f32, LINE_HI())
    } else {
        Stroke::new(1.0f32, LINE())
    };
    p.rect_stroke(pic, 6.0, ring, egui::StrokeKind::Outside);
    p.text(
        egui::pos2(r.left() + 2.0, pic.bottom() + 8.0),
        egui::Align2::LEFT_TOP,
        name,
        egui::FontId::proportional(13.0),
        if selected { TEXT() } else { MUTED() },
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A small drawing of a style's layout: sidebar or top tabs, the kind of
/// banner, and poster or landscape cards.
fn layout_swatch(ui: &mut Ui, skin: Skin, pal: &Palette, selected: bool) -> egui::Response {
    use theme::{Hero, Nav, Tiles};
    let spec = skin.spec();
    let lay = &spec.layout;
    let (r, resp) = ui.allocate_exact_size(vec2(108.0, 94.0), Sense::click());
    let p = ui.painter();
    let pic = egui::Rect::from_min_size(r.min, vec2(r.width(), 66.0));
    p.rect_filled(pic, 6.0, pal.bg);
    let mut body = pic.shrink(6.0);
    if lay.nav == Nav::Side {
        let side = egui::Rect::from_min_size(body.min, vec2(16.0, body.height()));
        for i in 0..3 {
            p.rect_filled(
                egui::Rect::from_min_size(
                    side.min + vec2(2.0, 2.0 + i as f32 * 6.0),
                    vec2(11.0, 3.0),
                ),
                1.0,
                if i == 0 { pal.text } else { pal.muted },
            );
        }
        body.min.x += 20.0;
    } else {
        for i in 0..3 {
            p.rect_filled(
                egui::Rect::from_min_size(body.min + vec2(i as f32 * 12.0, 0.0), vec2(9.0, 3.0)),
                1.0,
                if i == 0 { pal.text } else { pal.muted },
            );
        }
        body.min.y += 7.0;
    }
    let art = pal.surface_hi.gamma_multiply(1.6);
    let banner_h = 22.0;
    match lay.hero {
        Hero::Banner => {
            let b = egui::Rect::from_min_size(body.min, vec2(body.width(), banner_h));
            p.rect_filled(egui::Rect::from_min_max(b.center_top(), b.max), 2.0, art);
            p.rect_filled(
                egui::Rect::from_min_size(b.min + vec2(2.0, 12.0), vec2(26.0, 4.0)),
                1.0,
                pal.text,
            );
        }
        Hero::FullBleed => {
            let b = egui::Rect::from_min_size(body.min, vec2(body.width(), banner_h));
            p.rect_filled(b, 1.0, art);
            p.rect_filled(
                egui::Rect::from_min_size(b.min + vec2(3.0, 12.0), vec2(26.0, 4.0)),
                1.0,
                pal.text,
            );
        }
        Hero::Carousel => {
            let b = egui::Rect::from_min_max(
                body.min + vec2(8.0, 0.0),
                pos(body.right() - 8.0, body.top() + banner_h),
            );
            p.rect_filled(
                egui::Rect::from_min_max(
                    pos(body.left(), b.top() + 3.0),
                    pos(b.left() - 2.0, b.bottom() - 3.0),
                ),
                2.0,
                art.gamma_multiply(0.6),
            );
            p.rect_filled(
                egui::Rect::from_min_max(
                    pos(b.right() + 2.0, b.top() + 3.0),
                    pos(body.right(), b.bottom() - 3.0),
                ),
                2.0,
                art.gamma_multiply(0.6),
            );
            p.rect_filled(b, 3.0, art);
            p.rect_filled(
                egui::Rect::from_min_size(b.min + vec2(3.0, 12.0), vec2(22.0, 4.0)),
                1.0,
                pal.text,
            );
        }
        Hero::None => {
            for i in 0..4 {
                p.rect_filled(
                    egui::Rect::from_min_size(
                        body.min + vec2(i as f32 * 13.0, 0.0),
                        vec2(11.0, 4.0),
                    ),
                    2.0,
                    if i == 0 { pal.text } else { pal.surface_hi },
                );
            }
        }
    }
    let top = if lay.hero == Hero::None {
        body.top() + 8.0
    } else {
        body.top() + banner_h + 4.0
    };
    let (tw, th) = if lay.tiles == Tiles::Landscape {
        (20.0, 11.0)
    } else {
        (12.0, 17.0)
    };
    let rows = if lay.hero == Hero::None { 3 } else { 1 };
    for row in 0..rows {
        let mut x = body.left();
        let y = top + row as f32 * (th + 3.0);
        let mut i = 0;
        while x + tw <= body.right() + 0.5 {
            let c = if i == 0 && row == 0 { pal.accent } else { art };
            p.rect_filled(egui::Rect::from_min_size(pos(x, y), vec2(tw, th)), 1.5, c);
            x += tw + 3.0;
            i += 1;
        }
    }
    let ring = if selected {
        Stroke::new(2.0f32, ACCENT())
    } else if resp.hovered() {
        Stroke::new(1.0f32, LINE_HI())
    } else {
        Stroke::new(1.0f32, LINE())
    };
    p.rect_stroke(pic, 6.0, ring, egui::StrokeKind::Outside);
    p.text(
        egui::pos2(r.left() + 2.0, pic.bottom() + 8.0),
        egui::Align2::LEFT_TOP,
        spec.name,
        egui::FontId::proportional(13.0),
        if selected { TEXT() } else { MUTED() },
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn pos(x: f32, y: f32) -> egui::Pos2 {
    egui::pos2(x, y)
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

pub fn settings(ctx: &egui::Context, cfg: &mut Config, conn: Option<&Conn>) -> Pick {
    let mut out = Pick::Keep;
    let mut changed = false;
    let (_, close) = dialog(ctx, "settings", 500.0, |ui| {
        header(ui, "Settings", concat!("Valo ", env!("CARGO_PKG_VERSION")));

        section(ui, "Server");
        match conn {
            Some(c) => {
                row(ui, "Library", |ui| {
                    if small_button(ui, "Switch server").clicked() {
                        out = Pick::ChangeServer;
                    }
                    ui.add_space(6.0);
                    ui.label(RichText::new(&c.name).color(MUTED()));
                });
                ui.label(mono(&c.base, 11.0).color(MUTED()));
                ui.add_space(4.0);
                if c.signed_in {
                    row(ui, "Signed in", |ui| {
                        if small_button(ui, "Sign out").clicked() {
                            out = Pick::SignOut;
                        }
                        ui.add_space(6.0);
                        ui.label(RichText::new(c.user.as_deref().unwrap_or("")).color(MUTED()));
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
        changed |= check(
            ui,
            &mut cfg.discord_presence,
            "Show what I'm watching on Discord",
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
                .color(MUTED()),
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
        if conn.is_some() {
            row(ui, "Reload the library now", |ui| {
                if small_button(ui, "Refresh").clicked() {
                    out = Pick::Refresh;
                }
            });
        }

        section(ui, "Appearance");
        ui.label(RichText::new("App style").size(15.0));
        ui.label(
            RichText::new(
                "How the pages and the player are laid out, like the service it's named after.",
            )
            .small()
            .color(MUTED()),
        );
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(10.0, 10.0);
            for s in Skin::ALL {
                let pal = theme::palette(s, resolved(ctx, cfg.theme));
                if layout_swatch(ui, s, &pal, cfg.style == s).clicked() && cfg.style != s {
                    cfg.style = s;
                    changed = true;
                }
            }
        });
        ui.add_space(12.0);
        ui.label(RichText::new("Color theme").size(15.0));
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(10.0, 10.0);
            for (m, name) in Mode::ALL {
                let pal = theme::palette(cfg.style, resolved(ctx, m));
                if swatch(ui, &pal, name, cfg.theme == m).clicked() && cfg.theme != m {
                    cfg.theme = m;
                    changed = true;
                }
            }
        });
        ui.add_space(12.0);
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
            if check(ui, &mut cfg.app_menu, "Show Valo in the app menu") {
                crate::integrate::sync(cfg.app_menu);
                changed = true;
            }
        }

        ui.add_space(22.0);
        ui.horizontal(|ui| {
            if super::secondary_button(ui, "Quit Valo").clicked() {
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
    out
}
