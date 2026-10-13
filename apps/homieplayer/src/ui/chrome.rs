//! Window chrome: our own title bar (the window has no system decorations),
//! the status bar, and edge resizing.
use eframe::egui::{
    self, vec2, Align, Color32, CursorIcon, Id, Layout, Pos2, Rect, ResizeDirection, Response,
    Sense, Stroke, Ui, ViewportCommand,
};

use super::{mono, BG, LINE, MUTED, SURFACE_HI, TEXT};

pub const BAR_H: f32 = 46.0;

#[derive(Clone, Copy)]
pub enum Icon {
    Refresh,
    Settings,
    Minimize,
    Maximize,
    Restore,
    Close,
}

/// Top bar: word mark, `left` (tabs), `right` (search, buttons), window buttons.
pub fn title_bar(ctx: &egui::Context, left: impl FnOnce(&mut Ui), right: impl FnOnce(&mut Ui)) {
    egui::TopBottomPanel::top("title_bar")
        .exact_height(BAR_H)
        .frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin {
            left: 18,
            right: 0,
            top: 0,
            bottom: 0,
        }))
        .show(ctx, |ui| {
            drag_area(ui, ui.max_rect());
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                super::logo(ui, 19.0);
                ui.add_space(14.0);
                vsep(ui);
                ui.add_space(10.0);
                left(ui);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    window_buttons(ui);
                    vsep(ui);
                    right(ui);
                });
            });
        });
}

/// Dragging moves the window; double-click maximizes.
pub fn drag_area(ui: &mut Ui, rect: Rect) {
    let r = ui.interact(
        rect,
        Id::new("window_drag").with(rect.top() as i32),
        Sense::click_and_drag(),
    );
    if r.drag_started_by(egui::PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }
    if r.double_clicked() {
        toggle_maximized(ui.ctx());
    }
}

fn toggle_maximized(ctx: &egui::Context) {
    let max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
    ctx.send_viewport_cmd(ViewportCommand::Maximized(!max));
}

/// Minimize / maximize / close, laid out right-to-left.
pub fn window_buttons(ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    if icon_button(ui, Icon::Close, "Close").clicked() {
        ctx.send_viewport_cmd(ViewportCommand::Close);
    }
    let max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
    if icon_button(
        ui,
        if max { Icon::Restore } else { Icon::Maximize },
        if max { "Restore" } else { "Maximize" },
    )
    .clicked()
    {
        toggle_maximized(&ctx);
    }
    if icon_button(ui, Icon::Minimize, "Minimize").clicked() {
        ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
    }
}

/// A full-height vertical divider.
pub fn vsep(ui: &mut Ui) {
    let h = ui.available_height().min(BAR_H);
    let (r, _) = ui.allocate_exact_size(vec2(1.0, h), Sense::hover());
    ui.painter()
        .vline(r.center().x, r.y_range(), Stroke::new(1.0f32, LINE));
}

/// A square bar button with a drawn icon.
pub fn icon_button(ui: &mut Ui, icon: Icon, tip: &str) -> Response {
    let (r, resp) = ui.allocate_exact_size(
        vec2(BAR_H, BAR_H.min(ui.available_height())),
        Sense::click(),
    );
    let p = ui.painter();
    let hot = resp.hovered();
    if hot {
        let fill = if matches!(icon, Icon::Close) {
            Color32::from_rgb(196, 43, 28)
        } else {
            SURFACE_HI
        };
        p.rect_filled(r, 0.0, fill);
    }
    let col = if hot {
        Color32::WHITE
    } else {
        Color32::from_gray(200)
    };
    draw_icon(p, r.center(), icon, col);
    resp.on_hover_text(tip)
}

pub fn draw_icon(p: &egui::Painter, c: Pos2, icon: Icon, col: Color32) {
    let s = Stroke::new(1.4f32, col);
    match icon {
        Icon::Minimize => {
            p.hline(c.x - 6.0..=c.x + 6.0, c.y, s);
        }
        Icon::Maximize => {
            p.rect_stroke(
                Rect::from_center_size(c, vec2(11.0, 11.0)),
                0.0,
                s,
                egui::StrokeKind::Inside,
            );
        }
        Icon::Restore => {
            p.rect_stroke(
                Rect::from_center_size(c + vec2(-1.5, 1.5), vec2(9.0, 9.0)),
                0.0,
                s,
                egui::StrokeKind::Inside,
            );
            p.line_segment([c + vec2(-1.5, -4.5), c + vec2(4.5, -4.5)], s);
            p.line_segment([c + vec2(4.5, -4.5), c + vec2(4.5, 1.5)], s);
        }
        Icon::Close => {
            p.line_segment([c + vec2(-6.0, -6.0), c + vec2(6.0, 6.0)], s);
            p.line_segment([c + vec2(6.0, -6.0), c + vec2(-6.0, 6.0)], s);
        }
        Icon::Refresh => {
            let s = Stroke::new(1.8f32, col);
            let r = 7.0;
            let pts: Vec<Pos2> = (0..=24)
                .map(|i| {
                    let a = -0.35 * std::f32::consts::PI
                        + i as f32 / 24.0 * 1.65 * std::f32::consts::PI;
                    c + vec2(a.cos(), a.sin()) * r
                })
                .collect();
            let tip = pts[0];
            p.add(egui::Shape::line(pts, s));
            p.add(egui::Shape::convex_polygon(
                vec![
                    tip + vec2(-4.5, -2.0),
                    tip + vec2(2.5, -3.5),
                    tip + vec2(1.0, 3.5),
                ],
                col,
                Stroke::NONE,
            ));
        }
        Icon::Settings => {
            for i in 0..8 {
                let a = i as f32 / 8.0 * std::f32::consts::TAU;
                let d = vec2(a.cos(), a.sin());
                p.line_segment([c + d * 5.0, c + d * 8.5], Stroke::new(3.0f32, col));
            }
            p.circle_filled(c, 6.2, col);
            p.circle_filled(c, 2.6, BG);
        }
    }
}

/// Bottom status line: small monospace text on both sides.
pub fn status_bar(ctx: &egui::Context, left: &str, right: &str) {
    egui::TopBottomPanel::bottom("status_bar")
        .exact_height(30.0)
        .frame(
            egui::Frame::new()
                .fill(BG)
                .inner_margin(egui::Margin::symmetric(18, 0)),
        )
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.add(egui::Label::new(mono(left, 10.5).color(MUTED)).truncate());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(mono(right, 10.5).color(MUTED));
                });
            });
        });
}

/// Resize from the window edges (the window has no system frame), plus a hairline border.
pub fn window_frame(ctx: &egui::Context) {
    let (max, fs) = ctx.input(|i| {
        (
            i.viewport().maximized.unwrap_or(false),
            i.viewport().fullscreen.unwrap_or(false),
        )
    });
    if max || fs {
        return;
    }
    let screen = ctx.screen_rect();
    ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        Id::new("window_border"),
    ))
    .rect_stroke(
        screen,
        0.0,
        Stroke::new(1.0f32, LINE),
        egui::StrokeKind::Inside,
    );
    let Some(p) = ctx.input(|i| i.pointer.hover_pos()) else {
        return;
    };
    let m = 5.0;
    let (l, r, t, b) = (
        p.x - screen.left() < m,
        screen.right() - p.x < m,
        p.y - screen.top() < m,
        screen.bottom() - p.y < m,
    );
    use ResizeDirection::*;
    let dir = match (l, r, t, b) {
        (true, _, true, _) => NorthWest,
        (_, true, true, _) => NorthEast,
        (true, _, _, true) => SouthWest,
        (_, true, _, true) => SouthEast,
        (true, ..) => West,
        (_, true, ..) => East,
        (_, _, true, _) => North,
        (.., true) => South,
        _ => return,
    };
    ctx.set_cursor_icon(match dir {
        NorthWest => CursorIcon::ResizeNorthWest,
        NorthEast => CursorIcon::ResizeNorthEast,
        SouthWest => CursorIcon::ResizeSouthWest,
        SouthEast => CursorIcon::ResizeSouthEast,
        West => CursorIcon::ResizeWest,
        East => CursorIcon::ResizeEast,
        North => CursorIcon::ResizeNorth,
        South => CursorIcon::ResizeSouth,
    });
    if ctx.input(|i| i.pointer.primary_pressed()) {
        ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
    }
}

/// Open a link in the user's browser.
pub fn open_url(url: &str) {
    if let Err(e) = std::process::Command::new("xdg-open").arg(url).spawn() {
        eprintln!("can't open {url}: {e}");
    }
}

/// Avatar circle with the user's initial.
pub fn avatar(p: &egui::Painter, c: Pos2, r: f32, name: &str) {
    p.circle_filled(c, r, super::ACCENT);
    let initial = name
        .chars()
        .next()
        .unwrap_or('?')
        .to_uppercase()
        .to_string();
    p.text(
        c,
        egui::Align2::CENTER_CENTER,
        initial,
        super::black_font(r * 1.05),
        TEXT,
    );
}
