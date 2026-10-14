//! Windows only: give the exe its icon and version info, and copy FFmpeg's
//! DLLs next to it so `cargo run` works.
fn main() {
    #[cfg(windows)]
    windows();
}

#[cfg(windows)]
fn windows() {
    use std::path::{Path, PathBuf};
    use std::{env, fs};

    let icon = "../../packaging/windows/localplayer.ico";
    println!("cargo:rerun-if-changed={icon}");
    println!("cargo:rerun-if-env-changed=FFMPEG_DIR");
    let mut res = winresource::WindowsResource::new();
    res.set_icon(icon)
        .set("FileDescription", "LocalPlayer")
        .set("ProductName", "LocalPlayer");
    res.compile().expect("can't embed the Windows icon");

    // OUT_DIR is target/<profile>/build/localplayer-<hash>/out
    let Some(ffmpeg) = env::var_os("FFMPEG_DIR").map(PathBuf::from) else {
        return;
    };
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let Some(profile_dir) = out.ancestors().nth(3).map(Path::to_path_buf) else {
        return;
    };
    let Ok(dlls) = fs::read_dir(ffmpeg.join("bin")) else {
        return;
    };
    for dll in dlls.flatten().map(|e| e.path()) {
        if dll
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("dll"))
        {
            let _ = fs::copy(&dll, profile_dir.join(dll.file_name().unwrap()));
        }
    }
}
