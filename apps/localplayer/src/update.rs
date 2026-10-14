//! "Is there a newer LocalPlayer?" — asks GitHub for the latest release, and
//! when running as an AppImage or from the Windows installer, installs it in
//! place and restarts.
use std::fs;
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use eframe::egui::{self, Align, Align2, Layout, RichText};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::ui::{chrome, mono, small_button, BG, LINE, MUTED, TEXT};
use crate::util::Task;

const REPO: &str = "Okaza03/LocalPlayer";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone)]
pub struct Release {
    pub version: String,
    pub url: String,
    /// (file name, download URL) of this machine's AppImage, or of the
    /// installer on Windows.
    package: Option<(String, String)>,
    /// Download URL of SHA256SUMS.txt.
    sums: Option<String>,
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    assets: Vec<GhAsset>,
}

#[derive(Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
}

/// How this copy was installed, which decides how it can be updated.
#[derive(Clone, PartialEq, Eq)]
pub enum Install {
    /// Replace the file and restart.
    AppImage(PathBuf),
    /// Installed by the Windows installer: run the new one, which replaces
    /// this copy and starts it again.
    WindowsSetup,
    /// Read-only sandbox: the user installs the new .flatpak.
    Flatpak,
    /// .deb, tarball, from source: send them to the release page.
    Other,
}

impl Install {
    pub fn detect() -> Self {
        if let Some(p) = crate::integrate::appimage() {
            Install::AppImage(p.into())
        } else if cfg!(windows) && installed_by_setup() {
            Install::WindowsSetup
        } else if std::env::var_os("FLATPAK_ID").is_some() {
            Install::Flatpak
        } else {
            Install::Other
        }
    }
}

/// The installer leaves its uninstaller next to the exe; the portable zip doesn't.
fn installed_by_setup() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|exe| Some(exe.parent()?.join("unins000.exe").is_file()))
        .unwrap_or(false)
}

/// End of the name of the release file this platform updates itself from.
fn package_suffix() -> String {
    let arch = std::env::consts::ARCH;
    if cfg!(windows) {
        format!("-{arch}-setup.exe")
    } else {
        format!("-{arch}.AppImage")
    }
}

/// "v1.2.3-beta" -> [1, 2, 3]
fn parse(v: &str) -> Vec<u64> {
    let v = v.trim().trim_start_matches(['v', 'V']);
    let core = v.split(['-', '+']).next().unwrap_or(v);
    core.split('.').map(|p| p.parse().unwrap_or(0)).collect()
}

fn newer(latest: &str, current: &str) -> bool {
    let (mut a, mut b) = (parse(latest), parse(current));
    let n = a.len().max(b.len());
    a.resize(n, 0);
    b.resize(n, 0);
    a > b
}

fn agent(timeout: Option<Duration>) -> ureq::Agent {
    let mut b = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(30))
        .user_agent(concat!("LocalPlayer/", env!("CARGO_PKG_VERSION")));
    if let Some(t) = timeout {
        b = b.timeout(t);
    }
    b.build()
}

/// The latest release if it's newer than this build, None if we're current.
fn latest() -> Result<Option<Release>> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let r: GhRelease = match agent(Some(Duration::from_secs(10)))
        .get(&url)
        .set("Accept", "application/vnd.github+json")
        .call()
    {
        Ok(r) => r.into_json()?,
        Err(ureq::Error::Status(404, _)) => return Err(anyhow!("no releases published yet")),
        Err(ureq::Error::Status(code, _)) => return Err(anyhow!("GitHub answered {code}")),
        Err(e) => return Err(anyhow!("can't reach GitHub ({e})")),
    };
    let version = r.tag_name.trim_start_matches(['v', 'V']).to_owned();
    let suffix = package_suffix();
    let asset = |f: &dyn Fn(&str) -> bool| {
        r.assets
            .iter()
            .find(|a| f(&a.name))
            .map(|a| (a.name.clone(), a.browser_download_url.clone()))
    };
    let package = asset(&|n| n.ends_with(&suffix));
    let sums = asset(&|n| n == "SHA256SUMS.txt").map(|(_, u)| u);
    Ok(newer(&version, VERSION).then_some(Release {
        version,
        url: r.html_url,
        package,
        sums,
    }))
}

/// The hash `sha256sum` printed for `name` (paths in the file may have folders).
fn expected_hash(sums: &str, name: &str) -> Option<String> {
    sums.lines().find_map(|l| {
        let (hash, path) = l.split_once(char::is_whitespace)?;
        let file = path.trim().trim_start_matches('*');
        (file.rsplit('/').next() == Some(name)).then(|| hash.to_lowercase())
    })
}

/// Download the release's package into `dir` as `name.part`, check it
/// against SHA256SUMS.txt, and rename it to `name`. `progress` is per-mille.
fn download(rel: &Release, dir: &Path, progress: &AtomicU32) -> Result<PathBuf> {
    let (Some((name, url)), Some(sums_url)) = (&rel.package, &rel.sums) else {
        bail!("this release has nothing to update this copy with");
    };
    let agent = agent(None);
    let sums = agent
        .get(sums_url)
        .call()
        .context("can't download SHA256SUMS.txt")?
        .into_string()?;
    let want = expected_hash(&sums, name)
        .ok_or_else(|| anyhow!("{name} isn't listed in SHA256SUMS.txt"))?;

    let r = agent
        .get(url)
        .call()
        .with_context(|| format!("can't download {name}"))?;
    let total: u64 = r
        .header("Content-Length")
        .and_then(|l| l.parse().ok())
        .unwrap_or(0);
    let tmp = dir.join(format!(".{name}.part"));
    let res = (|| -> Result<()> {
        let mut out =
            fs::File::create(&tmp).with_context(|| format!("can't write to {}", dir.display()))?;
        let mut hasher = Sha256::new();
        let mut reader = r.into_reader();
        let mut buf = vec![0; 64 << 10];
        let mut done = 0u64;
        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            out.write_all(&buf[..n])?;
            done += n as u64;
            if let Some(p) = (done * 1000).checked_div(total) {
                progress.store(p.min(1000) as u32, Ordering::Relaxed);
            }
        }
        out.sync_all()?;
        let got: String = hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if got != want {
            bail!("the download is corrupt (checksum mismatch), nothing was changed");
        }
        Ok(())
    })();
    if res.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    res.map(|()| tmp)
}

/// Download the new AppImage next to `dest` and swap it in.
fn install_appimage(rel: &Release, dest: &Path, progress: &AtomicU32) -> Result<()> {
    // same folder as the AppImage, so the final rename can't cross filesystems
    let tmp = download(rel, dest.parent().unwrap_or(Path::new(".")), progress)?;
    #[cfg(unix)]
    fs::set_permissions(&tmp, fs::Permissions::from_mode(0o755))?;
    // the running copy stays mounted from the old file until we exit
    let res = fs::rename(&tmp, dest).with_context(|| format!("can't replace {}", dest.display()));
    if res.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    res
}

/// Download the new installer and start it. It closes this copy if it's still
/// running, replaces the files, and starts the new version.
fn run_setup(rel: &Release, progress: &AtomicU32) -> Result<()> {
    let tmp = download(rel, &std::env::temp_dir(), progress)?;
    // Windows only runs it under its .exe name
    let exe = tmp.with_extension("");
    fs::rename(&tmp, &exe).with_context(|| format!("can't write {}", exe.display()))?;
    Command::new(&exe)
        .args([
            "/SILENT",
            "/SUPPRESSMSGBOXES",
            "/NORESTART",
            "/CLOSEAPPLICATIONS",
        ])
        .spawn()
        .with_context(|| format!("can't start {}", exe.display()))?;
    Ok(())
}

/// Start the (new) AppImage with our arguments; the caller then closes this window.
fn relaunch(appimage: &Path) -> Result<()> {
    let mut cmd = Command::new(appimage);
    cmd.args(std::env::args_os().skip(1));
    // the runtime sets these afresh for the new process
    for v in ["APPIMAGE", "APPDIR", "ARGV0", "OWD"] {
        cmd.env_remove(v);
    }
    cmd.spawn()
        .with_context(|| format!("can't start {}", appimage.display()))?;
    Ok(())
}

enum Toast {
    UpToDate,
    Failed(String),
    Available,
    Installing,
    InstallFailed(String),
}

/// What the top-bar update control asked for.
pub enum Click {
    Check,
    ShowUpdate,
}

pub struct Updates {
    task: Option<(bool, Task<Result<Option<Release>>>)>,
    pub available: Option<Release>,
    pub checked_at: Option<Instant>,
    toast: Option<(Toast, Instant)>,
    install: Install,
    installing: Option<(Task<Result<()>>, Arc<AtomicU32>)>,
}

impl Default for Updates {
    fn default() -> Self {
        Self {
            task: None,
            available: None,
            checked_at: None,
            toast: None,
            install: Install::detect(),
            installing: None,
        }
    }
}

impl Updates {
    /// `manual` checks always report back; automatic ones only when there's an update.
    pub fn check(&mut self, ctx: &egui::Context, manual: bool) {
        if self.task.is_none() {
            self.task = Some((manual, Task::spawn(ctx, latest)));
        }
    }

    pub fn checking(&self) -> bool {
        self.task.is_some()
    }

    /// Bring the "update available" notice back.
    pub fn show(&mut self) {
        if self.available.is_some() && self.installing.is_none() {
            self.toast = Some((Toast::Available, Instant::now()));
        }
    }

    /// "Checked 3 min ago" for the status bar.
    pub fn checked_label(&self) -> Option<String> {
        let m = self.checked_at?.elapsed().as_secs() / 60;
        Some(match m {
            0 => "Checked for updates just now".into(),
            1 => "Checked for updates 1 min ago".into(),
            m => format!("Checked for updates {m} min ago"),
        })
    }

    /// Whether the update button installs in place (vs. opening the release page).
    fn in_place(&self, rel: &Release) -> bool {
        matches!(self.install, Install::AppImage(_) | Install::WindowsSetup)
            && rel.package.is_some()
            && rel.sums.is_some()
    }

    fn start_install(&mut self, ctx: &egui::Context) {
        let Some(rel) = self.available.clone() else {
            return;
        };
        let install = self.install.clone();
        let progress = Arc::new(AtomicU32::new(0));
        let p = progress.clone();
        self.installing = Some((
            Task::spawn(ctx, move || match install {
                Install::AppImage(dest) => {
                    install_appimage(&rel, &dest, &p)?;
                    relaunch(&dest)
                }
                Install::WindowsSetup => run_setup(&rel, &p),
                Install::Flatpak | Install::Other => bail!("this copy can't update itself"),
            }),
            progress,
        ));
        self.toast = Some((Toast::Installing, Instant::now()));
    }

    /// Poll the check and draw its result (a notice in the bottom-right corner).
    pub fn ui(&mut self, ctx: &egui::Context) {
        if let Some((manual, mut t)) = self.task.take() {
            match t.poll() {
                None => self.task = Some((manual, t)),
                Some(r) => {
                    self.checked_at = Some(Instant::now());
                    let toast = match r {
                        Ok(Some(rel)) => {
                            self.available = Some(rel);
                            Some(Toast::Available)
                        }
                        Ok(None) => {
                            self.available = None;
                            manual.then_some(Toast::UpToDate)
                        }
                        Err(e) => manual.then(|| Toast::Failed(format!("{e:#}"))),
                    };
                    if self.installing.is_none() {
                        self.toast = toast.map(|t| (t, Instant::now()));
                    }
                }
            }
        }
        if let Some((t, _)) = self.installing.as_mut() {
            if let Some(r) = t.poll() {
                self.installing = None;
                match r {
                    // the new version is starting; it takes over from here
                    Ok(()) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                    Err(e) => {
                        self.toast = Some((Toast::InstallFailed(format!("{e:#}")), Instant::now()))
                    }
                }
            }
        }

        let Some((toast, since)) = &self.toast else {
            return;
        };
        let keep = matches!(
            toast,
            Toast::Available | Toast::Installing | Toast::InstallFailed(_)
        ) || since.elapsed() < Duration::from_secs(6);
        if !keep {
            self.toast = None;
            return;
        }
        let mut close = false;
        let mut update = false;
        egui::Area::new(egui::Id::new("update_toast"))
            .anchor(Align2::RIGHT_BOTTOM, [-16.0, -46.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(BG)
                    .stroke(egui::Stroke::new(1.0f32, LINE))
                    .corner_radius(4)
                    .inner_margin(egui::Margin::symmetric(16, 12))
                    .show(ui, |ui| {
                        ui.set_max_width(380.0);
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 2.0;
                                let v = self.available.as_ref().map_or("", |r| r.version.as_str());
                                let (title, body) = match toast {
                                    Toast::Available => (
                                        "Update available".to_owned(),
                                        match self.install {
                                            Install::Flatpak => format!(
                                                "LocalPlayer {v} is out (you have {VERSION}). \
                                                 Download the new .flatpak and open it to update."
                                            ),
                                            _ => format!(
                                                "LocalPlayer {v} is out (you have {VERSION})."
                                            ),
                                        },
                                    ),
                                    Toast::Installing => {
                                        let p = self
                                            .installing
                                            .as_ref()
                                            .map_or(0, |(_, p)| p.load(Ordering::Relaxed));
                                        (
                                            format!("Updating to {v}… {}%", p / 10),
                                            "LocalPlayer restarts by itself when it's done."
                                                .to_owned(),
                                        )
                                    }
                                    Toast::UpToDate => (
                                        "Up to date".to_owned(),
                                        format!("LocalPlayer {VERSION} is the latest version."),
                                    ),
                                    Toast::Failed(e) => {
                                        ("Couldn't check for updates".to_owned(), e.clone())
                                    }
                                    Toast::InstallFailed(e) => {
                                        ("Couldn't update".to_owned(), e.clone())
                                    }
                                };
                                ui.label(mono(&title, 11.0).color(MUTED));
                                ui.label(RichText::new(body).color(TEXT));
                            });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if matches!(toast, Toast::Installing) {
                                    ui.spinner();
                                    return;
                                }
                                if small_button(ui, "✕").clicked() {
                                    close = true;
                                }
                                let Some(r) = &self.available else { return };
                                let button = match toast {
                                    Toast::Available if self.in_place(r) => Some("Update now"),
                                    Toast::Available | Toast::InstallFailed(_) => Some("Download"),
                                    _ => None,
                                };
                                if let Some(label) = button {
                                    if small_button(ui, label).clicked() {
                                        if label == "Update now" {
                                            update = true;
                                        } else {
                                            chrome::open_url(&r.url);
                                            close = true;
                                        }
                                    }
                                }
                            });
                        });
                    });
            });
        if close {
            self.toast = None;
        }
        if update {
            self.start_install(ctx);
        }
        let every = if self.installing.is_some() { 100 } else { 500 };
        ctx.request_repaint_after(Duration::from_millis(every));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert!(newer("0.2.0", "0.1.0"));
        assert!(newer("v1.0", "0.9.9"));
        assert!(newer("0.1.10", "0.1.9"));
        assert!(!newer("0.1.0", "0.1.0"));
        assert!(!newer("v0.1.0-beta", "0.1.0"));
        assert!(!newer("0.0.9", "0.1.0"));
    }

    #[test]
    fn sums() {
        let s = "abc123  target/dist/localplayer-v0.2.0-x86_64.AppImage\n\
                 def456 *localplayer_0.2.0-1_amd64.deb\n";
        assert_eq!(
            expected_hash(s, "localplayer-v0.2.0-x86_64.AppImage").as_deref(),
            Some("abc123")
        );
        assert_eq!(
            expected_hash(s, "localplayer_0.2.0-1_amd64.deb").as_deref(),
            Some("def456")
        );
        assert_eq!(expected_hash(s, "other.AppImage"), None);
    }
}
