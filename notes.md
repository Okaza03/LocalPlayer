# Valo v0.5.0

LocalPlayer is now **Valo**, with a new logo and an app for Android TV and Google TV.

### New
- **Valo for Android TV / Google TV** (`valo-v0.5.0-androidtv.apk`). Made for the remote, and it plays with the TV's own hardware video decoders. It looks and works like the desktop app: Home, Movies and TV Shows with the same filters, search, Continue watching, resume, and autoplay of the next episode. Progress syncs with the desktop app through your server. It can update itself from the button in its top bar.
- **A splash screen** with the new wordmark while your library loads after signing in.

### Changed
- **The new name and logo everywhere:** the app, its menu entries and icons, the installers and the GitHub page (now github.com/Okaza03/Valo; old links still work).
- **Updating keeps everything.** Your server, sign-in, settings and resume points carry over from LocalPlayer. The Windows installer and the AppImage replace the old app, and the .deb replaces the `localplayer` package.
- **Flatpak has a new id, `io.github.okaza03.Valo`.** Install the new file, start it once so it copies your settings, then remove the old one with `flatpak uninstall io.github.okaza03.LocalPlayer`.
- **The bundled server is now `server/valo_server.py`.** It moves its users, sign-ins and progress from `.localplayer` to `.valo` on first start.

### Installing on a TV
On the TV, install the free **Downloader** app from the Play Store, allow it to install unknown apps when it asks, and enter the link to `valo-v0.5.0-androidtv.apk` from this page. Later versions install from the app's own Update button.

### Downloads
| File | For |
|---|---|
| `valo-v0.5.0-x86_64-setup.exe` | Windows 10/11 (updates itself) |
| `valo-v0.5.0-aarch64-setup.exe` | Windows 11 on ARM64 (updates itself) |
| `valo-0.5.0-windows-x86_64.zip` | Windows, portable |
| `valo-0.5.0-windows-aarch64.zip` | Windows on ARM64, portable |
| `valo-v0.5.0-x86_64.AppImage` | any Linux distro (`chmod +x` and run, updates itself) |
| `valo-v0.5.0-x86_64.flatpak` | any distro with Flatpak (added a few minutes after the others) |
| `valo_0.5.0-1_amd64.deb` | Debian / Ubuntu / Mint |
| `valo-0.5.0-linux-x86_64.tar.gz` | manual install |
| `valo-v0.5.0-androidtv.apk` | Android TV and Google TV (updates itself) |

If you're using the Windows installer or the AppImage, click **Update now** to get this version.

Checksums for every file except the Flatpak are in `SHA256SUMS.txt`.
