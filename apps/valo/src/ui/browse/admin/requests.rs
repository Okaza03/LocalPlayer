//! Admin › Requests: titles people asked for, usually through a Discord
//! bot that posts them to the server. Answering here changes the request's
//! status; the server (or its bot) tells the requester.
use eframe::egui::{self, vec2, Align, Layout, RichText, Sense, Stroke, Ui};

use super::super::details::{caps, padded, panel, TEAL};
use super::super::people::{ago, columns_with};
use super::users::danger_button;
use super::{not_set_up, Cx, Done, Remote, Section};
use crate::api::{Client, Library, MediaRequest};
use crate::ui::{
    bold, image_cover, menu_item, mono, placeholder, segmented, small_button, text_field,
};
use crate::ui::{ERROR, LINE, MUTED, OK, TEXT, WARN};

/// How often the queue is checked for new requests, in seconds.
const EVERY: u64 = 30;

#[derive(Clone, Copy, PartialEq, Default)]
enum Filter {
    #[default]
    Pending,
    Approved,
    Available,
    Denied,
    All,
}

impl Filter {
    fn matches(self, status: &str) -> bool {
        match self {
            Filter::All => true,
            Filter::Pending => status == "pending" || status.is_empty(),
            Filter::Approved => status == "approved",
            Filter::Available => status == "available",
            Filter::Denied => status == "denied",
        }
    }
}

#[derive(Default)]
pub(in super::super) struct State {
    pub list: Remote<Vec<MediaRequest>>,
    filter: Filter,
    /// Denying this request, with this reason.
    deny: Option<(String, String)>,
    confirm_delete: Option<String>,
}

impl State {
    pub fn refresh(&mut self) {
        self.list.retry();
    }

    pub fn missing(&self) -> bool {
        self.list.missing
    }

    /// Requests waiting for an answer, for the sidebar.
    pub fn pending(&self) -> usize {
        self.list.data.as_ref().map_or(0, |l| {
            l.iter()
                .filter(|r| Filter::Pending.matches(&r.status))
                .count()
        })
    }

    /// Keep the queue fresh, whichever section is open (the sidebar counts it).
    pub fn background(&mut self, ctx: &egui::Context, client: &Client, login: &mut bool) {
        self.list.poll(login);
        if self.list.idle()
            || (!self.list.missing && self.list.error.is_none() && self.list.stale(EVERY))
        {
            let c = client.clone();
            self.list.load(ctx, move || c.admin_requests());
        }
        if !self.list.missing {
            ctx.request_repaint_after(std::time::Duration::from_secs(EVERY));
        }
    }

    pub fn ui(&mut self, ui: &mut Ui, cx: &mut Cx) {
        if self.list.missing {
            padded(ui, |ui| {
                if not_set_up(ui, Section::Requests, &cx.client.info.name) {
                    self.list.retry();
                }
            });
            return;
        }
        let all = self.list.data.clone().unwrap_or_default();
        let count = |f: Filter| Some(all.iter().filter(|r| f.matches(&r.status)).count());
        padded(ui, |ui| {
            segmented(
                ui,
                &mut self.filter,
                &[
                    (Filter::Pending, "Pending", count(Filter::Pending)),
                    (Filter::Approved, "Approved", count(Filter::Approved)),
                    (Filter::Available, "Available", count(Filter::Available)),
                    (Filter::Denied, "Denied", count(Filter::Denied)),
                    (Filter::All, "All", count(Filter::All)),
                ],
            );
            ui.add_space(18.0);
            if let Some(e) = &self.list.error {
                ui.label(RichText::new(format!("Couldn't load the requests: {e}")).color(ERROR()));
            }
        });

        let mut shown: Vec<&MediaRequest> = all
            .iter()
            .filter(|r| self.filter.matches(&r.status))
            .collect();
        shown.sort_by_key(|r| std::cmp::Reverse(r.created));
        let mut job: Option<(String, &'static str, Option<String>)> = None;
        let mut delete = None;
        columns_with(
            ui,
            &mut (&mut *self, &mut *cx, &mut job, &mut delete),
            |ui, (st, cx, job, delete)| {
                if shown.is_empty() && st.list.data.is_some() {
                    ui.add_space(20.0);
                    ui.label(mono("No requests here", 12.0).color(MUTED()));
                }
                if st.list.data.is_none() && st.list.loading() {
                    ui.add(egui::Spinner::new().size(20.0).color(MUTED()));
                }
                for r in shown {
                    if let Some(a) = st.row(ui, r, cx) {
                        match a {
                            Answer::Status(s, note) => **job = Some((r.id.clone(), s, note)),
                            Answer::Delete => **delete = Some(r.id.clone()),
                        }
                    }
                    ui.add_space(8.0);
                }
            },
            |ui, _| {
                panel(ui, |ui| {
                    caps(ui, "How requests work", TEAL());
                    ui.add_space(8.0);
                    for line in [
                        "People ask for a movie or series in Discord, through the server's bot.",
                        "Approve it when you'll get it, then mark it available once it's in \
                         the library. Deny it with a reason if you won't.",
                        "Each answer goes back to the requester on Discord.",
                    ] {
                        ui.label(RichText::new(line).color(MUTED()));
                        ui.add_space(6.0);
                    }
                    if crate::ui::ext_link(ui, "Bot setup guide") {
                        crate::ui::chrome::open_url(Section::Requests.docs());
                    }
                });
            },
        );

        if let Some((id, status, note)) = job {
            let title = all
                .iter()
                .find(|r| r.id == id)
                .map(|r| r.title.clone())
                .unwrap_or_default();
            cx.run(move |c| {
                c.admin_update_request(&id, status, note.as_deref())?;
                let what = match status {
                    "approved" => "Approved",
                    "available" => "Marked available:",
                    "denied" => "Denied",
                    _ => "Updated",
                };
                Ok(Done::msg(format!("{what} {title}.")))
            });
        }
        if let Some(id) = delete {
            let title = all
                .iter()
                .find(|r| r.id == id)
                .map(|r| r.title.clone())
                .unwrap_or_default();
            cx.run(move |c| {
                c.admin_delete_request(&id)?;
                Ok(Done::msg(format!("Removed the request for {title}.")))
            });
        }
    }

    fn row(&mut self, ui: &mut Ui, r: &MediaRequest, cx: &mut Cx) -> Option<Answer> {
        let mut answer = None;
        let busy = cx.busy;
        let have = in_library(cx.lib, r);
        egui::Frame::new()
            .stroke(Stroke::new(1.0f32, LINE()))
            .corner_radius(8)
            .inner_margin(egui::Margin::symmetric(14, 12))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_top(|ui| {
                    let (art, _) = ui.allocate_exact_size(vec2(54.0, 81.0), Sense::hover());
                    match r.poster.as_ref().and_then(|u| cx.images.get(u)) {
                        Some(t) => image_cover(ui.painter(), art, &t, 4, egui::Color32::WHITE),
                        None => placeholder(ui.painter(), art, &r.title, 4),
                    }
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 3.0;
                        ui.horizontal(|ui| {
                            let year = r.year.map(|y| format!(" ({y})")).unwrap_or_default();
                            ui.label(bold(format!("{}{year}", r.title), 16.0));
                            if let Some(k) = &r.kind {
                                ui.label(mono(k, 9.5).color(MUTED()));
                            }
                        });
                        let mut who = Vec::new();
                        if let Some(b) = &r.requested_by {
                            who.push(format!("by {b}"));
                        }
                        if let Some(s) = &r.source {
                            who.push(format!("via {}", capitalize(s)));
                        }
                        if let Some(t) = r.created {
                            who.push(ago(t));
                        }
                        if !who.is_empty() {
                            ui.label(mono(who.join("   ·   "), 10.0).color(MUTED()));
                        }
                        let (col, label) = match r.status.as_str() {
                            "approved" => (TEAL(), "Approved"),
                            "available" => (OK(), "Available"),
                            "denied" => (ERROR(), "Denied"),
                            _ => (WARN(), "Pending"),
                        };
                        ui.horizontal(|ui| {
                            ui.label(mono(label, 10.0).color(col));
                            if have && r.status != "available" {
                                ui.label(mono("already in the library", 10.0).color(OK()));
                            }
                            if r.notified == Some(false) {
                                ui.label(mono("requester wasn't told", 10.0).color(ERROR()))
                                    .on_hover_text(
                                        "The bot couldn't message them, for example because \
                                         their Discord DMs are closed.",
                                    );
                            }
                        });
                        if let Some(n) = r.note.as_ref().filter(|n| !n.trim().is_empty()) {
                            ui.label(RichText::new(format!("“{n}”")).italics().color(TEXT()));
                        }
                    });
                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                        ui.add_enabled_ui(!busy, |ui| {
                            let more = crate::ui::small_icon_button(
                                ui,
                                crate::ui::chrome::Icon::More,
                                "More",
                            );
                            let id = ui.make_persistent_id(("request-menu", &r.id));
                            if more.clicked() {
                                ui.memory_mut(|m| m.toggle_popup(id));
                            }
                            egui::popup::popup_below_widget(
                                ui,
                                id,
                                &more,
                                egui::PopupCloseBehavior::CloseOnClickOutside,
                                |ui| {
                                    ui.set_min_width(200.0);
                                    if r.status != "pending" && menu_item(ui, "Back to pending") {
                                        answer = Some(Answer::Status("pending", None));
                                        ui.memory_mut(|m| m.close_popup());
                                    }
                                    if menu_item(ui, "Remove request…") {
                                        self.confirm_delete = Some(r.id.clone());
                                        ui.memory_mut(|m| m.close_popup());
                                    }
                                },
                            );
                            if r.status != "denied"
                                && r.status != "available"
                                && small_button(ui, "Deny…").clicked()
                            {
                                self.deny = Some((r.id.clone(), String::new()));
                            }
                            if r.status != "available"
                                && (have || r.status == "approved")
                                && small_button(ui, "Mark available").clicked()
                            {
                                answer = Some(Answer::Status("available", None));
                            }
                            if Filter::Pending.matches(&r.status)
                                && small_button(ui, "Approve").clicked()
                            {
                                answer = Some(Answer::Status("approved", None));
                            }
                        });
                    });
                });

                if let Some((_, reason)) = self.deny.as_mut().filter(|(id, _)| *id == r.id) {
                    ui.add_space(10.0);
                    ui.label(mono("Reason (sent to the requester)", 11.0).color(MUTED()));
                    text_field(ui, reason, "e.g. not available in good quality yet", false);
                    ui.add_space(6.0);
                    let mut close = false;
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled_ui(!busy, |ui| danger_button(ui, "Deny request"))
                            .inner
                            .clicked()
                        {
                            answer = Some(Answer::Status("denied", Some(reason.clone())));
                            close = true;
                        }
                        if small_button(ui, "Cancel").clicked() {
                            close = true;
                        }
                    });
                    if close {
                        self.deny = None;
                    }
                }
                if self.confirm_delete.as_deref() == Some(r.id.as_str()) {
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(format!(
                            "Remove the request for {}? The requester isn't told.",
                            r.title
                        ))
                        .color(TEXT()),
                    );
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled_ui(!busy, |ui| danger_button(ui, "Remove"))
                            .inner
                            .clicked()
                        {
                            answer = Some(Answer::Delete);
                            self.confirm_delete = None;
                        }
                        if small_button(ui, "Cancel").clicked() {
                            self.confirm_delete = None;
                        }
                    });
                }
            });
        answer
    }
}

enum Answer {
    Status(&'static str, Option<String>),
    Delete,
}

/// The library already has it (same IMDb id, or same title and year).
fn in_library(lib: &Library, r: &MediaRequest) -> bool {
    let title = r.title.to_lowercase();
    lib.all().any(|i| match (&i.imdb_id, &r.imdb_id) {
        (Some(a), Some(b)) => a == b,
        _ => i.title.to_lowercase() == title && (r.year.is_none() || i.year == r.year),
    })
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map_or(String::new(), |f| f.to_uppercase().chain(c).collect())
}
