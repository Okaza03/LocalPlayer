//! `cargo xtask <install|uninstall|dist|deb|appimage|ci>`
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{exit, Command};
use std::{env, fs};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

const NAME: &str = "homieplayer";

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        exit(1);
    }
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn sh(cmd: &mut Command) -> Res {
    println!("+ {cmd:?}");
    if cmd.status()?.success() { Ok(()) } else { Err(format!("command failed: {cmd:?}").into()) }
}

fn cargo(args: &[&str]) -> Res {
    sh(Command::new("cargo").args(args).current_dir(root()))
}

fn run() -> Res {
    let mut a = env::args().skip(1);
    match a.next().as_deref() {
        Some("install") => install(a.next()),
        Some("uninstall") => uninstall(a.next()),
        Some("dist") => dist(),
        Some("deb") => cargo(&["deb", "-p", NAME]),
        Some("appimage") => appimage(),
        Some("ci") => {
            cargo(&["fmt", "--all", "--check"])?;
            cargo(&["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"])?;
            cargo(&["test", "--workspace"])
        }
        _ => {
            eprintln!("usage: cargo xtask <install [prefix] | uninstall [prefix] | dist | deb | appimage | ci>");
            exit(2);
        }
    }
}

fn put(src: &Path, dst: &Path, mode: u32) -> Res {
    fs::create_dir_all(dst.parent().unwrap())?;
    fs::copy(src, dst)?;
    fs::set_permissions(dst, fs::Permissions::from_mode(mode))?;
    Ok(())
}

fn prefix_or_default(p: Option<String>) -> PathBuf {
    p.map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env::var("HOME").expect("HOME not set")).join(".local"))
}

fn stage(prefix: &Path, absolute_exec: bool) -> Res {
    let r = root();
    let bin = prefix.join("bin").join(NAME);
    put(&r.join("target/release").join(NAME), &bin, 0o755)?;
    put(
        &r.join("packaging/homieplayer.svg"),
        &prefix.join("share/icons/hicolor/scalable/apps/homieplayer.svg"),
        0o644,
    )?;
    let mut desktop = fs::read_to_string(r.join("packaging/homieplayer.desktop"))?;
    if absolute_exec {
        desktop = desktop.replace("Exec=homieplayer", &format!("Exec={}", bin.display()));
    }
    let dst = prefix.join("share/applications/homieplayer.desktop");
    fs::create_dir_all(dst.parent().unwrap())?;
    fs::write(&dst, desktop)?;
    Ok(())
}

fn install(prefix: Option<String>) -> Res {
    let prefix = prefix_or_default(prefix);
    cargo(&["build", "--release", "-p", NAME])?;
    stage(&prefix, true)?;
    let _ = Command::new("update-desktop-database").arg(prefix.join("share/applications")).status();
    println!("Installed to {}. Run `{NAME}` or open it from your app menu.", prefix.display());
    Ok(())
}

fn uninstall(prefix: Option<String>) -> Res {
    let prefix = prefix_or_default(prefix);
    for f in [
        "bin/homieplayer",
        "share/applications/homieplayer.desktop",
        "share/icons/hicolor/scalable/apps/homieplayer.svg",
    ] {
        let _ = fs::remove_file(prefix.join(f));
    }
    println!("Removed from {}", prefix.display());
    Ok(())
}

fn dist() -> Res {
    cargo(&["build", "--release", "-p", NAME])?;
    let name = format!("{NAME}-{}-linux-{}", env!("CARGO_PKG_VERSION"), env::consts::ARCH);
    let out = root().join("target/dist");
    let tree = out.join(&name);
    let _ = fs::remove_dir_all(&tree);
    stage(&tree, false)?;
    sh(Command::new("tar")
        .args(["-czf", &format!("{name}.tar.gz"), &name])
        .current_dir(&out))?;
    println!("Built {}", out.join(format!("{name}.tar.gz")).display());
    Ok(())
}

fn appimage() -> Res {
    cargo(&["build", "--release", "-p", NAME])?;
    let r = root();
    let work = r.join("target/appimage");
    fs::create_dir_all(&work)?;
    // needs linuxdeploy on PATH: https://github.com/linuxdeploy/linuxdeploy
    sh(Command::new("linuxdeploy")
        .args(["--appdir", "AppDir"])
        .arg("--executable").arg(r.join("target/release").join(NAME))
        .arg("--desktop-file").arg(r.join("packaging/homieplayer.desktop"))
        .arg("--icon-file").arg(r.join("packaging/homieplayer.svg"))
        .args(["--output", "appimage"])
        .current_dir(&work))
}