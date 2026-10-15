# Troubleshooting

[← Back to the API guide](README.md)

What to do about each message on the [server check](server-check.md).

| Check says                                                    | Fix                                                                                                                                                               |
| ------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| _No answer from …_                                            | Check the IP and port. Bind the server to `0.0.0.0`, not `127.0.0.1`, so other machines can reach it. Open the port in the firewall (`sudo ufw allow 8080/tcp`).  |
| _…doesn't share a folder listing either_                      | Add `/api/info`, or turn on directory listings. If you added the API, check that the app isn't behind a path prefix the user didn't type.                         |
| _It answered, but not with JSON_                              | A web app is serving its HTML page for every unknown path. Register the API routes before the catch-all route.                                                    |
| Sign-in API _404/405_                                         | Add `POST /api/login`, or return `"auth": false` from `/api/info` if the library doesn't need to be private. See [Sign-in](auth.md).                              |
| Sign-in API _signed in with an empty username_                | Return `401` for wrong credentials instead of `200`.                                                                                                              |
| Library API _anyone can read the library_                     | Put `/api/library`, streams and images behind the token check.                                                                                                    |
| Library _the server turned down the sign-in token_            | Accept `Authorization: Bearer <token>` on `/api/library`, not only cookies.                                                                                       |
| Library _a library this app doesn't understand_               | The error names the field and line. Usually a number sent as a string (`"year": "2010"`) or an object where a list is expected. See [The library](library.md).    |
| Movies _no "stream" URL_                                      | Every movie needs `stream`. Movies without it are hidden.                                                                                                         |
| Streaming _sent text/html instead of a video_                 | The stream URL redirects to a login page, or isn't checking the bearer token.                                                                                     |
| Streaming _ignores Range requests_                            | Serve the file with a Range-aware helper (see [Streams and images](streams.md)). `python -m http.server` can't do this. Use the bundled server or nginx.          |
| Artwork _Couldn't load …_                                     | The image URL needs the same bearer token as the API, or it must be public. Use JPEG, PNG or WebP.                                                                |
| Progress sync _don't match a movie or episode in the library_ | `id` in `/api/progress` must be the same `id` the library gives that movie or episode, and `type` must be `"movie"` or `"episode"`.                               |
| Progress sync _not a list the app can read_                   | Return a JSON array in the shape under [`GET /api/progress`](progress.md#get-apiprogress-optional). Send `0` for an unknown `duration`.                            |
| Progress sync _accepted an entry with "type": "valo-check"_   | Validate the body and answer `422` when it can't be stored.                                                                                                       |
| Admin API _the server has no admin API_                       | Add the routes in [Admin API](admin.md). Users are named by username in the URL, not by id.                                                                       |
| Admin API _the server says this account isn't an admin_       | The admin middleware must accept bearer tokens, not only a web session. Check the user is an admin in the database, then sign out and back in.                    |
| Metadata bars                                                 | Add the missing fields to the library JSON (or to `info.json` with the bundled server). Each one shows up on the details pages as soon as it's there.             |
