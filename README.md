# LocalPlayer

A desktop video player and media-library client (Rust, egui, FFmpeg).

1. **Setup**: enter your media server's address (`192.168.1.20`, `nas.local:8080`, …). It's remembered for next time.
2. **Sign in**: only shown if the server says it needs a login.
3. **Library**: Prime Video–style home page (featured banner, Continue watching, Recently added, Movies, TV shows, rows per genre), Movies/TV grids with sorting and genre filter, search (Ctrl+F), and detail pages with seasons and episodes.
4. **Player**: streams straight from the server. It remembers where you stopped and plays the next episode automatically.

You can still play local files: pass them on the command line, drag them onto the window, or use *Open local file…*.

```sh
cargo run --release -p localplayer
```

Settings and resume points are stored in `~/.config/localplayer/`.

## Installing

Each [release](https://github.com/Okaza03/LocalPlayer/releases) has:

- **AppImage**: `chmod +x` it and run it. The first launch adds LocalPlayer to your app menu (you can turn this off in Settings). Updates are one click: the app downloads the new AppImage, verifies it against `SHA256SUMS.txt`, replaces itself and restarts.
- **Flatpak**: `flatpak install --user localplayer-vX.Y.Z-x86_64.flatpak`. To update, install the newer file the same way, since sandboxed apps can't replace themselves. The manifest is in `packaging/flatpak/`.
- **.deb** and **.tar.gz**: the app tells you when an update is out and opens the release page.

## Releasing

1. Bump `version` in `Cargo.toml`.
2. Rewrite `notes.md` for the new version. Its first line must name it (`# LocalPlayer vX.Y.Z`). It becomes the release description.
3. Add `<release version="X.Y.Z" …/>` to `packaging/flatpak/io.github.okaza03.LocalPlayer.metainfo.xml`.
4. Commit, then `git tag vX.Y.Z && git push origin main vX.Y.Z`.

The release workflow refuses to publish if any of these are out of sync with the tag.

## Supported servers

**A server with the LocalPlayer API (below).** It provides titles, artwork, descriptions, genres and optional login. `server/localplayer_server.py` is a working reference implementation that only needs the Python standard library:

```sh
python3 server/localplayer_server.py ~/Videos                 # Movies/ and Series/ inside
python3 server/localplayer_server.py ~/Videos --add-user noah # turn on sign-in
python3 server/localplayer_server.py ~/Videos --add-user noah --admin  # ...as an admin
```

**Any plain directory listing** (nginx `autoindex on`, Apache, Caddy `file_server browse`). LocalPlayer crawls it and guesses movies and series from folder and file names (`Show/Season 1/Show.S01E02.Title.mkv`, `Movies/Title (2010)/…`). It also picks up `poster.jpg` and `fanart.jpg`. The server must support HTTP range requests for seeking to work; nginx, Apache and Caddy do, but `python -m http.server` doesn't.

Only `http://` is supported for streaming. The bundled FFmpeg is built without TLS.

## The LocalPlayer API

All URLs in responses can be absolute or relative to the server root.

### `GET /api/info` (no auth)
```json
{ "name": "Living-room server", "auth": true, "version": 1 }
```
`auth: true` makes the app show the sign-in screen.

### `POST /api/login`
Request `{"username": "...", "password": "..."}`. Response `{"token": "...", "name": "Noah", "admin": false}`.
`admin` is optional. Only users with `"admin": true` see the app's Admin buttons, which open `<server>/admin`. Hiding the buttons is just convenience, so the `/admin` page must still check permissions itself.
Return `401` for a wrong password. After that, every request carries `Authorization: Bearer <token>`, including video and image requests. Any `401` sends the user back to the sign-in screen.

### `POST /api/logout` (optional)
Invalidates the bearer token.

### `GET /api/library`
```json
{
  "movies": [{
    "id": "42", "title": "Inception", "year": 2010,
    "overview": "…", "genres": ["Sci-Fi"], "rating": "PG-13",
    "duration": 8880, "added": 1760000000,
    "poster": "/art/42/poster.jpg", "backdrop": "/art/42/backdrop.jpg",
    "stream": "/api/stream/movie/42"
  }],
  "series": [{
    "id": "7", "title": "The Office", "year": 2005, "overview": "…", "genres": ["Comedy"],
    "poster": "…", "backdrop": "…", "added": 1760000000,
    "seasons": [{ "number": 1, "episodes": [
      { "id": "7-1-1", "number": 1, "title": "Pilot", "overview": "…",
        "duration": 1380, "still": "…", "stream": "/api/stream/episode/123" }
    ]}]
  }]
}
```
Only `title` plus `stream` (movies) or `seasons` (series) are required. `duration` is in seconds and `added` is a Unix timestamp. Stream URLs must serve the raw file with `Range` support.

### Adding it to a Laravel app

```php
// routes/api.php
Route::get('/info', fn () => ['name' => config('app.name'), 'auth' => true, 'version' => 1]);
Route::post('/login', function (Request $r) {
    if (!Auth::attempt($r->only('username', 'password'))) return response()->json([], 401);
    return ['token' => $r->user()->createToken('localplayer')->plainTextToken, 'name' => $r->user()->name,
            'admin' => (bool) $r->user()->is_admin];
});
Route::middleware('auth:sanctum')->group(function () {
    Route::post('/logout', fn (Request $r) => $r->user()->currentAccessToken()->delete());
    Route::get('/library', [LocalPlayerController::class, 'library']);   // builds the JSON above
    Route::get('/stream/{type}/{id}', [LocalPlayerController::class, 'stream']); // return response()->file($path)
});
```
`response()->file()` (Symfony's `BinaryFileResponse`) handles range requests on its own.
