# LocalPlayer v0.3.1

A fix for the server check from v0.3.0.

### Fixed
- **Laravel servers pass the sign-in check.** The check tests `/api/login` with an empty username and password. Laravel answers that with `422` (a validation error), which the check wrongly counted as a failure. `422` now counts as "sign-in found", and the sign-in screen shows it as a wrong username or password.
- **API requests ask for JSON.** Every request to the API now sends `Accept: application/json`, so frameworks like Laravel answer `401` when you're signed out instead of redirecting to a web login page.

### For server owners
Nothing to change on your side. [docs/API.md](https://github.com/Okaza03/LocalPlayer/blob/main/docs/API.md) now lists `422` as a valid answer to a failed sign-in.

### Downloads
| File | For |
|---|---|
| `localplayer-v0.3.1-x86_64.AppImage` | any distro (`chmod +x` and run, updates itself) |
| `localplayer-v0.3.1-x86_64.flatpak` | any distro with Flatpak (added a few minutes after the others) |
| `localplayer_0.3.1-1_amd64.deb` | Debian / Ubuntu / Mint |
| `localplayer-0.3.1-linux-x86_64.tar.gz` | manual install |

If you're using the AppImage, click **Update now** to get this version.

Checksums for the AppImage, `.deb` and `.tar.gz` are in `SHA256SUMS.txt`.
