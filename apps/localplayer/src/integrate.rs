//! When running as an AppImage, put LocalPlayer in the desktop's app menu
//! (~/.local/share/applications), pointing at wherever the AppImage lives.
use std::path::PathBuf;
use std::process::Command;
use std::{env, fs, io};

const DESKTOP: &str = include_str!("../../../packaging/localplayer.desktop");
const ICON: &str = include_str!("../../../packaging/localplayer.svg");
/// Separate from the `cargo xtask install` entry so neither clobbers the other.
const ENTRY: &str = "localplayer-appimage.desktop";

/// Path of the running AppImage, set by the AppImage runtime.
pub fn appimage() -> Option<String> {
    env::var("APPIMAGE").ok().filter(|p| !p.is_empty())
}

fn data_home() -> Option<PathBuf> {
    env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
}

/// Escape a desktop-file string value (backslashes).
fn string(s: &str) -> String {
    s.replace('\\', "\\\\")
}

/// Quote one Exec argument per the desktop entry spec.
fn exec_arg(s: &str) -> String {
    let mut q = String::from("\"");
    for c in s.chars() {
        if matches!(c, '"' | '`' | '$' | '\\') {
            q.push('\\');
        }
        q.push(c);
    }
    q.push('"');
    string(&q).replace('%', "%%")
}

fn entry(img: &str) -> String {
    // TryExec makes menus hide the entry once the AppImage is deleted
    DESKTOP.replace(
        "Exec=localplayer",
        &format!("TryExec={}\nExec={}", string(img), exec_arg(img)),
    )
}

/// Add, refresh, or remove the menu entry. No-op unless running as an AppImage.
pub fn sync(enabled: bool) {
    let (Some(img), Some(data)) = (appimage(), data_home()) else {
        return;
    };
    let apps = data.join("applications");
    let desktop = apps.join(ENTRY);
    let res = if enabled {
        let want = entry(&img);
        if fs::read_to_string(&desktop).ok().as_deref() == Some(want.as_str()) {
            return; // up to date
        }
        (|| -> io::Result<()> {
            let icon = data.join("icons/hicolor/scalable/apps/localplayer.svg");
            fs::create_dir_all(icon.parent().unwrap())?;
            fs::write(&icon, ICON)?;
            fs::create_dir_all(&apps)?;
            fs::write(&desktop, want)
        })()
    } else {
        match fs::remove_file(&desktop) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => return,
            r => r,
        }
    };
    match res {
        Ok(()) => {
            let _ = Command::new("update-desktop-database").arg(&apps).status();
        }
        Err(e) => eprintln!("can't update app menu entry: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoting() {
        assert_eq!(exec_arg("/a b/L.AppImage"), "\"/a b/L.AppImage\"");
        assert_eq!(exec_arg("/x$y\"z%"), "\"/x\\\\$y\\\\\"z%%\"");
    }

    #[test]
    fn entry_points_at_appimage() {
        let e = entry("/home/u/Apps/LocalPlayer.AppImage");
        assert!(e.contains("TryExec=/home/u/Apps/LocalPlayer.AppImage\n"));
        assert!(e.contains("Exec=\"/home/u/Apps/LocalPlayer.AppImage\" %F"));
        assert!(!e.contains("Exec=localplayer"));
    }
}
