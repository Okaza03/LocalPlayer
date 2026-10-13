//! Full-window playback of a queue (an episode run, a movie, or local files).
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Result;
use eframe::egui::{
    self, pos2, vec2, Align2, Color32, CursorIcon, FontId, Key, Rect, RichText, Sense, Vec2,
};

use super::chrome;
use super::{bold, gradient, mono, small_button, BG, ERROR, MUTED, TEXT};
use crate::audio::AudioOut;
use crate::config::{Config, Progress};
use crate::player::{Player, Source};
use crate::util::{fmt_time, Task};

pub struct PlayItem {
    pub title: String,
    pub subtitle: String,
    pub source: Source,
    /// Progress key; None for local files.
    pub key: Option<String>,
    /// Resume position in seconds.
    pub start: f64,
}

impl PlayItem {
    pub fn local(path: PathBuf) -> Self {
        let source = Source::File(path);
        Self {
            title: source.name(),
            subtitle: String::new(),
            source,
            key: None,
            start: 0.0,
        }
    }
}

pub struct Watch {
    items: Vec<PlayItem>,
    current: Option<usize>,
    player: Option<Player>,
    opening: Option<(usize, Task<Result<Player>>)>,
    queued: Option<usize>,
    tex: Option<egui::TextureHandle>,
    seek_drag: Option<f64>,
    status: String,
    show_list: bool,
    last_tick: Instant,
    last_motion: Instant,
    last_save: Instant,
}

impl Watch {
    pub fn new(items: Vec<PlayItem>) -> Self {
        Self {
            show_list: items.iter().any(|i| i.key.is_none()),
            queued: (!items.is_empty()).then_some(0),
            items,
            current: None,
            player: None,
            opening: None,
            tex: None,
            seek_drag: None,
            status: String::new(),
            last_tick: Instant::now(),
            last_motion: Instant::now(),
            last_save: Instant::now(),
        }
    }

    pub fn add_files(&mut self, files: Vec<PathBuf>) {
        let first = self.items.len();
        self.items.extend(files.into_iter().map(PlayItem::local));
        if self.items.len() > first {
            self.queued = Some(first);
            self.show_list = true;
        }
    }

    pub fn save_progress(&mut self, progress: &mut Progress) {
        let (Some(i), Some(p)) = (self.current, self.player.as_ref()) else {
            return;
        };
        if let Some(key) = &self.items[i].key {
            let pos = if p.finished() {
                p.meta.duration
            } else {
                p.clock()
            };
            if pos > 1.0 {
                progress.set(key, pos, p.meta.duration);
            }
        }
        self.last_save = Instant::now();
    }

    fn open_index(
        &mut self,
        ctx: &egui::Context,
        i: usize,
        audio: &AudioOut,
        progress: &mut Progress,
    ) {
        if self.opening.is_some() {
            self.queued = Some(i); // one open at a time; picked up when it lands
            return;
        }
        self.save_progress(progress);
        self.player = None; // drop (and join) the old decoder first
        self.tex = None;
        self.current = Some(i);
        self.status.clear();
        let Some(item) = self.items.get(i) else {
            return;
        };
        let (src, shared) = (item.source.clone(), audio.shared.clone());
        self.opening = Some((i, Task::spawn(ctx, move || Player::open(&src, shared))));
        let title = if item.subtitle.is_empty() {
            item.title.clone()
        } else {
            format!("{} — {}", item.title, item.subtitle)
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!("{title} — HomiePlayer")));
    }

    fn toggle_play(&mut self) {
        if let Some(p) = self.player.as_mut() {
            if p.finished() {
                p.seek(0.0);
                p.set_paused(false);
            } else {
                let paused = p.paused;
                p.set_paused(!paused);
            }
        }
    }

    fn skip(&mut self, delta: f64) {
        if let Some(p) = self.player.as_mut() {
            let mut t = p.clock() + delta;
            if p.meta.duration > 0.0 {
                t = t.min(p.meta.duration);
            }
            p.seek(t);
        }
    }

    fn has_next(&self) -> bool {
        self.current.is_some_and(|i| i + 1 < self.items.len())
    }

    fn toggle_fullscreen(ctx: &egui::Context) {
        let fs = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!fs));
    }

    /// Draws the player; returns true when the user wants to leave it.
    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        audio: &AudioOut,
        cfg: &mut Config,
        muted: &mut bool,
        progress: &mut Progress,
    ) -> bool {
        let (skip, autoplay) = (f64::from(cfg.skip_seconds.max(1)), cfg.autoplay_next);
        let volume = &mut cfg.volume;
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f64();
        self.last_tick = now;

        // a finished open
        if let Some((i, mut task)) = self.opening.take() {
            match task.poll() {
                None => self.opening = Some((i, task)),
                Some(r) if self.queued.is_some() => drop(r), // superseded, Drop cleans up
                Some(Ok(mut p)) => {
                    let start = self.items[i].start;
                    if start > 0.0 && (p.meta.duration <= 0.0 || start < p.meta.duration - 5.0) {
                        p.seek(start);
                    }
                    self.items[i].start = 0.0;
                    self.player = Some(p);
                }
                Some(Err(e)) => {
                    self.status = format!("Can't play {}: {e:#}", self.items[i].source.name())
                }
            }
        }
        if self.opening.is_none() {
            if let Some(q) = self.queued.take() {
                self.open_index(ctx, q, audio, progress);
            }
        }

        let (mut close, mut do_open, mut do_prev, mut do_next, mut do_toggle) =
            (false, false, false, false, false);
        let mut seek_to: Option<f64> = None;
        let mut skip_by: Option<f64> = None;
        let mut pick: Option<usize> = None;

        // keys
        let fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
        ctx.input(|i| {
            let shift = i.modifiers.shift;
            let step = if shift { skip * 6.0 } else { skip };
            if i.modifiers.command && i.key_pressed(Key::O) {
                do_open = true;
            }
            do_toggle |= i.key_pressed(Key::Space);
            if i.key_pressed(Key::ArrowRight) {
                skip_by = Some(step);
            }
            if i.key_pressed(Key::ArrowLeft) {
                skip_by = Some(-step);
            }
            if i.key_pressed(Key::ArrowUp) {
                *volume = (*volume + 0.05).min(1.0);
            }
            if i.key_pressed(Key::ArrowDown) {
                *volume = (*volume - 0.05).max(0.0);
            }
            if i.key_pressed(Key::M) {
                *muted = !*muted;
            }
            do_next |= i.key_pressed(Key::N);
            do_prev |= i.key_pressed(Key::P);
            if i.key_pressed(Key::Escape) || i.key_pressed(Key::Backspace) {
                if fullscreen {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                } else {
                    close = true;
                }
            }
        });
        if let Some(d) = skip_by {
            self.skip(d);
        }
        if ctx.input(|i| !i.modifiers.command && (i.key_pressed(Key::F) || i.key_pressed(Key::F11)))
        {
            Self::toggle_fullscreen(ctx);
        }
        if ctx.input(|i| i.key_pressed(Key::L)) {
            self.show_list = !self.show_list;
        }

        audio.shared.set_volume(*volume);
        audio.shared.set_muted(*muted);

        // advance playback, upload the newest due frame
        let mut finished = false;
        if let Some(p) = self.player.as_mut() {
            p.tick(dt);
            if let Some(f) = p.take_frame() {
                let img = egui::ColorImage::from_rgba_unmultiplied([f.w, f.h], &f.rgba);
                match self.tex.as_mut() {
                    Some(t) => t.set(img, egui::TextureOptions::LINEAR),
                    None => {
                        self.tex =
                            Some(ctx.load_texture("video", img, egui::TextureOptions::LINEAR))
                    }
                }
            }
            finished = p.finished() && !p.paused;
        }
        if finished {
            self.save_progress(progress);
            match self.current {
                Some(i) if autoplay && i + 1 < self.items.len() => {
                    self.open_index(ctx, i + 1, audio, progress)
                }
                _ => {
                    if let Some(p) = self.player.as_mut() {
                        p.set_paused(true);
                    }
                }
            }
        }
        if self.last_save.elapsed() > Duration::from_secs(5) {
            self.save_progress(progress);
        }

        // auto-hide controls while playing
        if ctx.input(|i| {
            i.pointer.delta() != Vec2::ZERO || i.pointer.any_pressed() || !i.keys_down.is_empty()
        }) {
            self.last_motion = now;
        }
        let playing = self.player.as_ref().is_some_and(|p| !p.paused);
        let hide_ui = playing
            && self.last_motion.elapsed()
                > Duration::from_millis(if fullscreen { 2500 } else { 3500 });

        if !hide_ui {
            egui::TopBottomPanel::bottom("controls")
                .frame(
                    egui::Frame::new()
                        .fill(BG)
                        .inner_margin(egui::Margin::symmetric(14, 8)),
                )
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        do_prev |= ui.button("⏮").on_hover_text("Previous (P)").clicked();
                        do_toggle |= ui
                            .button(if playing { "⏸" } else { "⏵" })
                            .on_hover_text("Play/Pause (Space)")
                            .clicked();
                        do_next |= ui
                            .add_enabled(self.has_next(), egui::Button::new("⏭"))
                            .on_hover_text("Next (N)")
                            .clicked();

                        let (pos, dur) = self
                            .player
                            .as_ref()
                            .map_or((0.0, 0.0), |p| (p.clock(), p.meta.duration));
                        let mut shown = self.seek_drag.unwrap_or(pos);
                        ui.label(mono(fmt_time(shown), 11.5));
                        ui.spacing_mut().slider_width = (ui.available_width() - 250.0).max(80.0);
                        let r = ui.add_enabled(
                            dur > 0.0,
                            egui::Slider::new(&mut shown, 0.0..=dur.max(0.001)).show_value(false),
                        );
                        if r.dragged() {
                            self.seek_drag = Some(shown);
                        }
                        if r.drag_stopped() {
                            seek_to = Some(shown);
                            self.seek_drag = None;
                        } else if r.changed() && !r.dragged() {
                            seek_to = Some(shown);
                        }
                        ui.label(mono(fmt_time(dur), 11.5).color(MUTED));

                        if ui.button(if *muted { "🔇" } else { "🔊" }).clicked() {
                            *muted = !*muted;
                        }
                        ui.spacing_mut().slider_width = 80.0;
                        ui.add(egui::Slider::new(volume, 0.0..=1.0).show_value(false));
                        if self.items.len() > 1
                            && ui.button("☰").on_hover_text("Queue (L)").clicked()
                        {
                            self.show_list = !self.show_list;
                        }
                        if ui.button("⛶").on_hover_text("Fullscreen (F)").clicked() {
                            Self::toggle_fullscreen(ctx);
                        }
                    });
                });

            if self.show_list && self.items.len() > 1 {
                egui::SidePanel::right("queue")
                    .default_width(260.0)
                    .show(ctx, |ui| {
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(mono("Up next", 12.0));
                            if ui
                                .small_button("📂")
                                .on_hover_text("Add files (Ctrl+O)")
                                .clicked()
                            {
                                do_open = true;
                            }
                        });
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for (i, it) in self.items.iter().enumerate() {
                                let label = if it.subtitle.is_empty() {
                                    it.title.clone()
                                } else {
                                    it.subtitle.clone()
                                };
                                if ui
                                    .selectable_label(self.current == Some(i), label)
                                    .clicked()
                                {
                                    pick = Some(i);
                                }
                            }
                        });
                    });
            }
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(Color32::BLACK))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let resp = ui.allocate_rect(rect, Sense::click());
                if let Some(t) = &self.tex {
                    let sz = t.size_vec2();
                    let scale = (rect.width() / sz.x).min(rect.height() / sz.y);
                    let dest = Rect::from_center_size(rect.center(), sz * scale);
                    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
                    ui.painter().image(t.id(), dest, uv, Color32::WHITE);
                } else if !self.status.is_empty() {
                    ui.painter().text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        &self.status,
                        FontId::proportional(18.0),
                        ERROR,
                    );
                } else if self.opening.is_some() || self.player.is_some() {
                    let c = rect.center();
                    let sp = Rect::from_center_size(c - vec2(0.0, 20.0), vec2(40.0, 40.0));
                    egui::Spinner::new()
                        .size(40.0)
                        .color(TEXT)
                        .paint_at(ui, sp);
                    let name = self
                        .current
                        .map(|i| self.items[i].title.clone())
                        .unwrap_or_default();
                    ui.painter().text(
                        c + vec2(0.0, 24.0),
                        Align2::CENTER_TOP,
                        name,
                        FontId::proportional(16.0),
                        MUTED,
                    );
                } else {
                    ui.painter().text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        "Drop a video here or press Ctrl+O",
                        FontId::proportional(18.0),
                        Color32::GRAY,
                    );
                }

                // title bar overlay
                if !hide_ui {
                    let bar = Rect::from_min_size(rect.min, vec2(rect.width(), 72.0));
                    let shade = Color32::from_black_alpha(170);
                    gradient(
                        ui.painter(),
                        bar,
                        [shade, shade, Color32::TRANSPARENT, Color32::TRANSPARENT],
                    );
                    chrome::drag_area(ui, Rect::from_min_size(bar.min, vec2(bar.width(), chrome::BAR_H)));
                    let mut child = ui
                        .new_child(egui::UiBuilder::new().max_rect(bar.shrink2(vec2(14.0, 10.0))));
                    child.horizontal(|ui| {
                        close |= small_button(ui, "‹ Back").on_hover_text("Esc").clicked();
                        ui.add_space(8.0);
                        if let Some(it) = self.current.and_then(|i| self.items.get(i)) {
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 0.0;
                                ui.label(bold(&it.title, 16.0).color(Color32::WHITE));
                                if !it.subtitle.is_empty() {
                                    ui.label(mono(&it.subtitle, 10.5).color(Color32::from_gray(200)));
                                }
                            });
                        }
                    });
                    let buttons = Rect::from_min_size(
                        pos2(bar.right() - 3.0 * chrome::BAR_H, bar.top()),
                        vec2(3.0 * chrome::BAR_H, chrome::BAR_H),
                    );
                    let mut child = ui.new_child(
                        egui::UiBuilder::new().max_rect(buttons).layout(egui::Layout::right_to_left(egui::Align::Center)),
                    );
                    child.spacing_mut().item_spacing.x = 0.0;
                    chrome::window_buttons(&mut child);
                }

                if resp.double_clicked() {
                    Self::toggle_fullscreen(ctx);
                } else if resp.clicked() {
                    do_toggle = true;
                }
            });

        if hide_ui {
            ctx.set_cursor_icon(CursorIcon::None);
        }

        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        if !dropped.is_empty() {
            self.add_files(dropped);
        }
        if do_open {
            if let Some(files) = crate::pick_files() {
                self.add_files(files);
            }
        }
        if do_prev {
            let restart = self.player.as_ref().is_some_and(|p| p.clock() > 3.0);
            match self.current {
                _ if restart => seek_to = Some(0.0),
                Some(i) if i > 0 => pick = Some(i - 1),
                _ => {}
            }
        }
        if do_next && self.has_next() {
            pick = self.current.map(|i| i + 1);
        }
        if do_toggle {
            self.toggle_play();
        }
        if let (Some(t), Some(p)) = (seek_to, self.player.as_mut()) {
            p.seek(t);
        }
        if let Some(i) = pick {
            self.open_index(ctx, i, audio, progress);
        }

        if close {
            self.save_progress(progress);
            progress.flush();
            if fullscreen {
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
            }
        }
        ctx.request_repaint_after(Duration::from_millis(if playing { 8 } else { 100 }));
        close
    }
}
