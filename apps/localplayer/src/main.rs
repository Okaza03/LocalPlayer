mod api;
mod audio;
mod config;
mod images;
mod integrate;
mod player;
mod ui;
mod update;
mod util;

use std::path::PathBuf;

use eframe::egui;

use api::{Client, Mode};
use audio::AudioOut;
use config::{Config, Progress};
use images::Images;
use ui::browse::Browse;
use ui::chrome::{self, Icon};
use ui::login::Login;
use ui::settings::{self as dialogs, Conn, SettingsState};
use ui::setup::Setup;
use ui::watch::{PlayItem, Watch};
use ui::Action;
use update::Updates;

fn main() -> eframe::Result<()> {
    let paths: Vec<PathBuf> = std::env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .filter(|p| p.is_file())
        .collect();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 780.0])
            .with_min_inner_size([560.0, 400.0])
            .with_title("LocalPlayer")
            .with_decorations(false) // we draw our own title bar
            .with_app_id("localplayer"), // matches localplayer.desktop
        ..Default::default()
    };
    eframe::run_native(
        "LocalPlayer",
        options,
        Box::new(move |cc| {
            ui::apply_theme(&cc.egui_ctx);
            Ok(Box::new(App::new(&cc.egui_ctx, paths)))
        }),
    )
}

pub fn pick_files() -> Option<Vec<PathBuf>> {
    let start = std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Videos"));
    let mut dlg = rfd::FileDialog::new()
        .add_filter("Video", api::VIDEO_EXTS)
        .add_filter("All files", &["*"]);
    if let Some(s) = start.filter(|s| s.is_dir()) {
        dlg = dlg.set_directory(s);
    }
    dlg.pick_files()
}

enum Stage {
    Setup(Setup),
    Login(Login),
    Browse(Box<Browse>),
}

struct App {
    cfg: Config,
    progress: Progress,
    audio: AudioOut,
    images: Images,
    muted: bool,
    stage: Stage,
    watch: Option<Watch>,
    dialog: Option<Dialog>,
    settings: SettingsState,
    updates: Updates,
}

#[derive(Clone, Copy, PartialEq)]
enum Dialog {
    Settings,
    Profile,
}

impl App {
    fn new(ctx: &egui::Context, files: Vec<PathBuf>) -> Self {
        let cfg = Config::load();
        integrate::sync(cfg.app_menu);
        // reconnect to the saved server; files on the command line play first
        let setup = Setup::new(ctx, cfg.server.clone(), files.is_empty());
        let watch = (!files.is_empty())
            .then(|| Watch::new(files.into_iter().map(PlayItem::local).collect()));
        Self {
            progress: Progress::load(),
            audio: AudioOut::new(),
            images: Images::new(ctx),
            muted: false,
            stage: Stage::Setup(setup),
            watch,
            cfg,
            dialog: None,
            settings: SettingsState::default(),
            updates: {
                let mut u = Updates::default();
                u.check(ctx, false); // quietly, once per start
                u
            },
        }
    }

    fn browse(&mut self, ctx: &egui::Context, client: Client, user: Option<String>) {
        self.images.set_client(Some(client.clone()));
        self.stage = Stage::Browse(Box::new(Browse::new(ctx, client, user)));
    }

    fn conn(&self) -> Option<Conn> {
        let conn = |c: &Client, user: Option<&str>| Conn {
            name: c.info.name.clone(),
            base: c.base.clone(),
            user: user.map(str::to_owned),
            signed_in: c.token.is_some(),
        };
        match &self.stage {
            Stage::Browse(b) => Some(conn(b.client(), b.user())),
            Stage::Login(l) => Some(conn(l.client(), None)),
            Stage::Setup(_) => None,
        }
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        let Some(d) = self.dialog else { return };
        let conn = self.conn();
        let out = match (d, &conn) {
            (Dialog::Settings, _) => dialogs::settings(
                ctx,
                &mut self.settings,
                &mut self.cfg,
                &mut self.progress,
                conn.as_ref(),
            ),
            (Dialog::Profile, Some(c)) => dialogs::profile(ctx, c, &self.progress),
            (Dialog::Profile, None) => dialogs::Pick::Close,
        };
        match out {
            dialogs::Pick::Keep => {}
            dialogs::Pick::Close => self.dialog = None,
            dialogs::Pick::OpenSettings => self.dialog = Some(Dialog::Settings),
            dialogs::Pick::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            dialogs::Pick::ChangeServer => {
                self.dialog = None;
                self.handle(ctx, Action::ChangeServer);
            }
            dialogs::Pick::SignOut => {
                self.dialog = None;
                if let Stage::Browse(b) = &self.stage {
                    let c = b.client().clone();
                    self.handle(ctx, Action::SignOut(c));
                }
            }
        }
    }

    fn handle(&mut self, ctx: &egui::Context, action: Action) {
        match action {
            Action::None | Action::Refresh => {}
            Action::OpenSettings => self.dialog = Some(Dialog::Settings),
            Action::CheckUpdates => self.updates.check(ctx, true),
            Action::OpenProfile => self.dialog = Some(Dialog::Profile),
            Action::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            Action::Connected(mut client) => {
                if let Stage::Setup(s) = &self.stage {
                    if s.address().trim() != self.cfg.server {
                        self.cfg.server = s.address().trim().to_owned();
                        self.cfg.token = None; // belonged to the old server
                    }
                }
                self.cfg.save();
                if !client.info.auth {
                    self.browse(ctx, client, None);
                } else if let Some(t) = self.cfg.token.clone() {
                    client.token = Some(t);
                    let user = Some(self.cfg.username.clone()).filter(|u| !u.is_empty());
                    self.browse(ctx, client, user);
                } else {
                    self.stage = Stage::Login(Login::new(client, self.cfg.username.clone(), None));
                }
            }
            Action::SignedIn {
                client,
                user,
                remember,
            } => {
                self.cfg.username = user.clone();
                self.cfg.token = if remember { client.token.clone() } else { None };
                self.cfg.save();
                self.browse(ctx, client, Some(user));
            }
            Action::NeedLogin(mut client) => {
                client.token = None;
                self.cfg.token = None;
                self.cfg.save();
                self.images.set_client(None);
                let msg = (client.info.mode == Mode::Api)
                    .then(|| "Your session has expired. Please sign in again.".to_owned());
                self.stage = Stage::Login(Login::new(client, self.cfg.username.clone(), msg));
            }
            Action::SignOut(mut client) => {
                let c = client.clone();
                std::thread::spawn(move || c.logout());
                client.token = None;
                self.cfg.token = None;
                self.cfg.save();
                self.images.set_client(None);
                self.stage = Stage::Login(Login::new(client, self.cfg.username.clone(), None));
            }
            Action::ChangeServer => {
                self.images.set_client(None);
                self.stage = Stage::Setup(Setup::new(ctx, self.cfg.server.clone(), false));
            }
            Action::OpenFiles => {
                if let Some(files) = pick_files() {
                    self.watch = Some(Watch::new(files.into_iter().map(PlayItem::local).collect()));
                }
            }
            Action::Play(items) => {
                if self.cfg.fullscreen_on_play {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
                }
                self.watch = Some(Watch::new(items));
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.images.poll(ctx);
        if (ctx.zoom_factor() - self.cfg.ui_scale).abs() > 0.001 {
            ctx.set_zoom_factor(self.cfg.ui_scale.clamp(0.5, 2.0));
        }

        if let Some(w) = self.watch.as_mut() {
            if w.ui(
                ctx,
                &self.audio,
                &mut self.cfg,
                &mut self.muted,
                &mut self.progress,
            ) {
                self.watch = None;
                self.cfg.save();
                ctx.send_viewport_cmd(egui::ViewportCommand::Title("LocalPlayer".into()));
            }
            chrome::window_frame(ctx);
            return;
        }

        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        if !dropped.is_empty() {
            self.watch = Some(Watch::new(
                dropped.into_iter().map(PlayItem::local).collect(),
            ));
        }

        if !matches!(self.stage, Stage::Browse(_)) {
            let (mut open_settings, mut check) = (false, false);
            let name = match &self.stage {
                Stage::Login(l) => l.client().info.name.clone(),
                _ => "LocalPlayer".to_owned(),
            };
            chrome::title_bar(
                ctx,
                &name,
                |_| {},
                |ui| {
                    open_settings = chrome::icon_button(ui, Icon::Settings, "Settings").clicked();
                    chrome::vsep(ui);
                    check = chrome::update_button(ui, &self.updates);
                },
            );
            if open_settings {
                self.dialog = Some(Dialog::Settings);
            }
            if check {
                self.updates.check(ctx, true);
            }
            let left = match &self.stage {
                Stage::Login(l) => {
                    format!("Connected to {}  ·  sign-in required", l.client().info.name)
                }
                _ => "Not connected".to_owned(),
            };
            let right = format!(
                "LocalPlayer {}  ·  {} {}",
                env!("CARGO_PKG_VERSION"),
                std::env::consts::OS,
                std::env::consts::ARCH
            );
            chrome::status_bar(ctx, &left, &right);
        }

        let action = match &mut self.stage {
            Stage::Setup(s) => s.ui(ctx),
            Stage::Login(l) => l.ui(ctx),
            Stage::Browse(b) => b.ui(
                ctx,
                &mut self.images,
                &self.progress,
                &self.cfg,
                &self.updates,
            ),
        };
        self.handle(ctx, action);
        self.updates.ui(ctx);
        self.dialogs(ctx);
        chrome::window_frame(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(w) = self.watch.as_mut() {
            w.save_progress(&mut self.progress);
        }
        self.watch = None;
        self.progress.flush();
        self.cfg.save();
    }
}
