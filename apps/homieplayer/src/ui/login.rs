//! Sign-in, shown when the server says it needs one.
use anyhow::Result;
use eframe::egui::{self, RichText};

use super::{card, primary_button, spinner_text, text_field, Action, ERROR, MUTED};
use crate::api::{Client, Session};
use crate::util::Task;

pub struct Login {
    client: Client,
    username: String,
    password: String,
    remember: bool,
    task: Option<Task<Result<Session>>>,
    error: Option<String>,
}

impl Login {
    pub fn new(client: Client, username: String, error: Option<String>) -> Self {
        Self {
            client,
            username,
            password: String::new(),
            remember: true,
            task: None,
            error,
        }
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    fn submit(&mut self, ctx: &egui::Context) {
        let (c, u, p) = (
            self.client.clone(),
            self.username.trim().to_owned(),
            self.password.clone(),
        );
        self.task = Some(Task::spawn(ctx, move || c.login(&u, &p)));
        self.error = None;
    }

    pub fn ui(&mut self, ctx: &egui::Context) -> Action {
        if let Some(r) = self.task.as_mut().and_then(|t| t.poll()) {
            self.task = None;
            match r {
                Ok(s) => {
                    let mut client = self.client.clone();
                    client.token = Some(s.token);
                    let user = s.name.unwrap_or_else(|| self.username.trim().to_owned());
                    return Action::SignedIn {
                        client,
                        user,
                        remember: self.remember,
                    };
                }
                Err(e) => {
                    self.error = Some(format!("{e:#}"));
                    self.password.clear();
                }
            }
        }
        let busy = self.task.is_some();
        card(ctx, |ui| {
            let mut action = Action::None;
            super::logo(ui, 30.0);
            ui.add_space(6.0);
            ui.label(super::bold("Sign in", 22.0));
            ui.label(RichText::new(format!("to {}", self.client.info.name)).color(MUTED));
            ui.add_space(14.0);

            ui.label(super::mono("Username", 11.0).color(MUTED));
            let u = ui
                .add_enabled_ui(!busy, |ui| text_field(ui, &mut self.username, "", false))
                .inner;
            ui.add_space(4.0);
            ui.label(super::mono("Password", 11.0).color(MUTED));
            let p = ui
                .add_enabled_ui(!busy, |ui| text_field(ui, &mut self.password, "", true))
                .inner;
            if ui.memory(|m| m.focused().is_none()) && !busy {
                if self.username.is_empty() {
                    u.request_focus()
                } else {
                    p.request_focus()
                }
            }
            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
            if u.lost_focus() && enter {
                p.request_focus();
            }
            ui.add_space(4.0);
            ui.checkbox(&mut self.remember, "Keep me signed in");
            ui.add_space(10.0);

            if busy {
                spinner_text(ui, "Signing in…");
            } else {
                let ok = !self.username.trim().is_empty();
                let b = ui.add_sized([ui.available_width(), 42.0], |ui: &mut egui::Ui| {
                    primary_button(ui, "Sign in", ok)
                });
                if ok && (b.clicked() || (p.lost_focus() && enter)) {
                    self.submit(ui.ctx());
                }
            }
            if let Some(e) = &self.error {
                ui.add_space(6.0);
                ui.label(RichText::new(e).color(ERROR));
            }
            ui.add_space(14.0);
            ui.vertical_centered(|ui| {
                if ui
                    .link(format!("Not {}? Use a different server", self.client.base))
                    .clicked()
                {
                    action = Action::ChangeServer;
                }
            });
            action
        })
    }
}
