# Examples

[← Back to the API guide](README.md)

Minimal servers in three frameworks. They implement the endpoints described in [Sign-in](auth.md), [The library](library.md), [Watch progress](progress.md) and [Streams and images](streams.md).

## Laravel

```php
// routes/api.php
Route::get('/info', fn () => ['name' => config('app.name'), 'auth' => true, 'version' => 1]);

Route::post('/login', function (Request $r) {
    if (!Auth::attempt($r->only('username', 'password'))) {
        return response()->json(['error' => 'wrong username or password'], 401);
    }
    return [
        'token' => $r->user()->createToken('valo')->plainTextToken,
        'name'  => $r->user()->name,
        'admin' => (bool) $r->user()->is_admin,
    ];
});

Route::middleware('auth:sanctum')->group(function () {
    Route::post('/logout', fn (Request $r) => $r->user()->currentAccessToken()->delete());
    Route::get('/library', [ValoController::class, 'library']);           // builds the library JSON
    Route::get('/progress', [ProgressController::class, 'index']);        // optional: watch progress sync
    Route::post('/progress', [ProgressController::class, 'store']);
    Route::get('/stream/{type}/{id}', [ValoController::class, 'stream']); // return response()->file($path)
    Route::get('/me', fn (Request $r) => [                                // optional: picks up admin changes
        'name' => $r->user()->name, 'admin' => (bool) $r->user()->is_admin,
    ]);
});
```

For the app's Admin page, add the routes from the [Admin API Laravel example](admin.md#laravel-example).

`response()->file()` (Symfony's `BinaryFileResponse`) handles range requests on its own. Make sure unauthenticated API requests get a JSON `401` and not a redirect to a login page. Sanctum does this when the request has `Accept: application/json`, or you can render `AuthenticationException` as JSON for `api/*`.

## Express (Node.js)

```js
const express = require("express");
const crypto = require("crypto");
const app = express();
app.use(express.json());

const users = { user: "hunter2" }; // use real password hashing
const tokens = new Map(); // token -> username
const movies = [
  {
    id: "1",
    title: "Inception",
    year: 2010,
    file: "/srv/videos/Inception.mkv",
    poster: "/srv/videos/Inception.jpg",
  },
];

app.get("/api/info", (req, res) =>
  res.json({ name: "Node server", auth: true, version: 1 }),
);

app.post("/api/login", (req, res) => {
  const { username, password } = req.body || {};
  if (!username || users[username] !== password)
    return res.status(401).json({ error: "wrong" });
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

app.get("/api/library", auth, (req, res) =>
  res.json({
    movies: movies.map((m) => ({
      id: m.id,
      title: m.title,
      year: m.year,
      poster: `/art/movie/${m.id}`,
      stream: `/api/stream/movie/${m.id}`,
    })),
    series: [],
  }),
);

const byId = (id) => movies.find((m) => m.id === id);
app.get("/api/stream/movie/:id", auth, (req, res) =>
  res.sendFile(byId(req.params.id).file),
); // Range-aware
app.get("/art/movie/:id", auth, (req, res) =>
  res.sendFile(byId(req.params.id).poster),
);

app.listen(8080, "0.0.0.0");
```

## Flask (Python)

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
