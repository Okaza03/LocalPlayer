# Library editing

[← Back to the Admin API](admin.md)

The Admin page's _Library_ section lets an admin fix a title by hand (title, year, overview, genres, director or creators, age rating, score, IMDb id, poster, backdrop, thumbnail, logo) or fetch its details from IMDb and check them before saving. The title list and the "missing something" markers work without any of this. Editing needs the endpoints below; when the server answers `404`, `405` or `501`, the section says _Not set up_ and links here.

## URLs

`<kind>` is `movies` or `series`. `<id>` is the item's `id` from [`/api/library`](library.md), percent-encoded as one path segment (`m:Movies/Heat (1995)/Heat.mkv` becomes `m%3AMovies%2FHeat%20%281995%29%2FHeat.mkv`). Decode it after splitting the path on `/`.

| Request                                       | Body                          | Answer                                                      |
| --------------------------------------------- | ----------------------------- | ----------------------------------------------------------- |
| `PATCH /api/admin/<kind>/<id>`                | changed fields                | `200`; the app reloads the library                          |
| `POST /api/admin/<kind>/<id>/fetch`           | `{"title", "year", "imdb_id"}` | details found, library-shaped; **nothing is saved**        |
| `PUT /api/admin/<kind>/<id>/art/<which>`      | the image file                | `200`. `<which>` is `poster`, `backdrop`, `thumb` or `logo`          |

Admins only, under the [usual rules](admin.md#rules-for-every-request). Errors with a short message (`{"error": "…"}` or `{"message": "…"}`) are shown to the admin.

## `PATCH /api/admin/<kind>/<id>`

Only the fields the admin changed, with the [library's names](library.md#movie):

```json
{
  "title": "Batman",
  "year": 1989,
  "overview": "The Dark Knight of Gotham City begins his war on crime…",
  "genres": ["Action", "Crime"],
  "director": "Tim Burton",
  "score": 7.5,
  "imdb_id": "tt0096895",
  "poster": "https://m.media-amazon.com/images/M/….jpg",
  "rating": null
}
```

- `null` clears a field.
- Movies may send `title`, `year`, `overview`, `genres`, `rating`, `director`, `released`, `score`, `imdb_id`, `poster`, `backdrop`, `thumb`, `logo`.
- Series may send the same, with `creators` (a list) instead of `director`, plus `network`, `status`, `end_year`.
- `poster`, `backdrop`, `thumb` and `logo` are image addresses the admin pasted (or that a fetch found). **Download them** and serve them like the rest of your artwork; don't hand the remote URL to every client. They're only sent when the admin changed them, so the current artwork's URL never comes back. `null` means the admin removed it: delete the image.

Store the edits so a rescan or a sync doesn't overwrite them (for example, mark edited fields as locked, or keep edits in their own table that wins over scanned data).

## `POST /api/admin/<kind>/<id>/fetch`

The _Fetch from IMDb_ button.

```json
{ "title": "Batman", "year": 1989, "imdb_id": "tt0096895" }
```

`imdb_id` is sent when the admin filled it in; look the title up by it then, else by `title` and `year`. Answer with what you found, in the library's shape:

```json
{
  "title": "Batman",
  "year": 1989,
  "overview": "The Dark Knight of Gotham City begins his war on crime…",
  "genres": ["Action", "Adventure"],
  "rating": "PG-13",
  "director": "Tim Burton",
  "released": "1989-06-23",
  "score": 7.5,
  "score_source": "IMDb",
  "imdb_id": "tt0096895",
  "poster": "https://m.media-amazon.com/images/M/….jpg"
}
```

The app fills the editor with every field that isn't empty and the admin saves with `PATCH`, so this endpoint must not change anything. `422` with a message when nothing was found (`{"error": "Movie not found!"}`); `501` when lookups aren't configured (the app then hides the button behind a link here).

IMDb itself has no public API. Ways to get its data:

- [OMDb](https://www.omdbapi.com/apikey.aspx): IMDb's data, free key (1,000 requests a day). The reference server uses it: `--omdb-key` or `OMDB_API_KEY`.
- [TMDB](https://developer.themoviedb.org/docs): free key, better posters and backdrops, has the IMDb id (`/find/{imdb_id}?external_source=imdb_id`).
- [IMDb's datasets](https://developer.imdb.com/non-commercial-datasets/): daily TSV files, no images.

## `PUT /api/admin/<kind>/<id>/art/<which>`

The _Upload…_ buttons. The body is the image file, with `Content-Type: image/jpeg`, `image/png` or `image/webp`, up to about 25 MB. Replace the old image (and any other format of it, so a stale `poster.jpg` doesn't win over the new `poster.png`). A logo should be a PNG with a transparent background.

Afterwards the app reloads the library and downloads the image again, even if its URL didn't change.

## Reference server

Edits go into the title's `info.json` (the movie's own folder, or `<file>.json` for a loose file; the show's folder for a series). Artwork is saved as `poster.*`, `fanart.*`, `thumb.*` and `logo.png` in the folder (`<file>-poster.*`, `<file>-fanart.*` and `<file>-thumb.*` for a loose movie file, which can't have a logo).

## Laravel example

```php
Route::middleware(['auth:sanctum', 'can:admin'])->prefix('admin')->group(function () {
    Route::patch('/movies/{movie}', function (Request $r, Movie $movie) {
        $data = $r->only(['title', 'year', 'overview', 'genres', 'rating', 'director', 'released', 'score', 'imdb_id']);
        foreach (['poster', 'backdrop', 'logo'] as $art) {
            if ($url = $r->input($art)) {
                $movie->{$art.'_path'} = Storage::disk('public')->putFile('art', Http::get($url)->toPsrResponse()->getBody());
            }
        }
        $movie->fill($data)->forceFill(['locked' => array_keys($data)])->save();
        return [];
    });
    Route::post('/movies/{movie}/fetch', function (Request $r, Movie $movie) {
        $q = $r->filled('imdb_id') ? ['i' => $r->imdb_id] : ['t' => $movie->title, 'y' => $movie->year, 'type' => 'movie'];
        $d = Http::get('https://www.omdbapi.com/', $q + ['apikey' => config('services.omdb.key'), 'plot' => 'full'])->json();
        abort_if(($d['Response'] ?? '') !== 'True', 422, $d['Error'] ?? 'Not found');
        return ['title' => $d['Title'], 'year' => (int) $d['Year'], 'overview' => $d['Plot'], 'rating' => $d['Rated'],
                'genres' => array_map('trim', explode(',', $d['Genre'])), 'director' => $d['Director'],
                'imdb_id' => $d['imdbID'], 'score' => (float) $d['imdbRating'], 'poster' => $d['Poster']];
    });
    Route::put('/movies/{movie}/art/{which}', function (Request $r, Movie $movie, string $which) {
        abort_unless(in_array($which, ['poster', 'backdrop', 'logo']), 404);
        $path = 'art/'.Str::uuid().'.'.(['image/png' => 'png', 'image/webp' => 'webp'][$r->header('Content-Type')] ?? 'jpg');
        Storage::disk('public')->put($path, $r->getContent());
        $movie->update([$which.'_path' => $path]);
        return [];
    });
    // the same three for /series/{series}
});
```
