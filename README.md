# LocalPlayer

A desktop video player and media-library client (Rust, egui, FFmpeg).

1. **Setup**: enter your media server's address (`192.168.1.20`, `nas.local:8080`, …). It's remembered for next time.
2. **Server check**: the first time you add a server, LocalPlayer checks every part of its API one by one (reachability, `/api/info`, sign-in, library, streaming, artwork) and shows how many titles have each piece of metadata. Problems that stop the app from working are errors and block the server. Missing extras, or a server without a sign-in, are warnings you can accept with _I know this is missing, continue_. _Copy report_ gives you the results to send to the server's admin.
3. **Sign in**: only shown if the server says it needs a login. The library checks run after it.
4. **Library**: Prime Video–style home page (featured banner, Continue watching, Recently added, Movies, TV shows, rows per genre), Movies/TV grids with sorting and genre filter, search (Ctrl+F), and detail pages with seasons and episodes.
5. **Player**: streams straight from the server. It remembers where you stopped and plays the next episode automatically.

You can still play local files: pass them on the command line, drag them onto the window, or use _Open local file…_.

```sh
cargo run --release -p localplayer
```

On Windows, first run `cargo xtask ffmpeg` once. It downloads a prebuilt LGPL FFmpeg into `target/ffmpeg`, which Windows builds link against (Linux builds compile FFmpeg from source). You also need LLVM (`winget install LLVM.LLVM`) for bindgen.

Settings and resume points are stored in `~/.config/localplayer/` (`%APPDATA%\localplayer\` on Windows).

## Installing

Each [release](https://github.com/Okaza03/LocalPlayer/releases) has:

- **AppImage**: `chmod +x` it and run it. The first launch adds LocalPlayer to your app menu (you can turn this off in Settings). Updates are one click: the app downloads the new AppImage, verifies it against `SHA256SUMS.txt`, replaces itself and restarts.
- **Flatpak**: `flatpak install --user localplayer-vX.Y.Z-x86_64.flatpak`. To update, install the newer file the same way, since sandboxed apps can't replace themselves. The manifest is in `packaging/flatpak/`.
- **.deb** and **.tar.gz**: the app tells you when an update is out and opens the release page.
- **Windows installer** (`localplayer-vX.Y.Z-x86_64-setup.exe`, or `-aarch64-setup.exe` for ARM64 PCs such as Snapdragon laptops): installs for your user only, so no admin prompt. Adds a Start menu entry and _Open with → LocalPlayer_ for video files. Updates are one click: the app downloads the next installer, verifies it against `SHA256SUMS.txt`, and runs it, which restarts LocalPlayer. Windows SmartScreen may warn on the first run because the installer isn't code-signed.
- **Windows zip**: portable. Unzip and run `localplayer.exe`, keeping the FFmpeg DLLs next to it. Updates open the release page.

Windows builds are made with `cargo xtask dist [arch]` (zip) and `cargo xtask installer [arch]` (needs [Inno Setup 6](https://jrsoftware.org/isinfo.php)), where `arch` is `x86_64` (default) or `aarch64`. The installer script is `packaging/windows/localplayer.iss`. Build ARM64 with the native ARM64 Rust toolchain and LLVM 19 or newer: older libclang can't parse MSVC's ARM64 headers.

## Releasing

1. Bump `version` in `Cargo.toml`.
2. Rewrite `notes.md` for the new version. Its first line must name it (`# LocalPlayer vX.Y.Z`). It becomes the release description.
3. Add `<release version="X.Y.Z" …/>` to `packaging/flatpak/io.github.okaza03.LocalPlayer.metainfo.xml`.
4. Commit, then `git tag vX.Y.Z && git push origin main vX.Y.Z`.

The release workflow refuses to publish if any of these are out of sync with the tag.

## Supported servers

**[docs/API.md](docs/API.md) is the full guide for server admins.** It covers every endpoint and field, what each step of the server check tests and how to fix it, plus examples for Laravel, Express and Flask, and curl commands to test by hand.

In short, there are three options:

1. **The bundled server.** `server/localplayer_server.py` implements the whole API with only the Python standard library. Metadata comes from `info.json` files and images next to the videos.
   ```sh
   python3 server/localplayer_server.py ~/Videos                 # Movies/ and Series/ inside
   python3 server/localplayer_server.py ~/Videos --add-user user # turn on sign-in
   python3 server/localplayer_server.py ~/Videos --add-user user --admin  # ...as an admin
   ```
2. **Your own app with the LocalPlayer API.** It needs four routes:

   | Route              | Needed                | Returns                                                                  |
   | ------------------ | --------------------- | ------------------------------------------------------------------------ |
   | `GET /api/info`    | always, no sign-in    | `{"name": "…", "auth": true, "version": 1}`                              |
   | `POST /api/login`  | when `auth` is `true` | `{"token": "…", "name": "…", "admin": false}`, or `401`                  |
   | `POST /api/logout` | optional              | invalidates the token                                                    |
   | `GET /api/library` | always                | `{"movies": […], "series": […]}` with `stream` URLs that support `Range` |

   After signing in, every request (including videos and images) carries `Authorization: Bearer <token>`. Only `title` plus `stream` (movies) or `seasons` (series) are required. Everything else (artwork, descriptions, genres, cast, reviews, file info, …) is optional and fills in the details pages. The server check shows which fields are missing.

3. **Any plain directory listing** (nginx `autoindex on`, Apache, Caddy `file_server browse`). LocalPlayer crawls it and guesses movies and series from folder and file names (`Show/Season 1/Show.S01E02.Title.mkv`, `Movies/Title (2010)/…`). It also picks up `poster.jpg` and `fanart.jpg`. The server must support HTTP range requests for seeking to work; nginx, Apache and Caddy do, but `python -m http.server` doesn't.

Only `http://` is supported for streaming. The bundled FFmpeg is built without TLS.
