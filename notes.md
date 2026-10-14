# LocalPlayer v0.4.1

Fixes for progress sync and the Linux app icon, and a server check that covers every part of the API the app uses.

### Fixed
- **Progress from the server shows up in the app again.** If the server sent `"duration": null` for any title, the app couldn't read the whole list, so nothing synced. A missing duration is now treated as unknown.
- **The AppImage's icon shows in the taskbar and Alt+Tab** on KDE and other Wayland desktops. The window now names the menu entry the AppImage installs, so the desktop can find its icon. KDE is also told to reload icons after the app writes its icon.

### Changed
- **The server check tests progress sync.** It reads `GET /api/progress`, flags entries whose ids aren't in the library, and checks that `POST /api/progress` exists. The write test sends an entry no server should store, so your real progress is left alone.
- **It also checks sign-out, every kind of artwork and the admin page:** `POST /api/logout`, backdrops, title logos and episode thumbnails besides posters, and `/admin` when you sign in as an admin.

### For server owners
The new checks only warn, so no server stops working. [docs/API.md](https://github.com/Okaza03/LocalPlayer/blob/main/docs/API.md) lists every step and how to fix each warning.

### Downloads
| File | For |
|---|---|
| `localplayer-v0.4.1-x86_64-setup.exe` | Windows 10/11 (updates itself) |
| `localplayer-v0.4.1-aarch64-setup.exe` | Windows 11 on ARM64 (updates itself) |
| `localplayer-0.4.1-windows-x86_64.zip` | Windows, portable |
| `localplayer-0.4.1-windows-aarch64.zip` | Windows on ARM64, portable |
| `localplayer-v0.4.1-x86_64.AppImage` | any Linux distro (`chmod +x` and run, updates itself) |
| `localplayer-v0.4.1-x86_64.flatpak` | any distro with Flatpak (added a few minutes after the others) |
| `localplayer_0.4.1-1_amd64.deb` | Debian / Ubuntu / Mint |
| `localplayer-0.4.1-linux-x86_64.tar.gz` | manual install |

If you're using the Windows installer or the AppImage, click **Update now** to get this version.

Checksums for every file except the Flatpak are in `SHA256SUMS.txt`.
