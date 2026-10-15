# Watch progress

[← Back to the API guide](README.md)

With these two endpoints, resume points and watched marks follow the user between the app on every device and your website. Both are optional. Without them, progress stays on each device.

## `GET /api/progress`: optional

The signed-in user's watch progress, most recently watched first.

```json
[
  {
    "type": "episode",
    "id": "42",
    "position": 812.5,
    "duration": 2640,
    "percent": 31,
    "completed": false,
    "updated": 1760400000
  }
]
```

| Field       | Type          |                                                                                                             |
| ----------- | ------------- | ----------------------------------------------------------------------------------------------------------- |
| `type`      | string        | `"movie"` or `"episode"`                                                                                    |
| `id`        | string or int | The `id` the [library](library.md) gives that movie or episode                                              |
| `position`  | number        | Seconds                                                                                                     |
| `duration`  | number        | Seconds, `0` if unknown                                                                                     |
| `completed` | bool          | Watched at least 95% (the app counts that as finished). The app shows it as watched whatever the position. |
| `updated`   | int           | Unix time of the last change, set by the server                                                             |
| `percent`   | int           | Ignored by the app                                                                                          |

The app asks for this on start, every minute while nothing is playing, and on _Refresh_. When an entry here is newer than the app's own copy, the server's copy wins. Answer `401` when there's no signed-in user (for example with sign-in off). `401`, `404`, `405` or `501` turn syncing off for the session, and progress stays on the device.

## `POST /api/progress`: optional

Saves one entry. The app sends it every 15 seconds while something plays, when the player closes, and on quit. Changes made offline are sent once the server answers again.

```json
{ "type": "movie", "id": 123, "position": 812.5, "duration": 2640 }
```

Ids that are whole numbers are sent as numbers, others as strings. Store one entry per user and item, set `updated` to the current time, and answer with the saved entry in the `GET` shape. Answer `404` for an unknown id and `422` for an invalid body; the app drops that change instead of retrying it. `"position": 0` means the user cleared their watch history.
