# Valo v0.8.4

An update popup with the release notes, and subtitles that stay in sync.

### New
- **Update popup:** when a new version is out, Valo shows a popup with what it brings when you open it, and updates in one click (Update now on the Windows installer and the AppImage, Download elsewhere). Click Later, outside it or press Escape to close it; the update button in the top bar opens it again.
- **Subtitles that fit your file:** the admin panel's subtitle search lists the subtitles made for your copy of a movie first and marks them Matches your file, instead of only going by downloads. Servers can send `release` and `moviehash_match` with each result; see the library editing docs for how bulk downloads should pick.

### Fixed
- **Sync with Bluetooth audio:** the picture and subtitles ran about a third of a second ahead of the sound on Bluetooth headphones and speakers. Valo now accounts for the delay the audio device reports.

### Downloads
| File | For |
|---|---|
| `valo-v0.8.4-x86_64-setup.exe` | Windows 10/11 (updates itself) |
| `valo-v0.8.4-aarch64-setup.exe` | Windows 11 on ARM64 (updates itself) |
| `valo-0.8.4-windows-x86_64.zip` | Windows, portable |
| `valo-0.8.4-windows-aarch64.zip` | Windows on ARM64, portable |
| `valo-v0.8.4-x86_64.AppImage` | any Linux distro (`chmod +x` and run, updates itself) |
| `valo-v0.8.4-x86_64.flatpak` | any distro with Flatpak (added a few minutes after the others) |
| `valo_0.8.4-1_amd64.deb` | Debian / Ubuntu / Mint |
| `valo-0.8.4-linux-x86_64.tar.gz` | manual install |
| `valo-v0.8.4-androidtv.apk` | Android TV and Google TV (updates itself) |

If you're using the Windows installer, the AppImage or the TV app, click **Update** to get this version.

Checksums for every file except the Flatpak are in `SHA256SUMS.txt`.
