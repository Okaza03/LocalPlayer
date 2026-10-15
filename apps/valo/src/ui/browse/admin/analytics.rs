//! Admin › Analytics: plays per day, the most watched titles and each
//! user's watching, from `GET /api/admin/analytics`.
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Rect, RichText, Sense, Stroke, Ui};

use super::super::details::{caps, padded, panel, GOLD, TEAL};
use super::super::people::{ago, bar, columns_with, stats};
use super::{not_set_up, Cx, Remote, Section};
use crate::api::{Analytics, DayStat};
use crate::ui::{bold, mono, segmented, ERROR, LINE, MUTED, TEXT};

pub(in super::super) struct State {
    days: u32,
    data: Remote<Analytics>,
    /// The range `data` is for.
    loaded: u32,
    /// Which kind the top lists show: None for both.
    kind: Option<&'static str>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            days: 30,
            data: Remote::default(),
            loaded: 30,
            kind: None,
        }
    }
}

impl State {
    pub fn refresh(&mut self) {
        self.data.retry();
    }

    pub fn missing(&self) -> bool {
        self.data.missing
    }

    pub fn ui(&mut self, ui: &mut Ui, cx: &mut Cx) {
        self.data.poll(&mut cx.login);
        if self.days != self.loaded && !self.data.loading() {
            self.data.retry();
            self.data.data = None;
        }
        if self.data.idle() || (self.data.data.is_some() && self.data.stale(300)) {
            let (c, days) = (cx.client.clone(), self.days);
            self.loaded = days;
            self.data.load(&cx.ctx, move || c.admin_analytics(days));
        }
        if self.data.missing {
            padded(ui, |ui| {
                if not_set_up(ui, Section::Analytics, &cx.client.info.name) {
                    self.data.retry();
                }
            });
            return;
        }

        padded(ui, |ui| {
            ui.horizontal(|ui| {
                segmented(
                    ui,
                    &mut self.days,
                    &[
                        (7, "7 days", None),
                        (30, "30 days", None),
                        (90, "90 days", None),
                        (365, "Year", None),
                    ],
                );
                if self.data.loading() {
                    ui.add_space(10.0);
                    ui.add(egui::Spinner::new().size(16.0).color(MUTED));
                }
            });
            ui.add_space(18.0);
            if let Some(e) = &self.data.error {
                ui.label(RichText::new(format!("Couldn't load analytics: {e}")).color(ERROR));
            }
        });
        let Some(a) = &self.data.data else {
            return;
        };

        padded(ui, |ui| {
            stats(
                ui,
                &[
                    (a.plays.to_string(), "Plays"),
                    (format!("{:.0}", a.hours), "Hours watched"),
                    (a.active_users.to_string(), "Active users"),
                    (
                        if a.plays > 0 {
                            format!("{:.1}", a.hours / a.plays as f64)
                        } else {
                            "—".into()
                        },
                        "Hours per play",
                    ),
                ],
            );
            ui.add_space(6.0);
            panel(ui, |ui| {
                caps(ui, "Plays per day", TEAL);
                ui.add_space(12.0);
                day_chart(ui, &a.days);
            });
        });
        ui.add_space(24.0);

        let kind = &mut self.kind;
        let fits =
            |k: &Option<String>, want: Option<&str>| want.is_none_or(|w| k.as_deref() == Some(w));
        columns_with(
            ui,
            kind,
            |ui, kind| {
                caps(ui, "Most watched", TEAL);
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(bold("Top titles", 22.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        segmented(
                            ui,
                            kind,
                            &[
                                (None, "Both", None),
                                (Some("movie"), "Movies", None),
                                (Some("series"), "Series", None),
                            ],
                        );
                    });
                });
                ui.add_space(12.0);
                let top: Vec<_> = a.top.iter().filter(|t| fits(&t.kind, *kind)).collect();
                if top.is_empty() {
                    ui.label(RichText::new("Nothing watched in this range.").color(MUTED));
                }
                let max = top.iter().map(|t| t.plays).max().unwrap_or(1).max(1);
                for t in top.iter().take(15) {
                    let mut value = format!(
                        "{} play{}   ·   {:.1} h",
                        t.plays,
                        if t.plays == 1 { "" } else { "s" },
                        t.hours
                    );
                    if let Some(u) = t.users {
                        value.push_str(&format!(
                            "   ·   {u} viewer{}",
                            if u == 1 { "" } else { "s" }
                        ));
                    }
                    bar(ui, &t.title, &value, t.plays as f32 / max as f32);
                    ui.add_space(4.0);
                }
                let rated: Vec<_> = a
                    .top_rated
                    .iter()
                    .filter(|t| fits(&t.kind, *kind))
                    .collect();
                if !rated.is_empty() {
                    ui.add_space(20.0);
                    caps(ui, "Users' ratings", TEAL);
                    ui.add_space(4.0);
                    ui.label(bold("Top rated", 22.0));
                    ui.add_space(12.0);
                    for t in rated.iter().take(15) {
                        let value = format!(
                            "{:.1} / 10   ·   {} rating{}",
                            t.avg_rating,
                            t.rating_count,
                            if t.rating_count == 1 { "" } else { "s" }
                        );
                        bar(ui, &t.title, &value, t.avg_rating / 10.0);
                        ui.add_space(4.0);
                    }
                }
            },
            |ui, _| {
                panel(ui, |ui| {
                    caps(ui, "By user", TEAL);
                    ui.add_space(10.0);
                    if a.users.is_empty() {
                        ui.label(RichText::new("No one watched in this range.").color(MUTED));
                    }
                    for u in &a.users {
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(vec2(28.0, 28.0), Sense::hover());
                            crate::ui::chrome::avatar(ui.painter(), r.center(), 13.0, &u.name);
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 1.0;
                                ui.label(RichText::new(&u.name).color(TEXT));
                                let mut line = format!("{} plays · {:.1} h", u.plays, u.hours);
                                if let (Some(m), Some(e)) = (u.movies, u.episodes) {
                                    line = format!("{m} movies · {e} episodes · {:.1} h", u.hours);
                                }
                                if let Some(t) = u.last_active {
                                    line.push_str(&format!(" · {}", ago(t)));
                                }
                                ui.label(mono(line, 10.0).color(MUTED));
                            });
                        });
                        ui.add_space(8.0);
                    }
                });
            },
        );
    }
}

/// One bar per day; hover for the numbers.
fn day_chart(ui: &mut Ui, days: &[DayStat]) {
    let w = ui.available_width();
    let h = 160.0;
    let (r, resp) = ui.allocate_exact_size(vec2(w, h + 22.0), Sense::hover());
    let p = ui.painter();
    let plot = Rect::from_min_max(r.min, pos2(r.right(), r.top() + h));
    if days.is_empty() {
        p.text(
            plot.center(),
            Align2::CENTER_CENTER,
            "No plays yet",
            FontId::monospace(11.0),
            MUTED,
        );
        return;
    }
    let max = days.iter().map(|d| d.plays).max().unwrap_or(0).max(1);
    for f in [0.5, 1.0] {
        let y = plot.bottom() - plot.height() * f;
        p.hline(plot.x_range(), y, Stroke::new(1.0f32, LINE));
    }
    p.hline(plot.x_range(), plot.bottom(), Stroke::new(1.0f32, LINE));
    p.text(
        pos2(plot.left(), plot.top()),
        Align2::LEFT_BOTTOM,
        max.to_string(),
        FontId::monospace(9.5),
        MUTED,
    );
    let slot = plot.width() / days.len() as f32;
    let bw = (slot * 0.7).clamp(1.0, 24.0);
    let hover = resp.hover_pos();
    let mut tip = None;
    for (i, d) in days.iter().enumerate() {
        let cx = plot.left() + slot * (i as f32 + 0.5);
        let bh = plot.height() * d.plays as f32 / max as f32;
        let bar = Rect::from_min_max(
            pos2(cx - bw / 2.0, plot.bottom() - bh),
            pos2(cx + bw / 2.0, plot.bottom()),
        );
        let col_rect = Rect::from_min_max(
            pos2(cx - slot / 2.0, plot.top()),
            pos2(cx + slot / 2.0, plot.bottom()),
        );
        let hot = hover.is_some_and(|h| col_rect.contains(h));
        if hot {
            p.rect_filled(col_rect, 2.0, Color32::from_white_alpha(8));
            tip = Some(d);
        }
        if d.plays > 0 {
            p.rect_filled(bar, 2.0, if hot { TEXT } else { GOLD });
        }
    }
    // first and last day under the axis
    for (d, align, x) in [
        (days.first(), Align2::LEFT_TOP, plot.left()),
        (days.last(), Align2::RIGHT_TOP, plot.right()),
    ] {
        if let Some(d) = d {
            p.text(
                pos2(x, plot.bottom() + 6.0),
                align,
                &d.day,
                FontId::monospace(9.5),
                MUTED,
            );
        }
    }
    if let Some(d) = tip {
        resp.on_hover_text(format!(
            "{}\n{} plays, {:.1} hours",
            d.day, d.plays, d.hours
        ));
    }
}
