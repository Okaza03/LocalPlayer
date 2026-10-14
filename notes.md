# LocalPlayer v0.4.0

Your watch progress now follows you between devices and your server's website.

### New
- **Progress sync.** Resume points and watched marks are saved to the server, so a movie you stop halfway on one computer continues where you left off on another, or on your server's website. Changes made while the server is unreachable are sent once it's back. When two devices disagree, the most recent change wins.
- **Clearing your watch history** now clears it on the server too, so every device starts fresh.

### Fixed
- **The app icon shows up again on Linux** in the taskbar, app menu and Alt+Tab when you use the AppImage. Removing a `cargo xtask install` copy could delete the icon, and the app never put it back.
- **The server check no longer warns about missing reviews.** Reviews are still shown on the details pages when the server has them.

### For server owners
Progress sync needs two new optional endpoints, `GET /api/progress` and `POST /api/progress`, both behind the same sign-in as `/api/library`. [docs/API.md](https://github.com/Okaza03/LocalPlayer/blob/main/docs/API.md) describes them. The bundled Python server supports them when sign-in is on. Servers without them keep working as before, and progress then stays on each device.

### Downloads
| File | For |
|---|---|
| `localplayer-v0.4.0-x86_64-setup.exe` | Windows 10/11 (updates itself) |
| `localplayer-v0.4.0-aarch64-setup.exe` | Windows 11 on ARM64 (updates itself) |
| `localplayer-0.4.0-windows-x86_64.zip` | Windows, portable |
| `localplayer-0.4.0-windows-aarch64.zip` | Windows on ARM64, portable |
| `localplayer-v0.4.0-x86_64.AppImage` | any Linux distro (`chmod +x` and run, updates itself) |
| `localplayer-v0.4.0-x86_64.flatpak` | any distro with Flatpak (added a few minutes after the others) |
| `localplayer_0.4.0-1_amd64.deb` | Debian / Ubuntu / Mint |
| `localplayer-0.4.0-linux-x86_64.tar.gz` | manual install |

If you're using the Windows installer or the AppImage, click **Update now** to get this version.

Checksums for every file except the Flatpak are in `SHA256SUMS.txt`.
