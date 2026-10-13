# LocalPlayer v0.1.2

### New: one-click updates for the AppImage
When a new version is out, click **Update now** in the notice (or the **Update** tab at the top). LocalPlayer downloads the new AppImage, checks it against the release's `SHA256SUMS.txt`, swaps it in at the same place and restarts. There's nothing to download or `chmod` yourself, and your app-menu entry keeps working.

> This starts with the **next** update: v0.1.1 and older still send you to the download page once. Grab this AppImage by hand, and future updates are one click.

The `.deb` and `.tar.gz` versions still open the release page. Flatpak updates go through Flatpak (see below).

### New: Flatpak
There's now a `.flatpak` download that runs in Flatpak's sandbox on any distro:
```sh
flatpak install --user localplayer-v0.1.2-x86_64.flatpak
```
To update, download the newer `.flatpak` and run the same command again. LocalPlayer tells you when there's one to get. The sandbox can reach your network and read your Videos folder. Files elsewhere can be opened with **Open file…**.

### Changed
- The **Update** tab in the top bar now shows the update notice instead of opening your browser.

### Downloads
| File | For |
|---|---|
| `localplayer-v0.1.2-x86_64.AppImage` | any distro (`chmod +x` and run, updates itself) |
| `localplayer-v0.1.2-x86_64.flatpak` | any distro with Flatpak |
| `localplayer_0.1.2-1_amd64.deb` | Debian / Ubuntu / Mint |
| `localplayer-0.1.2-linux-x86_64.tar.gz` | manual install |

Checksums for the AppImage, `.deb` and `.tar.gz` are in `SHA256SUMS.txt`.
