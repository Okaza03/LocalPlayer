# Analytics

[← Back to the Admin API](admin.md)

The Admin page's _Analytics_ section: plays per day, the most watched titles, and how much each user watched, over 7, 30 or 90 days or a year. It needs one endpoint. Without it, the section says _Not set up_ and links here.

## `GET /api/admin/analytics?days=30`

`days` is `7`, `30`, `90` or `365`. Admins only, under the [usual rules](admin.md#rules-for-every-request).

```json
{
  "plays": 128,
  "hours": 214.5,
  "active_users": 5,
  "days": [
    { "day": "2026-09-17", "plays": 3, "hours": 4.2 },
    { "day": "2026-09-18", "plays": 0, "hours": 0 }
  ],
  "top": [
    { "title": "The Office", "kind": "series", "plays": 41, "hours": 15.3 },
    { "title": "Inception", "kind": "movie", "plays": 4, "hours": 9.6 }
  ],
  "users": [
    { "name": "noah", "plays": 60, "hours": 101.2, "last_active": 1792103422 }
  ]
}
```

| Field          | Type   |                                                                                         |
| -------------- | ------ | --------------------------------------------------------------------------------------- |
| `plays`        | int    | Plays in the range                                                                      |
| `hours`        | number | Hours watched in the range                                                              |
| `active_users` | int    | Users who watched anything in the range                                                 |
| `days`         | array  | One entry per day of the range, **oldest first**, days without plays included: `day` (`YYYY-MM-DD`), `plays`, `hours` |
| `top`          | array  | Most played first, about 20: `title`, `kind` (`"movie"`/`"series"`), `plays`, `hours`. Episodes count toward their series |
| `users`        | array  | `name`, `plays`, `hours`, `last_active` (unix time)                                     |

Every field is optional; missing numbers show as 0 and missing lists as empty.

## Where the numbers come from

The simplest source is what you already store for [watch progress](progress.md): every saved entry is a play, its `updated` time says when, and its `position` says how much was watched. That's what the reference server does. It undercounts rewatches (a second viewing overwrites the first entry), which is fine for an overview.

For exact numbers, also write a row to a `plays` table each time `POST /api/progress` starts a new viewing (no entry yet, or the old one was finished or older than a few hours), and count those rows.

## Laravel example

```php
Route::get('/admin/analytics', function (Request $r) {
    $days = min(max((int) $r->query('days', 30), 1), 366);
    $since = now()->subDays($days - 1)->startOfDay();
    $rows = Progress::where('updated_at', '>=', $since)->with('user', 'item')->get();
    $hours = fn ($c) => round($c->sum(fn ($p) => min($p->position, $p->duration ?: $p->position)) / 3600, 1);
    return [
        'plays' => $rows->count(),
        'hours' => $hours($rows),
        'active_users' => $rows->pluck('user_id')->unique()->count(),
        'days' => collect(range(0, $days - 1))->map(function ($n) use ($since, $rows, $hours) {
            $day = $since->copy()->addDays($n)->toDateString();
            $on = $rows->filter(fn ($p) => $p->updated_at->toDateString() === $day);
            return ['day' => $day, 'plays' => $on->count(), 'hours' => $hours($on)];
        }),
        'top' => $rows->groupBy(fn ($p) => $p->item->seriesTitle() ?? $p->item->title)
            ->map(fn ($g, $t) => ['title' => $t, 'plays' => $g->count(), 'hours' => $hours($g)])
            ->sortByDesc('plays')->values()->take(20),
        'users' => $rows->groupBy('user.username')->map(fn ($g, $n) => [
            'name' => $n, 'plays' => $g->count(), 'hours' => $hours($g), 'last_active' => $g->max('updated_at')->timestamp,
        ])->values(),
    ];
})->middleware(['auth:sanctum', 'can:admin']);
```
