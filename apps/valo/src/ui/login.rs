//! Sign-in, shown when the server says it needs one.
use anyhow::Result;
use eframe::egui::{self, RichText};

use super::{card, primary_button, spinner_text, text_field, Action, ERROR, MUTED};
use crate::api::{Client, Missing, Session};
use crate::util::Task;

pub struct Login {
    client: Client,
    username: String,
    password: String,
    remember: bool,
    task: Option<Task<Result<Session>>>,
    error: Option<String>,
    missing: Option<Missing>,
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
            missing: None,
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
        self.missing = None;
    }

    pub fn ui(&mut self, ctx: &egui::Context) -> Action {
        if let Some(r) = self.task.as_mut().and_then(|t| t.poll()) {
            self.task = None;
            match r {
                Ok(s) => {
                    let mut client = self.client.clone();
                    client.token = Some(s.token);
                    client.admin = s.admin;
                    let user = s.name.unwrap_or_else(|| self.username.trim().to_owned());
                    return Action::SignedIn {
                        client,
                        user,
                        remember: self.remember,
                    };
                }
                Err(e) => {
                    self.missing = e.downcast_ref::<Missing>().copied();
                    self.error = Some(format!("{e:#}"));
                    self.password.clear();
                }
            }
        }
        let busy = self.task.is_some();
        card(ctx, |ui| {
            let mut action = Action::None;
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
            // Enter ends a field (it loses focus); read that before anything takes focus again
            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
            let user_enter = u.lost_focus() && enter;
            let pass_enter = p.lost_focus() && enter;
            // Enter in the username: on to the password, or sign in when it's filled in
            let go = pass_enter || (user_enter && !self.password.is_empty());
            if user_enter && !go {
                p.request_focus();
            } else if !go && !busy && ui.memory(|m| m.focused().is_none()) {
                if self.username.is_empty() {
                    u.request_focus()
                } else {
                    p.request_focus()
                }
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
                if ok && (b.clicked() || go) {
                    self.submit(ui.ctx());
                }
            }
            if let Some(e) = &self.error {
                ui.add_space(6.0);
                ui.label(RichText::new(e).color(ERROR));
                if let Some(m) = self.missing {
                    super::admin_help(ui, m, &self.client.base);
                }
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
