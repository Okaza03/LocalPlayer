//! The wordmark shown after signing in, while the library loads.
use std::time::{Duration, Instant};

use eframe::egui::{self, pos2, Color32, Rect, Sense, TextureHandle, TextureOptions};

use super::BG;

/// On the way to the library it stays at least this long, so it doesn't just flash.
const HOLD: Duration = Duration::from_millis(1200);
/// Seconds.
const FADE_IN: f32 = 0.45;
const FADE_OUT: f32 = 0.35;

pub struct Splash {
    start: Instant,
    /// When it started fading out.
    leaving: Option<Instant>,
    logo: TextureHandle,
}

impl Splash {
    pub fn new(ctx: &egui::Context) -> Self {
        let png = include_bytes!("../../assets/valo-text-white.png");
        let img = image::load_from_memory(png)
            .expect("bundled wordmark is a valid PNG")
            .into_rgba8();
        let size = [img.width() as usize, img.height() as usize];
        let img = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
        Self {
            start: Instant::now(),
            leaving: None,
            logo: ctx.load_texture("wordmark", img, TextureOptions::LINEAR),
        }
    }

    /// Draws over the whole window. `ready`: the page underneath has loaded;
    /// `hold`: it's the library, so stay up for HOLD first (an error shows
    /// straight away). Returns false once it has faded out.
    pub fn ui(&mut self, ctx: &egui::Context, ready: bool, hold: bool) -> bool {
        let now = Instant::now();
        if self.leaving.is_none() && ready && (!hold || now >= self.start + HOLD) {
            self.leaving = Some(now);
        }
        let cover = match self.leaving {
            Some(t) => 1.0 - (now - t).as_secs_f32() / FADE_OUT,
            None => 1.0,
        };
        if cover <= 0.0 {
            return false;
        }
        let t = ((now - self.start).as_secs_f32() / FADE_IN).min(1.0);
        let logo = 1.0 - (1.0 - t).powi(3); // ease out
        let screen = ctx.screen_rect();
        egui::Area::new(egui::Id::new("splash"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                // swallows clicks meant for the page underneath
                let (rect, _) = ui.allocate_exact_size(screen.size(), Sense::click_and_drag());
                let p = ui.painter();
                p.rect_filled(rect, 0.0, BG.gamma_multiply(cover));
                let size = self.logo.size_vec2();
                let w = (rect.width() * 0.32).clamp(180.0, 360.0) * (0.94 + 0.06 * logo);
                p.image(
                    self.logo.id(),
                    Rect::from_center_size(rect.center(), size * (w / size.x)),
                    Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                    Color32::WHITE.gamma_multiply(cover * logo),
                );
            });
        ctx.request_repaint();
        true
    }
}
