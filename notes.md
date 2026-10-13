# LocalPlayer v0.3.0

Adding a server now checks it step by step before you sign in.

### New
- **Server check.** When you add a server, LocalPlayer tests each part of it in turn and shows the result as it goes: whether it can be reached, `/api/info`, the server name, sign-in, `/api/login`, the library, movies and episodes, playback and seeking, and artwork.
- **Errors and warnings.** Problems that stop the app from working, like no library or videos that won't play, block the server until they're fixed. Smaller gaps, like missing artwork or a server without a sign-in, are warnings. Click **I know this is missing, continue** to go on anyway.
- **Metadata overview.** A bar for each field (poster, description, genres, cast, runtime, episode thumbnails, …) shows how many titles have it, for example `Poster 48/52`.
- **Copy report** copies every result as text, so you can send it to whoever runs the server.
- The check runs once, when a server is added. Reconnecting at startup works the same as before.

### For server owners
There's a new guide at [docs/API.md](https://github.com/Okaza03/LocalPlayer/blob/main/docs/API.md). It covers every endpoint and field, what each check step tests and how to fix it, setup for the bundled Python server and for nginx, Caddy and Apache folder listings, examples for Laravel, Express and Flask, and curl commands to test by hand.

### Downloads
| File | For |
|---|---|
| `localplayer-v0.3.0-x86_64.AppImage` | any distro (`chmod +x` and run, updates itself) |
| `localplayer-v0.3.0-x86_64.flatpak` | any distro with Flatpak (added a few minutes after the others) |
| `localplayer_0.3.0-1_amd64.deb` | Debian / Ubuntu / Mint |
| `localplayer-0.3.0-linux-x86_64.tar.gz` | manual install |

If you're using the AppImage, click **Update now** to get this version.

Checksums for the AppImage, `.deb` and `.tar.gz` are in `SHA256SUMS.txt`.
