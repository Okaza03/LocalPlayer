# Transcode

[← Back to the Admin API](admin.md)

The Admin page's _Transcode_ section lists video files that don't play smoothly everywhere (MKV, HEVC, DTS audio, …) and lets the admin convert them to MP4, one or all, with a live log. Your server decides which files need it and does the converting (for example with FFmpeg); the app only shows and starts it. Without `GET /api/admin/transcode`, the section says _Not set up_ and links here.

Admins only, under the [usual rules](admin.md#rules-for-every-request).

| Request                                   | Body                                     | Answer                                                        |
| ----------------------------------------- | ---------------------------------------- | ------------------------------------------------------------- |
| `GET /api/admin/transcode`                |                                          | the files and the queue, below                                |
| `POST /api/admin/transcode/scan`          |                                          | `200` or `202`: look through the library again                |
| `POST /api/admin/transcode`               | `{"keys": ["…"]}` or `{"all": true}`     | `202`: queue those files (all: every idle or failed one)      |
| `DELETE /api/admin/transcode/queue`       |                                          | `200`: drop the queued files; the one converting now finishes |
| `GET /api/admin/transcode/log?after=<id>` |                                          | log lines, the same shape as [`/api/admin/logs`](admin-dashboard.md#get-apiadminlogs) |

## `GET /api/admin/transcode`

```json
{
  "running": true,
  "files": [
    { "key": "movie:15", "title": "A Working Man (2025)", "kind": "movie", "size": 5568407824,
      "container": "mkv", "video_codec": "hevc", "audio_codec": "eac3", "status": "running", "progress": 0.42 },
    { "key": "episode:301", "title": "Arcane S01E01", "kind": "series", "container": "mkv", "status": "queued" },
    { "key": "movie:22", "title": "Heat (1995)", "status": "failed", "error": "ffmpeg exited with 1" }
  ]
}
```

| Field      | Type   |                                                                                               |
| ---------- | ------ | --------------------------------------------------------------------------------------------- |
| `running`  | bool   | Something is converting. The app then asks every 3 seconds (else every 30)                    |
| `files`    | array  | Every file that needs or had converting                                                       |
| `key`      | string | **required**: your id for the file, sent back in `keys`                                       |
| `title`    | string | **required**: what to show ("Arcane S01E01")                                                  |
| `status`   | string | `"idle"`, `"queued"`, `"running"`, `"done"` or `"failed"`                                     |
| `progress` | number | `0` to `1` while running                                                                      |
| `kind`, `size`, `container`, `video_codec`, `audio_codec`, `error` | | Optional details shown under the title                            |

The app has _To convert_, _Done_ and _All_ tabs, a _Convert_ button on each idle or failed file, _Convert all_, _Cancel queue_ and _Scan again_.

## Notes

- Convert one file at a time; video encoding uses all the CPU it gets.
- Replace the original only after the new file is complete, and keep its watch progress and metadata (same library id).
- Write each file's start, finish and failure to the transcode log, and to the main live log if you like.
