# LocalPlayer v0.2.1

Fixes for the new movie, series and episode pages from v0.2.0.

### Fixed
- **File info shows up even when the server doesn't send it.** Quality, container, codecs, audio channels, bitrate, file size and runtime are now read from the video itself (about half a second), so the File · Metadata panel and the episode File panel aren't empty anymore.
- **Long cast names** are shortened with "…" instead of being cut off at both ends.
- **A server sending `null` for a list** (like `"cast": null`) no longer stops the whole library from loading.

### For server owners
Cast, director, release date, budget, box office, score, logo, creators, network, status, and file name and path still come from your server. See the API section in the README.

### Downloads
| File | For |
|---|---|
| `localplayer-v0.2.1-x86_64.AppImage` | any distro (`chmod +x` and run, updates itself) |
| `localplayer-v0.2.1-x86_64.flatpak` | any distro with Flatpak (added a few minutes after the others) |
| `localplayer_0.2.1-1_amd64.deb` | Debian / Ubuntu / Mint |
| `localplayer-0.2.1-linux-x86_64.tar.gz` | manual install |

If you're using the AppImage, click **Update now** to get this version.

Checksums for the AppImage, `.deb` and `.tar.gz` are in `SHA256SUMS.txt`.
