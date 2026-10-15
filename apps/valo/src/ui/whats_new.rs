//! "What's new", shown once after the app updates itself (or is updated).
//! Only the big things go in [`NOTES`]; the full list is in the release notes.
use eframe::egui::{self, RichText};

use super::settings::{dialog, header};
use super::{bold, primary_button, MUTED};
use crate::update::{newer, VERSION};

struct Note {
    title: &'static str,
    text: &'static str,
    /// Only admins see it.
    admin: bool,
}

/// Newest last. Add an entry when a release brings something users would
/// want to know about; leave small fixes out.
const NOTES: &[(&str, &[Note])] = &[(
    "0.7.0",
    &[
        Note {
            title: "New arrivals",
            text: "When movies or series are added to your server, Valo shows them \
                   to you when you open the app or the library refreshes.",
            admin: false,
        },
        Note {
            title: "Refresh any time",
            text: "Click \"Library updated …\" in the bottom-left corner to reload \
                   the library without restarting.",
            admin: false,
        },
        Note {
            title: "Your devices",
            text: "Your profile lists every device you're signed in on, and you can \
                   sign any of them out.",
            admin: false,
        },
        Note {
            title: "Collections",
            text: "Film series like Batman stay together in A to Z, in the order your \
                   server's admin picked.",
            admin: false,
        },
        Note {
            title: "Admin page",
            text: "Admin in the profile menu: a dashboard with syncs and the live log, \
                   analytics, Discord requests, editing titles with IMDb, collections \
                   and users. Sections your server doesn't support link to their setup guide.",
            admin: true,
        },
    ],
)];

/// The notes for versions after `seen`, up to `current`.
fn between(seen: &str, current: &str, admin: bool) -> Vec<&'static Note> {
    NOTES
        .iter()
        .filter(|(v, _)| newer(v, seen) && !newer(v, current))
        .flat_map(|(_, notes)| notes.iter())
        .filter(|n| admin || !n.admin)
        .collect()
}

/// Whether there's anything to show for a user who last saw `seen`
/// (empty: a config from before this existed).
pub fn pending(seen: &str, admin: bool) -> bool {
    seen != VERSION && !between(seen, VERSION, admin).is_empty()
}

/// Draw the dialog; true once it's dismissed.
pub fn ui(ctx: &egui::Context, seen: &str, admin: bool) -> bool {
    let notes = between(seen, VERSION, admin);
    let (done, close) = dialog(ctx, "whats-new", 460.0, |ui| {
        header(ui, "What's new", &format!("Valo {VERSION}"));
        ui.add_space(6.0);
        for n in &notes {
            ui.label(bold(n.title, 16.0));
            ui.add_space(2.0);
            ui.label(RichText::new(n.text).size(14.5).color(MUTED));
            ui.add_space(14.0);
        }
        let b = ui.add_sized([ui.available_width(), 40.0], |ui: &mut egui::Ui| {
            primary_button(ui, "Got it", true)
        });
        b.clicked()
    });
    done || close
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_are_ordered_and_filtered() {
        assert!(NOTES.windows(2).all(|w| newer(w[1].0, w[0].0)));
        // admin-only notes stay hidden from everyone else
        let all = between("0.6.1", "0.7.0", true);
        let users = between("0.6.1", "0.7.0", false);
        assert!(users.iter().all(|n| !n.admin));
        assert!(users.len() < all.len());
        // nothing before its release, nothing twice
        assert!(between("0.6.1", "0.6.1", true).is_empty());
        assert!(between("0.7.0", "0.7.0", true).is_empty());
        assert_eq!(between("", "0.7.0", true).len(), all.len());
    }
}
