# The LocalPlayer server API

This guide is for whoever runs the media server. It covers what LocalPlayer asks the server for, the JSON it expects back, and how to get every check on the app's server check to pass.

- [Three ways to serve a library](#three-ways-to-serve-a-library)
- [The server check](#the-server-check)
- [Endpoints](#endpoints)
- [The library JSON](#the-library-json)
- [Streams and images](#streams-and-images)
- [Examples](#examples)
- [Testing with curl](#testing-with-curl)
- [Troubleshooting](#troubleshooting)

## Three ways to serve a library

| | Setup | Metadata | Sign-in |
|---|---|---|---|
| **Bundled Python server** | One command, no code | Everything, from `info.json` files | Optional |
| **Your own API** | Add four routes to an existing app | Everything you send | Optional |
| **Plain folder listing** | Turn on nginx/Apache/Caddy directory listings | Titles, years and artwork guessed from file names | None |

### Option 1: the bundled Python server

`server/localplayer_server.py` implements the whole API with only the Python 3.9+ standard library.

```sh
python3 server/localplayer_server.py ~/Videos                     # port 8080, no sign-in
python3 server/localplayer_server.py ~/Videos --port 9000 --name "Living room"
python3 server/localplayer_server.py ~/Videos --add-user noah     # asks for a password, turns sign-in on
python3 server/localplayer_server.py ~/Videos --add-user noah --admin
```

`--add-user` creates the user and exits. Start the server again afterwards. Users and tokens are stored in `~/Videos/.localplayer/`. Sign-in is on as soon as one user exists.

Put your files in `Movies/` and `Series/` (`Films/`, `TV/`, `TV Shows/` and `Shows/` also work):

```
Videos/
├── Movies/
│   └── Inception (2010)/
│       ├── Inception.2010.1080p.mkv
│       ├── poster.jpg          (or folder.jpg, cover.jpg, <video name>-poster.jpg)
│       ├── fanart.jpg          (or backdrop.jpg, background.jpg, <video name>-fanart.jpg)
│       ├── logo.png            (or clearlogo.png: transparent title artwork)
│       └── info.json
└── Series/
    └── The Office/
        ├── poster.jpg
        ├── fanart.jpg
        ├── info.json
        └── Season 1/
            ├── The.Office.S01E01.Pilot.mkv
            └── The.Office.S01E01.Pilot.jpg   (episode thumbnail, or <name>-thumb.jpg)
```

Everything else comes from `info.json`. Every key is optional:

```json
{
  "title": "Inception",
  "year": 2010,
  "overview": "A thief who steals corporate secrets through dream-sharing…",
  "genres": ["Sci-Fi", "Action"],
  "rating": "PG-13",
  "duration": 8880,
  "director": "Christopher Nolan",
  "cast": [{ "name": "Leonardo DiCaprio", "character": "Cobb", "photo": "dicaprio.jpg" }],
  "released": "2010-07-16",
  "budget": 160000000,
  "revenue": 836800000,
  "score": 8.4,
  "score_source": "TMDB",
  "reviews": [{ "quote": "A wildly ambitious…", "source": "Empire", "score": "5 / 5" }]
}
```

A series' `info.json` can also have `creators`, `network`, `status`, `end_year` and per-episode details:

```json
{
  "creators": ["Greg Daniels"],
  "network": "NBC",
  "status": "Ended",
  "end_year": 2013,
  "episodes": {
    "S01E01": { "title": "Pilot", "overview": "…", "aired": "2005-03-24", "score": 7.5 }
  }
}
```

A cast `photo` can be a URL or an image file next to `info.json`. The server works out `added` (file date) and `file` (size, container, quality) on its own. Episode titles are taken from the file name when `info.json` doesn't give one.

### Option 2: your own API

Add the [endpoints](#endpoints) below to an existing app. Only `/api/info`, `/api/library` and the stream URLs are required. Add `/api/login` if the library should be private. See [Examples](#examples) for Laravel, Express and Flask.

### Option 3: a plain folder listing

If `/api/info` is missing but the server lists its folders as HTML, LocalPlayer crawls the listing (up to 6 folders deep) and guesses movies and series from the names:

- `Show/Season 1/Show.S01E02.Title.mkv`, `Show.1x02.mkv`: episodes
- `Movies/Title (2010)/Title.mkv`, `Title.2010.1080p.mkv`: movies
- `poster.jpg`, `folder.jpg`, `fanart.jpg` next to the videos: artwork

The server must support HTTP range requests for seeking to work. nginx, Apache and Caddy do. `python -m http.server` doesn't.

```nginx
# nginx
location / {
    root /srv/videos;
    autoindex on;
}
```

```caddyfile
# Caddy
:8080 {
    root * /srv/videos
    file_server browse
}
```

```apache
# Apache
<Directory "/srv/videos">
    Options +Indexes
</Directory>
```

A folder listing can't give descriptions, genres, ratings, cast or a sign-in, so the server check shows these as warnings.

## The server check

When someone adds a server in LocalPlayer, the app checks each part of the API in turn before it shows the library. Each step ends as one of:

- **PASS**: works.
- **WARN**: LocalPlayer can use the server, but something is missing. The user has to click *I know this is missing, continue* to go on.
- **FAIL**: LocalPlayer can't use the server until this is fixed. The user can only try again or copy a report for you.
- **LATER**: needs a sign-in first; checked again after signing in.

The steps, in order:

| Step | Checks | FAIL when | WARN when |
|---|---|---|---|
| Reach the server | `GET /` answers at all | Nothing answers (wrong address/port, server down, firewall) | |
| LocalPlayer API | `GET /api/info` returns JSON | Neither `/api/info` nor a folder listing exists | No `/api/info`, but there's a folder listing to fall back to |
| Server name | `name` in `/api/info` | | `name` is missing; the app shows the host instead |
| Sign-in required | `auth` in `/api/info` | | `auth` is `false`: anyone on the network can watch |
| Sign-in API | `POST /api/login` with an empty username and password | 404/405/501, or a non-JSON answer | It signs in with an empty username and password |
| Library API | `GET /api/library` *without* a token | 404/405/501 | It answers without a token even though `auth` is `true` |
| *(sign-in screen, if `auth` is `true`)* | | | |
| Library | `GET /api/library` with the token, parsed | Error status, `401` with a fresh token, or JSON the app can't read | |
| Movies | Each movie has a `stream` | No movie has a `stream` | Some movies have no `stream` (they're hidden) |
| Series and episodes | Each series has episodes, each episode a `stream` | No episodes, or none has a `stream` | Some series have no episodes, or some episodes have no `stream` |
| Something to watch | At least one playable movie or episode | The library would be empty | |
| Streaming | `GET <first stream>` with `Range: bytes=0-1` | Error status, or an HTML/JSON answer instead of a video | Answers `200` instead of `206`: seeking and resuming won't work |
| Artwork | Downloads and decodes the first poster | | No posters at all, or the poster can't be loaded |
| Metadata | Counts how many titles have each field (below) | No title has a `title` | Any field is missing on some or all titles |

Under **Metadata** the app shows a bar for each field, e.g. `Poster 48/52`. Fields on every title pass. Fields on some or none are warnings. These are the fields it counts:

| Group | Fields |
|---|---|
| Movies and series | Title, Description (`overview`), Poster, Backdrop, Year, Genres, Age rating (`rating`), Date added (`added`), Title logo (`logo`), Cast, Release date (`released`), Score, Reviews |
| Movies | Runtime (`duration`), Director, Budget, Box office (`revenue`), File info (`file`) |
| Series | Creators, Network, Status |
| Episodes | Title, Description (`overview`), Thumbnail (`still`), Runtime (`duration`), Air date (`aired`), Score, File info (`file`) |

With a folder listing only Title, Poster, Backdrop, Year and episode Title are counted, because nothing else can be guessed from file names.

*Copy report* on the check screen copies every result as plain text. Users can send it to you. The check runs once, when a server is added. After that the app reconnects without it.

## Endpoints

All paths are relative to the address the user types. If they type `192.168.1.20:8080/media`, the app requests `http://192.168.1.20:8080/media/api/info`. Only `http://` works for streaming (the bundled FFmpeg is built without TLS).

### `GET /api/info`: required, no sign-in

```json
{ "name": "Living-room server", "auth": true, "version": 1 }
```

| Field | Type | |
|---|---|---|
| `name` | string | Shown in the title bar. Missing: the check warns and the host is shown. |
| `auth` | bool | `true` shows the sign-in screen. `false` (or missing): the check warns that the library is open. |
| `version` | int | Send `1`. |

This must answer without a token, even when `auth` is `true`.

### `POST /api/login`: required when `auth` is `true`

Request body:

```json
{ "username": "noah", "password": "hunter2" }
```

Success, `200`:

```json
{ "token": "3f1c…", "name": "Noah", "admin": false }
```

| Field | Type | |
|---|---|---|
| `token` | string | Required. Sent back as `Authorization: Bearer <token>` on every later request, including streams and images. |
| `name` | string | Display name. Defaults to the username. |
| `admin` | bool | `true` shows the app's Admin buttons, which open `<server>/admin` in the browser. Hiding the buttons is only a convenience, so `/admin` must check permissions itself. |

A wrong username or password must get `401` (`400` and `403` work too). Don't answer `200` with an error message in it. The check sends an empty username and password and expects one of those codes.

### `POST /api/logout`: optional

Sent with the bearer token when the user signs out. Invalidate the token. The answer is ignored.

### `GET /api/library`: required

Returns the whole catalog ([format below](#the-library-json)). When `auth` is `true`, answer `401` without a valid token. Any `401` while the app is running sends the user back to the sign-in screen.

### Stream URLs: required

Each movie and episode names its own `stream` URL. The path is up to you, for example `/api/stream/movie/42`. See [Streams and images](#streams-and-images).

### `GET /admin`: optional

A web page for admins. It's only linked when the sign-in answered `"admin": true`.

## The library JSON

```json
{
  "movies": [ <movie>, … ],
  "series": [ <series>, … ]
}
```

Either list can be empty, missing or `null`. Any field can be left out or set to `null`. URLs can be absolute (`http://cdn/x.jpg`) or relative to the server (`/art/42/poster.jpg`).

### Movie

| Field | Type | Needed | Shown | Example |
|---|---|---|---|---|
| `title` | string | **required** | everywhere | `"Inception"` |
| `stream` | URL | **required** | play button | `"/api/stream/movie/42"` |
| `id` | string | recommended | keeps resume points and *My list* stable | `"42"` |
| `year` | int | | tiles, details | `2010` |
| `overview` | string | | details, banner | `"A thief who…"` |
| `poster` | URL | | tiles (2:3) | `"/art/42/poster.jpg"` |
| `backdrop` | URL | | banner, details (16:9) | `"/art/42/backdrop.jpg"` |
| `logo` | URL | | replaces the title text (transparent PNG) | `"/art/42/logo.png"` |
| `genres` | string[] | | genre rows and filter | `["Sci-Fi"]` |
| `rating` | string | | age rating badge | `"PG-13"` |
| `duration` | number | | runtime, in seconds | `8880` |
| `added` | int | | *Recently added*, Unix time | `1760000000` |
| `released` | string | | details, `YYYY-MM-DD` | `"2010-07-16"` |
| `director` | string | | details | `"Christopher Nolan"` |
| `cast` | [Cast](#cast-member)[] | | cast row | |
| `score` | number | | out of 10 | `8.4` |
| `score_source` | string | | next to the score | `"TMDB"` |
| `reviews` | [Review](#review)[] | | critics' quotes | |
| `budget` | int | | US dollars | `160000000` |
| `revenue` | int | | box office, US dollars | `836800000` |
| `file` | [File](#file-info) | | file details (read from the video when missing) | |

### Series

Same as a movie, minus `stream`, `director`, `budget`, `revenue` and `duration`, plus:

| Field | Type | Needed | Shown | Example |
|---|---|---|---|---|
| `seasons` | [Season](#season)[] | **required** (at least one episode) | season picker | |
| `creators` | string[] | | details | `["Greg Daniels"]` |
| `network` | string | | details | `"NBC"` |
| `status` | string | | details | `"Ended"`, `"Returning Series"` |
| `end_year` | int | | `2005–2013` | `2013` |

### Season

| Field | Type | |
|---|---|---|
| `number` | int | `0` is shown as *Specials* |
| `title` | string | Optional. Replaces "Season N". |
| `episodes` | [Episode](#episode)[] | |

### Episode

| Field | Type | Needed | |
|---|---|---|---|
| `stream` | URL | **required** | |
| `number` | int | recommended | Episodes are sorted by this |
| `id` | string | recommended | Keeps resume points stable |
| `title` | string | recommended | |
| `overview` | string | | |
| `still` | URL | | Thumbnail, 16:9 |
| `duration` | number | | Seconds |
| `aired` | string | | `YYYY-MM-DD` |
| `score` | number | | Out of 10 |
| `file` | [File](#file-info) | | |

### Cast member

```json
{ "name": "Cillian Murphy", "character": "Jim", "photo": "/art/people/1.jpg" }
```

### Review

```json
{ "quote": "…", "source": "Reverse Shot", "score": "4.5 / 5" }
```

### File info

```json
{
  "quality": "1080p", "container": "mp4", "video_codec": "h264",
  "audio_codec": "aac", "audio_channels": 6, "bitrate": 2600000,
  "size": 2090000000, "name": "28.Days.Later.2002.1080p.mp4", "path": "/movies/28 Days Later"
}
```

`bitrate` is in bits per second and `size` in bytes. When `file` is missing, the app reads what it can from the video stream itself.

### A complete example

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
    "poster": "/art/7/poster.jpg", "backdrop": "/art/7/backdrop.jpg", "added": 1760000000,
    "seasons": [{ "number": 1, "episodes": [
      { "id": "7-1-1", "number": 1, "title": "Pilot", "overview": "…",
        "duration": 1380, "still": "/art/7/1/1.jpg", "stream": "/api/stream/episode/123" }
    ]}]
  }]
}
```

## Streams and images

Stream URLs must serve the raw video file and support range requests:

- A request with `Range: bytes=0-1` gets `206 Partial Content` with a `Content-Range: bytes 0-1/<size>` header and those bytes.
- Send `Accept-Ranges: bytes` and a video `Content-Type` (`video/mp4`, `video/x-matroska`, or `application/octet-stream`).
- Never answer with an HTML page (a login page or a 404 page). The check fails on `text/html` and JSON.

Most frameworks do this for you: Laravel's `response()->file()`, Express's `res.sendFile()`, Flask's `send_file(..., conditional=True)`, nginx's `alias`/`root`.

The app transcodes nothing. It plays whatever FFmpeg can decode (H.264, HEVC, AV1, VP9, AAC, AC3, Opus, …) in any common container.

Images (posters, backdrops, logos, stills, cast photos) can be JPEG, PNG or WebP. When a sign-in is on, image and stream requests to the server carry the same `Authorization: Bearer` header. Images on another host (a CDN, TMDB) are fetched without it.

## Examples

### Laravel

```php
// routes/api.php
Route::get('/info', fn () => ['name' => config('app.name'), 'auth' => true, 'version' => 1]);

Route::post('/login', function (Request $r) {
    if (!Auth::attempt($r->only('username', 'password'))) {
        return response()->json(['error' => 'wrong username or password'], 401);
    }
    return [
        'token' => $r->user()->createToken('localplayer')->plainTextToken,
        'name'  => $r->user()->name,
        'admin' => (bool) $r->user()->is_admin,
    ];
});

Route::middleware('auth:sanctum')->group(function () {
    Route::post('/logout', fn (Request $r) => $r->user()->currentAccessToken()->delete());
    Route::get('/library', [LocalPlayerController::class, 'library']);           // builds the JSON above
    Route::get('/stream/{type}/{id}', [LocalPlayerController::class, 'stream']); // return response()->file($path)
});
```

`response()->file()` (Symfony's `BinaryFileResponse`) handles range requests on its own. Make sure unauthenticated API requests get a JSON `401` and not a redirect to a login page. Sanctum does this when the request has `Accept: application/json`, or you can render `AuthenticationException` as JSON for `api/*`.

### Express (Node.js)

```js
const express = require("express");
const crypto = require("crypto");
const app = express();
app.use(express.json());

const users = { noah: "hunter2" };   // use real password hashing
const tokens = new Map();            // token -> username
const movies = [{ id: "1", title: "Inception", year: 2010, file: "/srv/videos/Inception.mkv",
                  poster: "/srv/videos/Inception.jpg" }];

app.get("/api/info", (req, res) => res.json({ name: "Node server", auth: true, version: 1 }));

app.post("/api/login", (req, res) => {
  const { username, password } = req.body || {};
  if (!username || users[username] !== password) return res.status(401).json({ error: "wrong" });
  const token = crypto.randomBytes(32).toString("hex");
  tokens.set(token, username);
  res.json({ token, name: username });
});

const auth = (req, res, next) => {
  const t = (req.get("Authorization") || "").replace(/^Bearer /, "");
  tokens.has(t) ? next() : res.status(401).json({ error: "sign in required" });
};

app.post("/api/logout", auth, (req, res) => {
  tokens.delete(req.get("Authorization").slice(7));
  res.json({});
});

app.get("/api/library", auth, (req, res) => res.json({
  movies: movies.map(m => ({ id: m.id, title: m.title, year: m.year,
    poster: `/art/movie/${m.id}`, stream: `/api/stream/movie/${m.id}` })),
  series: [],
}));

const byId = id => movies.find(m => m.id === id);
app.get("/api/stream/movie/:id", auth, (req, res) => res.sendFile(byId(req.params.id).file)); // Range-aware
app.get("/art/movie/:id", auth, (req, res) => res.sendFile(byId(req.params.id).poster));

app.listen(8080, "0.0.0.0");
```

### Flask (Python)

```python
from flask import Flask, jsonify, send_file

app = Flask(__name__)
MOVIES = {"1": {"title": "Inception", "year": 2010, "path": "/srv/videos/Inception.mkv"}}

@app.get("/api/info")
def info():
    return {"name": "Flask server", "auth": False, "version": 1}

@app.get("/api/library")
def library():
    return {"movies": [{"id": k, "title": m["title"], "year": m["year"],
                        "stream": f"/api/stream/movie/{k}"} for k, m in MOVIES.items()],
            "series": []}

@app.get("/api/stream/movie/<id>")
def stream(id):
    return send_file(MOVIES[id]["path"], conditional=True)  # conditional=True enables Range

app.run(host="0.0.0.0", port=8080)
```

This one has no sign-in, so the check warns about that.

## Testing with curl

These commands repeat the server check by hand. Replace `SERVER` with your address.

```sh
SERVER=http://192.168.1.20:8080

# API present? Expect JSON with name and auth.
curl -i $SERVER/api/info

# Sign-in endpoint present? Expect 401.
curl -i -X POST $SERVER/api/login -H 'Content-Type: application/json' \
     -d '{"username":"","password":""}'

# Library protected? With auth on, expect 401.
curl -i $SERVER/api/library

# Sign in and keep the token.
TOKEN=$(curl -s -X POST $SERVER/api/login -H 'Content-Type: application/json' \
        -d '{"username":"noah","password":"hunter2"}' | python3 -c 'import json,sys; print(json.load(sys.stdin)["token"])')

# Library loads? Expect JSON.
curl -s -H "Authorization: Bearer $TOKEN" $SERVER/api/library | python3 -m json.tool | head -40

# Streams support Range? Expect "206 Partial Content" and a Content-Range header.
curl -s -o /dev/null -D - -H "Authorization: Bearer $TOKEN" -H 'Range: bytes=0-1' \
     $SERVER/api/stream/movie/42
```

## Troubleshooting

| Check says | Fix |
|---|---|
| *No answer from …* | Check the IP and port. Bind the server to `0.0.0.0`, not `127.0.0.1`, so other machines can reach it. Open the port in the firewall (`sudo ufw allow 8080/tcp`). |
| *…doesn't share a folder listing either* | Add `/api/info`, or turn on directory listings. If you added the API, check that the app isn't behind a path prefix the user didn't type. |
| *It answered, but not with JSON* | A web app is serving its HTML page for every unknown path. Register the API routes before the catch-all route. |
| Sign-in API *404/405* | Add `POST /api/login`, or return `"auth": false` from `/api/info` if the library doesn't need to be private. |
| Sign-in API *signed in with an empty username* | Return `401` for wrong credentials instead of `200`. |
| Library API *anyone can read the library* | Put `/api/library`, streams and images behind the token check. |
| Library *the server turned down the sign-in token* | Accept `Authorization: Bearer <token>` on `/api/library`, not only cookies. |
| Library *a library this app doesn't understand* | The error names the field and line. Usually a number sent as a string (`"year": "2010"`) or an object where a list is expected. |
| Movies *no "stream" URL* | Every movie needs `stream`. Movies without it are hidden. |
| Streaming *sent text/html instead of a video* | The stream URL redirects to a login page, or isn't checking the bearer token. |
| Streaming *ignores Range requests* | Serve the file with a Range-aware helper (see [Streams and images](#streams-and-images)). `python -m http.server` can't do this. Use the bundled server or nginx. |
| Artwork *Couldn't load …* | The image URL needs the same bearer token as the API, or it must be public. Use JPEG, PNG or WebP. |
| Metadata bars | Add the missing fields to the library JSON (or to `info.json` with the bundled server). Each one shows up on the details pages as soon as it's there. |
