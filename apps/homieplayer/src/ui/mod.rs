//! Screens and the shared look.
pub mod browse;
pub mod chrome;
pub mod login;
pub mod settings;
pub mod setup;
pub mod watch;

use std::sync::Arc;

use eframe::egui::{
    self, pos2, text::LayoutJob, vec2, Color32, CornerRadius, FontData, FontDefinitions,
    FontFamily, FontId, Mesh, Painter, Rect, Response, RichText, Sense, Stroke, StrokeKind,
    TextFormat, TextureHandle, Ui,
};

use crate::util::color_for;

pub const BG: Color32 = Color32::from_rgb(18, 19, 23);
pub const SURFACE: Color32 = Color32::from_rgb(25, 26, 31);
pub const SURFACE_HI: Color32 = Color32::from_rgb(34, 35, 41);
pub const LINE: Color32 = Color32::from_rgb(44, 45, 51);
pub const LINE_HI: Color32 = Color32::from_rgb(86, 88, 96);
pub const ACCENT: Color32 = Color32::from_rgb(74, 124, 247);
pub const TEXT: Color32 = Color32::from_rgb(236, 236, 238);
pub const MUTED: Color32 = Color32::from_rgb(139, 141, 148);
/// Text on white (selected tabs, primary buttons).
pub const INK: Color32 = Color32::from_rgb(14, 15, 18);
pub const ERROR: Color32 = Color32::from_rgb(255, 120, 110);

fn font(bytes: &'static [u8]) -> Arc<FontData> {
    Arc::new(FontData::from_static(bytes))
}

pub fn apply_theme(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let fd = &mut fonts.font_data;
    fd.insert("sans".into(), font(include_bytes!("../../assets/fonts/NotoSans-Regular.ttf")));
    fd.insert("sans-bold".into(), font(include_bytes!("../../assets/fonts/NotoSans-Bold.ttf")));
    fd.insert("sans-black".into(), font(include_bytes!("../../assets/fonts/NotoSans-Black.ttf")));
    fd.insert("mono".into(), font(include_bytes!("../../assets/fonts/NotoSansMono-Medium.ttf")));
    let fallback = fonts.families[&FontFamily::Proportional].clone(); // keeps emoji/icons
    let fam = &mut fonts.families;
    fam.get_mut(&FontFamily::Proportional).unwrap().insert(0, "sans".into());
    fam.get_mut(&FontFamily::Monospace).unwrap().insert(0, "mono".into());
    for (name, first) in [("bold", "sans-bold"), ("black", "sans-black")] {
        let list = std::iter::once(first.to_owned()).chain(fallback.iter().cloned()).collect();
        fam.insert(FontFamily::Name(name.into()), list);
    }
    ctx.set_fonts(fonts);

    let mut style = (*ctx.style()).clone();
    use egui::TextStyle::*;
    style.text_styles = [
        (Heading, FontId::new(24.0, FontFamily::Name("bold".into()))),
        (Body, FontId::proportional(14.5)),
        (Button, FontId::proportional(14.0)),
        (Small, FontId::proportional(12.0)),
        (Monospace, FontId::monospace(12.5)),
    ]
    .into();
    style.spacing.button_padding = vec2(14.0, 6.0);
    style.spacing.item_spacing = vec2(10.0, 8.0);
    style.spacing.interact_size.y = 30.0;
    style.spacing.icon_width = 16.0;
    style.spacing.icon_width_inner = 9.0;
    style.spacing.menu_margin = egui::Margin::same(6);

    let v = &mut style.visuals;
    *v = egui::Visuals::dark();
    v.panel_fill = BG;
    v.window_fill = SURFACE;
    v.extreme_bg_color = Color32::from_rgb(22, 23, 27);
    v.faint_bg_color = SURFACE;
    v.override_text_color = Some(TEXT);
    v.hyperlink_color = TEXT;
    v.selection.bg_fill = Color32::from_white_alpha(40);
    v.selection.stroke = Stroke::new(1.0f32, TEXT);
    v.window_corner_radius = CornerRadius::same(4);
    v.menu_corner_radius = CornerRadius::same(4);
    v.window_stroke = Stroke::new(1.0f32, LINE);
    v.window_shadow = egui::Shadow { offset: [0, 8], blur: 24, spread: 0, color: Color32::from_black_alpha(140) };
    v.popup_shadow = v.window_shadow;
    for (w, fill) in [
        (&mut v.widgets.noninteractive, SURFACE),
        (&mut v.widgets.inactive, SURFACE),
        (&mut v.widgets.hovered, SURFACE_HI),
        (&mut v.widgets.active, Color32::from_rgb(44, 45, 52)),
        (&mut v.widgets.open, SURFACE_HI),
    ] {
        w.bg_fill = fill;
        w.weak_bg_fill = fill;
        w.corner_radius = CornerRadius::same(2);
        w.expansion = 0.0;
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0f32, LINE);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0f32, LINE);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0f32, LINE_HI);
    v.widgets.active.bg_stroke = Stroke::new(1.0f32, TEXT);
    v.widgets.open.bg_stroke = Stroke::new(1.0f32, LINE_HI);
    v.widgets.inactive.fg_stroke.color = TEXT;
    v.widgets.hovered.fg_stroke.color = TEXT;
    ctx.set_style(style);
}

// ----------------------------------------------------------------- text

/// Uppercase, letter-spaced monospace label (section headers, tabs, buttons).
pub fn mono(text: impl Into<String>, size: f32) -> RichText {
    RichText::new(text.into().to_uppercase())
        .family(FontFamily::Monospace)
        .size(size)
        .extra_letter_spacing(size * 0.14)
}

pub fn bold(text: impl Into<String>, size: f32) -> RichText {
    RichText::new(text.into()).family(FontFamily::Name("bold".into())).size(size)
}

pub fn black_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("black".into()))
}

/// The "HOMIEPLAYER" word mark.
pub fn logo(ui: &mut Ui, size: f32) {
    ui.label(RichText::new("HOMIEPLAYER").font(black_font(size)).extra_letter_spacing(size * 0.04).color(TEXT));
}

/// Muted section header, e.g. "PLAYBACK".
pub fn section(ui: &mut Ui, text: &str) {
    ui.add_space(10.0);
    ui.label(mono(text, 11.5).color(MUTED));
    ui.add_space(2.0);
}

// -------------------------------------------------------------- buttons

pub fn primary_button(ui: &mut Ui, text: &str, enabled: bool) -> Response {
    let b = egui::Button::new(mono(text, 12.5).color(INK))
        .fill(Color32::WHITE)
        .stroke(Stroke::NONE)
        .min_size(vec2(0.0, 38.0))
        .corner_radius(2);
    ui.add_enabled(enabled, b)
}

pub fn secondary_button(ui: &mut Ui, text: &str) -> Response {
    let b = egui::Button::new(mono(text, 12.5).color(TEXT))
        .fill(Color32::from_black_alpha(60))
        .stroke(Stroke::new(1.0f32, LINE_HI))
        .min_size(vec2(0.0, 38.0))
        .corner_radius(2);
    ui.add(b)
}

pub fn small_button(ui: &mut Ui, text: &str) -> Response {
    let b = egui::Button::new(mono(text, 11.5).color(TEXT))
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::new(1.0f32, LINE_HI))
        .min_size(vec2(0.0, 30.0))
        .corner_radius(2);
    ui.add(b)
}

pub fn text_field(ui: &mut Ui, value: &mut String, hint: &str, password: bool) -> Response {
    ui.add(
        egui::TextEdit::singleline(value)
            .hint_text(hint)
            .password(password)
            .desired_width(f32::INFINITY)
            .margin(egui::Margin::symmetric(10, 9))
            .font(FontId::proportional(15.0)),
    )
}

fn tab_galley(ui: &Ui, label: &str, count: Option<usize>, selected: bool) -> Arc<egui::Galley> {
    let font = FontId::monospace(12.0);
    let fmt = |color| TextFormat { font_id: font.clone(), color, extra_letter_spacing: 1.6, ..Default::default() };
    let mut job = LayoutJob::default();
    job.append(&label.to_uppercase(), 0.0, fmt(if selected { INK } else { TEXT }));
    if let Some(c) = count {
        job.append(&c.to_string(), 9.0, fmt(if selected { Color32::from_gray(90) } else { MUTED }));
    }
    ui.fonts(|f| f.layout_job(job))
}

/// A top-bar tab: plain text, or a white block when selected.
pub fn tab(ui: &mut Ui, label: &str, selected: bool) -> Response {
    let g = tab_galley(ui, label, None, selected);
    let (r, resp) = ui.allocate_exact_size(vec2(g.size().x + 16.0, 24.0), Sense::click());
    let p = ui.painter();
    if selected {
        p.rect_filled(r, 1.0, Color32::WHITE);
    } else if resp.hovered() {
        p.rect_filled(r, 1.0, SURFACE_HI);
    }
    p.galley(r.center() - g.size() / 2.0, g, TEXT);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A bordered row of options, the selected one white. Returns true on change.
pub fn segmented<T: PartialEq + Clone>(ui: &mut Ui, value: &mut T, items: &[(T, &str, Option<usize>)]) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let mut rects = Vec::new();
        for (v, label, count) in items {
            let sel = *value == *v;
            let g = tab_galley(ui, label, *count, sel);
            let (r, resp) = ui.allocate_exact_size(vec2(g.size().x + 24.0, 34.0), Sense::click());
            let p = ui.painter();
            if sel {
                p.rect_filled(r, 0.0, Color32::WHITE);
            } else if resp.hovered() {
                p.rect_filled(r, 0.0, SURFACE_HI);
            }
            p.galley(r.center() - g.size() / 2.0, g, TEXT);
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !sel {
                *value = v.clone();
                changed = true;
            }
            rects.push(r);
        }
        if let (Some(a), Some(b)) = (rects.first(), rects.last()) {
            let p = ui.painter();
            for r in &rects[1..] {
                p.vline(r.left(), r.y_range(), Stroke::new(1.0f32, LINE));
            }
            p.rect_stroke(a.union(*b), 0.0, Stroke::new(1.0f32, LINE), StrokeKind::Inside);
        }
    });
    changed
}

/// A full-width row in a dropdown menu.
pub fn menu_item(ui: &mut Ui, text: &str) -> bool {
    let w = ui.available_width().max(200.0);
    let (r, resp) = ui.allocate_exact_size(vec2(w, 32.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(r, 2.0, SURFACE_HI);
    }
    let pos = pos2(r.left() + 10.0, r.center().y);
    ui.painter().text(pos, egui::Align2::LEFT_CENTER, text, FontId::proportional(14.5), TEXT);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

// ---------------------------------------------------------------- panels

/// A centered card used by the setup and login screens.
pub fn card<R>(ctx: &egui::Context, add: impl FnOnce(&mut Ui) -> R) -> R {
    let mut out = None;
    egui::CentralPanel::default().frame(egui::Frame::new().fill(BG)).show(ctx, |ui| {
        let full = ui.max_rect();
        let w = full.width().clamp(280.0, 440.0);
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(((full.height() - 540.0) / 2.0).max(24.0));
            ui.vertical_centered(|ui| {
                ui.set_max_width(w);
                egui::Frame::new()
                    .fill(SURFACE)
                    .stroke(Stroke::new(1.0f32, LINE))
                    .corner_radius(4)
                    .inner_margin(28)
                    .show(ui, |ui| {
                        ui.set_width(w - 56.0);
                        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                            out = Some(add(ui));
                        });
                    });
            });
            ui.add_space(24.0);
        });
    });
    out.unwrap()
}

/// A rect filled with a gradient; colors are top-left, top-right, bottom-right, bottom-left.
pub fn gradient(p: &Painter, r: Rect, c: [Color32; 4]) {
    let mut m = Mesh::default();
    for (pos, col) in [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()].into_iter().zip(c) {
        m.colored_vertex(pos, col);
    }
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    p.add(m);
}

/// Paint `tex` so it covers `r`, cropping whatever doesn't fit.
pub fn image_cover(p: &Painter, r: Rect, tex: &TextureHandle, radius: u8, tint: Color32) {
    let sz = tex.size_vec2();
    let (ta, ra) = (sz.x / sz.y, r.width() / r.height());
    let uv = if ta > ra {
        let w = ra / ta;
        Rect::from_min_max(pos2(0.5 - w / 2.0, 0.0), pos2(0.5 + w / 2.0, 1.0))
    } else {
        let h = ta / ra;
        // keep the top of tall images (faces, titles) rather than the middle
        Rect::from_min_max(pos2(0.0, 0.15 * (1.0 - h)), pos2(1.0, 0.15 * (1.0 - h) + h))
    };
    if radius == 0 {
        p.image(tex.id(), r, uv, tint);
    } else {
        p.add(egui::epaint::RectShape::filled(r, radius, tint).with_texture(tex.id(), uv));
    }
}

/// Artwork stand-in: a colored tile with the title on it.
pub fn placeholder(p: &Painter, r: Rect, title: &str, radius: u8) {
    let c = color_for(title);
    p.rect_filled(r, radius, c);
    let dark = Color32::from_black_alpha(140);
    gradient(p, r, [Color32::TRANSPARENT, Color32::TRANSPARENT, dark, dark]);
    let size = (r.width() / 9.0).clamp(11.0, 22.0);
    let font = FontId::new(size, FontFamily::Name("bold".into()));
    let galley = p.layout(title.to_owned(), font, Color32::WHITE, r.width() - 16.0);
    let pos = r.center() - galley.size() / 2.0;
    p.galley(pos, galley, Color32::WHITE);
}

pub fn spinner_text(ui: &mut Ui, text: &str) {
    ui.horizontal(|ui| {
        ui.add(egui::Spinner::new().size(16.0).color(TEXT));
        ui.label(mono(text, 11.5).color(MUTED));
    });
}

/// What a screen asks the app to do next.
pub enum Action {
    None,
    /// The server answered; sign in or go straight to the library.
    Connected(crate::api::Client),
    SignedIn {
        client: crate::api::Client,
        user: String,
        remember: bool,
    },
    /// The saved session was rejected.
    NeedLogin(crate::api::Client),
    SignOut(crate::api::Client),
    ChangeServer,
    Refresh,
    OpenFiles,
    OpenSettings,
    OpenProfile,
    Quit,
    Play(Vec<watch::PlayItem>),
}
