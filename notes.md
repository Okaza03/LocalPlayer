# Valo v0.7.0

A fresh home page, collections that keep film series together, a list of your signed-in devices, and a much bigger Admin page for server admins.

### New
- **A fresh home page:** the Movies, TV shows and genre rows show a new mix each time you open Valo instead of the same A to Z start, and a new Top rated row shows the best scored titles.
- **New arrivals:** when movies or series are added to your server, Valo shows them to you when you open the app or the library refreshes.
- **Collections:** film series like Batman stay together in A to Z, in the order your server's admin picked, instead of being spread across the alphabet.
- **Your devices:** your profile lists every device you're signed in on, and you can sign any of them out.
- **Refresh any time:** click "Library updated …" in the bottom-left corner to reload the library.
- **What's new:** after an update, Valo shows what changed.

### New for server admins
- **Admin page:** Admin in the profile menu now has a sidebar with a dashboard (library numbers, server status, library health, syncs and a live log), analytics, Discord requests, library editing, collections and users.
- **Library editing:** change a title's details, artwork, thumbnail and logo, fetch details from IMDb or TMDB, manage subtitles and edit episodes, all from the app.
- **Requests:** approve, deny or mark available the titles people ask for through the Discord bot; the requester is told on Discord.
- **Works with any server:** sections your server doesn't support say so and link to a setup guide. The server API is documented in `docs/api/`, including how to set up the Discord bot.

### Changed
- Every page opens at the top.
- Arrows, menus and other small icons are drawn crisply instead of depending on the font.
- Wide cards such as Continue watching use a title's thumbnail when the server has one.

### Downloads
| File | For |
|---|---|
| `valo-v0.7.0-x86_64-setup.exe` | Windows 10/11 (updates itself) |
| `valo-v0.7.0-aarch64-setup.exe` | Windows 11 on ARM64 (updates itself) |
| `valo-0.7.0-windows-x86_64.zip` | Windows, portable |
| `valo-0.7.0-windows-aarch64.zip` | Windows on ARM64, portable |
| `valo-v0.7.0-x86_64.AppImage` | any Linux distro (`chmod +x` and run, updates itself) |
| `valo-v0.7.0-x86_64.flatpak` | any distro with Flatpak (added a few minutes after the others) |
| `valo_0.7.0-1_amd64.deb` | Debian / Ubuntu / Mint |
| `valo-0.7.0-linux-x86_64.tar.gz` | manual install |
| `valo-v0.7.0-androidtv.apk` | Android TV and Google TV (updates itself) |

If you're using the Windows installer, the AppImage or the TV app, click **Update** to get this version.

Checksums for every file except the Flatpak are in `SHA256SUMS.txt`.
