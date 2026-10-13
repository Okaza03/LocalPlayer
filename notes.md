# LocalPlayer v0.1.3

### Changed: Admin buttons only for admins
The **Admin** item in the profile menu and the **Open admin** button in your profile are now only shown to admins. Everyone else no longer sees them.

The server decides who's an admin: it adds `"admin": true` to its sign-in reply (`POST /api/login`) for those users. Servers that don't send it show no Admin buttons to anyone.

- **Admins who chose "Keep me signed in" before this update:** sign out and back in once to get the buttons back.
- **Running the bundled Python server:** create admins with `--add-user NAME --admin`.
- **Server developers:** the README's API section shows the new field. Hiding the buttons is only cosmetic, so your `/admin` page still has to check permissions itself.

### Downloads
| File | For |
|---|---|
| `localplayer-v0.1.3-x86_64.AppImage` | any distro (`chmod +x` and run, updates itself) |
| `localplayer-v0.1.3-x86_64.flatpak` | any distro with Flatpak (added a few minutes after the others) |
| `localplayer_0.1.3-1_amd64.deb` | Debian / Ubuntu / Mint |
| `localplayer-0.1.3-linux-x86_64.tar.gz` | manual install |

If you're on v0.1.2 with the AppImage, click **Update now** to get this version.

Checksums for the AppImage, `.deb` and `.tar.gz` are in `SHA256SUMS.txt`.
