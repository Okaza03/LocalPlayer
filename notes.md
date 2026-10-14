# LocalPlayer v0.3.2

LocalPlayer now runs on Windows, on both regular (x64) PCs and ARM64 PCs.

### New
- **Windows installer.** It installs for your user only, so there's no admin prompt. It adds LocalPlayer to the Start menu and to *Open with* for video files without changing your default player. When a new version is out, **Update now** downloads the next installer, checks it against `SHA256SUMS.txt`, installs it and restarts LocalPlayer.
- **Windows portable zip.** Unzip it anywhere and run `localplayer.exe`, keeping the FFmpeg DLLs next to it.
- **ARM64 builds for Windows** (Snapdragon laptops and other ARM64 PCs) run natively instead of emulated.

On Windows, settings are stored in `%APPDATA%\localplayer\`. The installer isn't code-signed yet, so SmartScreen may warn the first time you run it: click *More info → Run anyway*.

### Downloads
| File | For |
|---|---|
| `localplayer-v0.3.2-x86_64-setup.exe` | Windows 10/11 (updates itself) |
| `localplayer-v0.3.2-aarch64-setup.exe` | Windows 11 on ARM64 (updates itself) |
| `localplayer-0.3.2-windows-x86_64.zip` | Windows, portable |
| `localplayer-0.3.2-windows-aarch64.zip` | Windows on ARM64, portable |
| `localplayer-v0.3.2-x86_64.AppImage` | any Linux distro (`chmod +x` and run, updates itself) |
| `localplayer-v0.3.2-x86_64.flatpak` | any distro with Flatpak (added a few minutes after the others) |
| `localplayer_0.3.2-1_amd64.deb` | Debian / Ubuntu / Mint |
| `localplayer-0.3.2-linux-x86_64.tar.gz` | manual install |

If you're using the AppImage, click **Update now** to get this version.

Checksums for every file except the Flatpak are in `SHA256SUMS.txt`.
