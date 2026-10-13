//! First screen: where is the media server?
use anyhow::Result;
use eframe::egui::{self, RichText};

use super::{card, primary_button, spinner_text, text_field, Action, ERROR, MUTED};
use crate::api::{normalize, Client, Missing};
use crate::util::Task;

pub struct Setup {
    address: String,
    task: Option<Task<Result<Client>>>,
    error: Option<String>,
    missing: Option<Missing>,
}

impl Setup {
    /// With `auto`, connects to `address` right away (the saved server).
    pub fn new(ctx: &egui::Context, address: String, auto: bool) -> Self {
        let mut s = Self {
            address,
            task: None,
            error: None,
            missing: None,
        };
        if auto && !s.address.trim().is_empty() {
            s.connect(ctx);
        }
        s
    }

    fn connect(&mut self, ctx: &egui::Context) {
        let a = self.address.clone();
        self.task = Some(Task::spawn(ctx, move || Client::connect(&a)));
        self.error = None;
        self.missing = None;
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    pub fn ui(&mut self, ctx: &egui::Context) -> Action {
        if let Some(r) = self.task.as_mut().and_then(|t| t.poll()) {
            self.task = None;
            match r {
                Ok(c) => return Action::Connected(c),
                Err(e) => {
                    self.missing = e.downcast_ref::<Missing>().copied();
                    self.error = Some(format!("{e:#}"));
                }
            }
        }
        let busy = self.task.is_some();
        card(ctx, |ui| {
            let mut action = Action::None;
            ui.label(super::bold("Connect to your media library", 20.0));
            ui.label(
                RichText::new("Enter the IP address (and port) of the computer on your network that serves your movies and series.")
                    .color(MUTED),
            );
            ui.add_space(14.0);
            ui.label(super::mono("Server address", 11.0).color(MUTED));
            let r = ui
                .add_enabled_ui(!busy, |ui| {
                    text_field(ui, &mut self.address, "192.168.1.20:8080", false)
                })
                .inner;
            if !busy
                && self.address.is_empty()
                && !r.has_focus()
                && ui.memory(|m| m.focused().is_none())
            {
                r.request_focus();
            }
            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            ui.label(
                RichText::new(
                    "http:// is assumed. Include a path if the library lives in a subfolder.",
                )
                .small()
                .color(MUTED),
            );
            ui.add_space(12.0);
            if busy {
                ui.horizontal(|ui| {
                    spinner_text(ui, &format!("Connecting to {}…", self.address.trim()));
                    if ui.link("Cancel").clicked() {
                        self.task = None;
                    }
                });
            } else {
                let ok = !self.address.trim().is_empty();
                let b = ui.add_sized([ui.available_width(), 42.0], |ui: &mut egui::Ui| {
                    primary_button(ui, "Connect", ok)
                });
                if ok && (b.clicked() || enter) {
                    self.connect(ui.ctx());
                }
            }
            if let Some(e) = &self.error {
                ui.add_space(6.0);
                ui.label(RichText::new(e).color(ERROR));
                if let Some(m) = self.missing {
                    super::admin_help(ui, m, &normalize(&self.address));
                }
            }
            ui.add_space(16.0);
            ui.separator();
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Just want to watch a file?").color(MUTED));
                if super::small_button(ui, "Open file").clicked() {
                    action = Action::OpenFiles;
                }
            });
            action
        })
    }
}
