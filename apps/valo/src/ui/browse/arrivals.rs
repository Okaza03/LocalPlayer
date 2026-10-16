//! "New on the server": movies and series added since the user last saw
//! the library, announced on the browse pages (never over the player).
use std::collections::HashSet;

use eframe::egui::{self, RichText, Ui};

use super::details::poster_tile;
use super::{Browse, Click, View, GAP};
use crate::api::{Kind, Library};
use crate::images::Images;
use crate::ui::settings::{dialog, header};
use crate::ui::{primary_button, MUTED};

/// At most this many posters; the rest are counted.
const SHOWN: usize = 12;

fn ids(lib: &Library) -> HashSet<String> {
    lib.all().map(|i| i.id.clone()).collect()
}

/// "2 new movies and 1 new series"
fn summary(movies: usize, series: usize) -> String {
    let part = |n: usize, one: &str, many: &str| match n {
        0 => None,
        1 => Some(format!("1 new {one}")),
        n => Some(format!("{n} new {many}")),
    };
    [
        part(movies, "movie", "movies"),
        part(series, "series", "series"),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" and ")
}

impl Browse {
    /// A library (re)loaded: queue what's new since last time. The first
    /// load of a library only remembers it.
    pub(super) fn note_arrivals(&mut self, lib: &Library) {
        let now = ids(lib);
        let prefix = self.prefix();
        match self.known.get(&prefix) {
            None => self.known.set(&prefix, now),
            Some(old) => {
                let fresh: Vec<String> = lib
                    .all()
                    .filter(|i| !old.contains(&i.id) && !self.arrivals.contains(&i.id))
                    .map(|i| i.id.clone())
                    .collect();
                self.arrivals.extend(fresh);
                self.arrivals.retain(|id| now.contains(id));
            }
        }
    }

    /// Show the queued arrivals once the user is on Home, Movies or TV Shows.
    pub(super) fn arrivals_ui(
        &mut self,
        ctx: &egui::Context,
        lib: &Library,
        images: &mut Images,
        allowed: bool,
    ) -> Option<Click> {
        let browsing =
            matches!(self.view, View::Home | View::Movies | View::Series) && self.search.is_empty();
        if self.arrivals.is_empty() || !allowed || !browsing {
            return None;
        }
        if !self.arrivals_open {
            // seen now, even if the app closes with the dialog up
            self.arrivals_open = true;
            let prefix = self.prefix();
            self.known.set(&prefix, ids(lib));
        }
        let items: Vec<_> = self.arrivals.iter().filter_map(|id| lib.find(id)).collect();
        let movies = items.iter().filter(|i| i.kind == Kind::Movie).count();
        let mut open = None;
        let (done, close) = dialog(ctx, "new-arrivals", 640.0, |ui: &mut Ui| {
            header(
                ui,
                &format!("New on {}", self.client.info.name),
                &summary(movies, items.len() - movies),
            );
            ui.add_space(8.0);
            let w = ui.available_width();
            let cols = ((w + GAP) / (140.0 + GAP)).floor().clamp(1.0, 4.0);
            let tw = (w - GAP * (cols - 1.0)) / cols;
            for row in items
                .iter()
                .take(SHOWN)
                .collect::<Vec<_>>()
                .chunks(cols as usize)
            {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = GAP;
                    for item in row {
                        let sub = match (item.kind, item.year) {
                            (Kind::Series, Some(y)) => format!("Series · {y}"),
                            (Kind::Series, None) => "Series".to_owned(),
                            (_, y) => y.map(|y| y.to_string()).unwrap_or_default(),
                        };
                        if poster_tile(ui, item, Some(&sub), tw, images) {
                            open = Some(item.id.clone());
                        }
                    }
                });
                ui.add_space(12.0);
            }
            if items.len() > SHOWN {
                let more = items.len() - SHOWN;
                ui.label(RichText::new(format!("And {more} more.")).color(MUTED()));
                ui.add_space(12.0);
            }
            let b = ui.add_sized([ui.available_width(), 40.0], |ui: &mut Ui| {
                primary_button(ui, "Close", true)
            });
            b.clicked()
        });
        if done || close || open.is_some() || items.is_empty() {
            self.arrivals.clear();
            self.arrivals_open = false;
        }
        open.map(Click::Open)
    }
}

#[cfg(test)]
mod tests {
    use super::summary;

    #[test]
    fn summaries() {
        assert_eq!(summary(2, 1), "2 new movies and 1 new series");
        assert_eq!(summary(1, 0), "1 new movie");
        assert_eq!(summary(0, 3), "3 new series");
    }
}
