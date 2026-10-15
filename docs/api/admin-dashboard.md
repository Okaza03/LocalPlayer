# Dashboard

[← Back to the Admin API](admin.md)

The first section of the app's Admin page. Some of it works with no admin endpoints at all; the rest needs the endpoints below.

| Part              | Needs                                                                                     |
| ----------------- | ----------------------------------------------------------------------------------------- |
| Numbers (movies, series, episodes, size, users) | `library` in [`/api/admin/status`](#get-apiadminstatus). Without it, the app counts [`/api/admin/movies`](admin-library.md) (or `/api/library`) and [`/api/admin/users`](admin.md) |
| Library health    | `health` in `/api/admin/status`: your checks **replace** the app's, so it matches your website. Without them, the app checks for a missing poster, backdrop, overview, year and genres |
| API status        | [`GET /api/admin/status`](#get-apiadminstatus) for response time, version, uptime and services |
| Sync operations   | [`POST /api/admin/sync/<what>`](#post-apiadminsyncwhat), plus [`/api/admin/rescan`](admin.md#post-apiadminrescan) |
| Live log          | [`GET /api/admin/logs`](#get-apiadminlogs)                                                |

All of them follow the [rules for every admin request](admin.md#rules-for-every-request). `404`, `405` or `501` means "not set up", and the app links here.

## `GET /api/admin/status`

How the server and what it depends on are doing. The app asks every 30 seconds while the Dashboard is open and shows how long the answer took.

```json
{
  "version": "1.4.0",
  "uptime": 271800,
  "library": { "movies": 412, "series": 38, "episodes": 2210, "size": 5832110000000, "users": 6 },
  "services": [
    { "name": "Database", "ok": true, "detail": "PostgreSQL 16" },
    { "name": "IMDb lookups (OMDb)", "ok": true, "detail": "API key set" },
    { "name": "Discord bot", "ok": false, "detail": "last seen 2 hours ago" }
  ],
  "health": [
    { "label": "Files without a database entry", "count": 2, "items": ["/media/Movies/New (2024)/New.mkv"] },
    { "label": "Missing poster", "count": 1, "titles": [{ "type": "movie", "id": 12, "title": "Heat", "year": 1995 }] },
    { "label": "Database entries without a file", "count": 0 }
  ]
}
```

| Field      | Type     |                                                                                     |
| ---------- | -------- | ----------------------------------------------------------------------------------- |
| `version`  | string   | Your server's version                                                               |
| `uptime`   | int      | Seconds since it started                                                            |
| `library`  | object   | Your own numbers: `movies`, `series`, `episodes`, `size` (bytes), `users`. Each one is optional. **Any other number** in it becomes one more tile, labelled from its key: `"mp4": 271` shows as _MP4_, `"series_complete": 17` as _Series complete_ |
| `services` | array    | Anything it depends on: `name`, `ok` (green or red dot), optional `detail`         |
| `health`   | array    | Your library checks, in place of the app's: `label`, `count`, and what to list when the row is opened: `titles` (`{type, id, title, year}`, clickable: they open in the Library editor) and/or `items` (plain text, such as file paths). Send up to about 100 |

Every field is optional; `{}` is a valid answer. Good services to report: the database, the metadata source behind [Library editing](admin-library.md), the [Discord bot](admin-requests.md) (when it last called), free disk space, a transcoder.

## `POST /api/admin/sync/<what>`

The three sync buttons. `<what>` is one of:

| `<what>`  | Button       | What the server does                                                                                           |
| --------- | ------------ | -------------------------------------------------------------------------------------------------------------- |
| `folders` | Sync folders | Scans the media directories and compares them against the database. Lists new and removed files; **writes nothing** |
| `movies`  | Sync movies  | Adds new movie files to the database, removes movies whose files are gone, fetches details for the new ones    |
| `series`  | Sync series  | The same for series: new shows, seasons and episodes in, missing ones out, details for the new ones            |

Answer:

```json
{
  "added": ["/media/Movies/Dune Part Two (2024)/Dune.Part.Two.mkv"],
  "removed": ["/media/Movies/Old (1999)/Old.avi"],
  "updated": [],
  "applied": true,
  "running": false,
  "message": "1 new movie, 1 removed"
}
```

| Field     | Type     |                                                                                                  |
| --------- | -------- | ------------------------------------------------------------------------------------------------ |
| `added`   | string[] | New files (or titles). Shown in green                                                            |
| `removed` | string[] | Files that are gone. Shown in red                                                                |
| `updated` | string[] | Entries that changed                                                                             |
| `applied` | bool     | `false` for `folders` (a dry run); `true` when the database was changed. The app then reloads the library |
| `running` | bool     | `true` when the work carries on in the background. The app points the admin at the live log      |
| `message` | string   | One line shown above the lists                                                                   |

### Applying the folder sync

After a `folders` dry run that found changes, the app offers _Apply these changes_ (with a confirmation, since removed files lose their entries and watch progress). It sends `POST /api/admin/sync/folders?apply=1` and expects the same answer with `"applied": true`. Without `apply=1`, `folders` must never write anything.

The app waits up to 10 minutes for an answer. For anything slower, start the work, answer `{"running": true}` at once, and log progress to the [live log](#get-apiadminlogs).

`/api/admin/rescan` from the [Admin API](admin.md#post-apiadminrescan) shows up as a fourth button, _Rescan library_.

## `GET /api/admin/logs`

The live log. The app asks every 3 seconds while the Dashboard is open, with the last `id` it has:

```
GET /api/admin/logs?after=1041
```

```json
[
  { "id": 1042, "time": 1792103422, "level": "info", "message": "sync movies: 1 new, 0 removed" },
  { "id": 1043, "time": 1792103425, "level": "error", "message": "OMDb: request limit reached" }
]
```

| Field     | Type   |                                                                     |
| --------- | ------ | ------------------------------------------------------------------- |
| `id`      | int    | **required**: grows with every line                                 |
| `time`    | int    | Unix time                                                           |
| `level`   | string | `"debug"`, `"info"`, `"warning"` or `"error"`; colors the line      |
| `message` | string | One line of text                                                    |

Return only lines with an `id` greater than `after`, oldest first. With `after=0` (the first request), return the last 200 or so. Keep a ring buffer of the last thousand lines in memory; that's enough.

Don't log the log requests themselves, or the feed fills with its own polling. Never log passwords or tokens: admins see everything here.

## Laravel example

```php
Route::middleware(['auth:sanctum', 'can:admin'])->prefix('admin')->group(function () {
    Route::get('/status', fn () => [
        'version' => config('app.version'),
        'uptime' => now()->timestamp - (int) Cache::get('started_at', now()->timestamp),
        'services' => [
            ['name' => 'Database', 'ok' => rescue(fn () => DB::select('select 1') !== null, false)],
            ['name' => 'Discord bot', 'ok' => Cache::get('bot_seen', 0) > now()->subMinutes(10)->timestamp],
        ],
    ]);
    Route::post('/sync/{what}', function (string $what) {
        abort_unless(in_array($what, ['folders', 'movies', 'series']), 404);
        $r = app(LibrarySync::class)->run($what, dryRun: $what === 'folders');   // your scanner
        return ['added' => $r->added, 'removed' => $r->removed, 'applied' => $what !== 'folders'];
    });
    Route::get('/logs', fn (Request $r) => LogLine::where('id', '>', (int) $r->query('after', 0))
        ->orderBy('id')->limit(200)->get(['id', 'level', 'message', DB::raw('extract(epoch from created_at)::int as time')]));
});
```
