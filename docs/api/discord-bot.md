# Valo Discord bot: invite, set up, or run your own

[← Back to Requests and the Discord bot](admin-requests.md)

The Valo bot lets members of a Discord server request movies and series with `/film` and `/serie`.
Each request goes to **that server's own Valo media server**, where an admin approves, denies or marks
it available. The requester gets a DM when it is ready or declined.

One bot can serve many Discord servers. Every server keeps its own settings (request channel + Valo
server address + bot secret), so nobody needs access to anyone else's media server.

- [1. Invite the bot to a Discord server](#1-invite-the-bot-to-a-discord-server)
- [2. Set it up (server admins)](#2-set-it-up-server-admins)
- [3. What your Valo server needs](#3-what-your-valo-server-needs)
- [4. Commands](#4-commands)
- [5. Limits of shared use](#5-limits-of-shared-use)
- [6. Make the bot invitable by others (bot owner)](#6-make-the-bot-invitable-by-others-bot-owner)
- [7. Run your own bot instead](#7-run-your-own-bot-instead)
- [Troubleshooting](#troubleshooting)

---

## 1. Invite the bot to a Discord server

You need the **Manage Server** permission on the Discord server you add it to.

Open this link, pick your server, and press **Authorize**:

```
https://discord.com/oauth2/authorize?client_id=1276156465446654074&permissions=19456&scope=bot+applications.commands
```

What it asks for:

| Permission                    | Why                                         |
| ----------------------------- | ------------------------------------------- |
| View Channels                 | see the request channel                     |
| Send Messages                 | post a request in the request channel       |
| Embed Links                   | the request card (title, poster, requester) |
| `applications.commands` scope | the `/film`, `/serie`, `/setup` … commands  |

It never needs "Administrator" and does not read messages.

If the link says _"Private application cannot have a default authorization link"_ or _"integration
requires code grant"_, the bot owner has not made the bot public yet (see [section 6](#6-make-the-bot-invitable-by-others-bot-owner)).

## 2. Set it up (server admins)

After the bot joins, it posts a short welcome. Nothing works for members until an admin finishes setup.

1. Run **`/setup`**. It shows a checklist of what is missing (visible only to you).
2. **Pick the request channel**: `/settings channel #requests`.
   The bot checks it can view the channel, send messages and embed links there. Make the channel
   visible only to your staff if you don't want everyone to see open requests.
3. **Connect your Valo server**: `/settings server`. A private pop-up asks for:
   - **Server address**: where your Valo server is reachable on the internet, for example
     `https://media.example.com` (a trailing `/api` is fine).
   - **Bot secret**: the `BOT_API_SECRET` value from your Valo server's `.env`
     (see [section 3](#3-what-your-valo-server-needs)).

   The bot tests the connection before saving. If it fails, nothing is saved and the message says why.
4. Optional: `/settings test` re-tests the connection later, `/settings view` shows everything
   (the secret is masked).

When all three lines in `/setup` are ticked, members can use `/film` and `/serie`.

Settings are stored per Discord server in the bot's database. Only people with **Manage Server** can
change them. The secret is entered in a pop-up so it never appears in chat.

## 3. What your Valo server needs

- A running Valo server with the request API ([Requests and the Discord bot](admin-requests.md#1-the-server-endpoints)),
  reachable from the internet over HTTPS. The bot cannot reach private addresses such as `192.168.x.x`,
  `10.x.x.x` or `localhost`.
- A bot secret: add a long random value to the server's `.env` (or its settings) and restart it:

  ```
  BOT_API_SECRET=put-a-long-random-string-here      # e.g. output of: openssl rand -hex 32
  ```

  The bundled reference server takes it as `--bot-token` or `VALO_BOT_TOKEN`.

- TMDB is used by the bot for the title picker, so no key is needed on your side for that.
- The server exposes the bot routes (`POST/GET/PATCH/DELETE /api/requests`, optionally `POST /api/log`) and
  `GET /api/info`. The contract is in [Requests and the Discord bot](admin-requests.md#for-the-bot).

How a request flows:

```
member: /film Inception ──► bot ──► POST /api/requests ──► Valo server (status: pending)
admin approves / denies / marks available in the Valo app
bot polls GET /api/requests?updated_after=… every 30 s
   ├─ available ► deletes the request card, DMs the member "Added – Watch now"
   └─ denied    ► deletes the request card, DMs the member the admin's note
```

## 4. Commands

| Command             | Who           | What it does                                                   |
| ------------------- | ------------- | -------------------------------------------------------------- |
| `/film title:`      | everyone      | Request a movie (pick the right match from TMDB)               |
| `/serie title:`     | everyone      | Request a series                                               |
| `/setup`            | Manage Server | Setup guide and checklist                                      |
| `/settings view`    | Manage Server | Show channel, server address, masked secret, connection state  |
| `/settings channel` | Manage Server | Choose the request channel                                     |
| `/settings server`  | Manage Server | Enter the Valo server address and bot secret (private pop-up)  |
| `/settings test`    | Manage Server | Test the connection now                                        |
| `/settings reset`   | Manage Server | Forget this server's settings                                  |
| `/clear`            | Manage Server | Remove all open requests and their cards                       |

## 5. Limits of shared use

- **Your server must be public.** The bot can't reach a media server that is only on your home network.
  Put it behind HTTPS (a reverse proxy, a tunnel) or run [your own bot](#7-run-your-own-bot-instead) on
  the same network.
- **Removing a request in the Valo app doesn't remove its Discord card** for servers other than the bot
  owner's. The bot can only see requests that still exist. Use **deny** or **mark available** (the card
  is removed and the member notified), or `/clear`.
- **No file checks.** The "already on the disk" shortcut and file renaming only work for the bot owner's
  own server, which has its media folders mounted. Other servers always create a normal request.
- **The bot owner can see** which Discord servers use the bot and the stored settings (including the
  secrets) in the bot's database. Only add the bot if you trust whoever runs it, or run your own.
- Members get DMs. If they have DMs closed, the request is still resolved, and the Valo server records
  `notified: false` for it.

## 6. Make the bot invitable by others (bot owner)

1. Go to <https://discord.com/developers/applications> and open the application.
2. **Installation** (or **OAuth2**): make sure _Install Link_ is **Discord Provided Link** or **None**
   and the scopes `bot` + `applications.commands` are enabled with the permissions above.
3. **Bot** → turn **Public Bot** on. Leave **Requires OAuth2 Code Grant** off.
4. Leave the three Privileged Gateway Intents **off**: the bot only uses slash commands.
5. The commands are global. After changing the command list, run once:

   ```sh
   docker exec media-bot node register-commands.js
   ```

   New global commands can take a few minutes to appear everywhere.
6. Persist the data: `docker-compose.yml` mounts `./data:/app/data`. That folder holds every server's
   settings; back it up.

Discord verification is only required once the bot is in 100 or more servers. Since it uses no
privileged intents, verification is a simple identity check.

## 7. Run your own bot instead

If you would rather not depend on someone else's bot (or your media server is on a private network), run
your own copy. It is the same code; you just give it your own bot account. For a smaller, single-file
alternative in Python, see [the bot in the requests guide](admin-requests.md#3-run-the-bot).

### Create the Discord application

1. <https://discord.com/developers/applications> → **New Application** → name it (for example "Valo").
2. **Bot** → **Reset Token** → copy the token. Keep it secret.
3. Note the **Application ID** (General Information → _Application ID_). This is `CLIENT_ID`.
4. Invite it with the link from section 1, replacing `client_id=` with your Application ID.

### Run the bot

Requirements: Node.js 20+ (or Docker). Files: `server.js`, `guilds.js`, `db.js`, `tmdb.js`,
`register-commands.js`, `rename-*.js`, `package.json`.

`.env`:

```
DISCORD_TOKEN=your-bot-token
CLIENT_ID=your-application-id
TMDB_API_KEY=your-tmdb-api-key        # free at themoviedb.org

# Optional: pre-configure ONE server so you can skip /setup
GUILD_ID=your-discord-server-id
REQUEST_CHANNEL_ID=your-request-channel-id
LARAVEL_URL=https://media.example.com   # or http://host.docker.internal if on the same machine
BOT_API_SECRET=same-value-as-on-your-valo-server
APP_URL=https://media.example.com       # used for the "Watch now" link
ANNOUNCE_CHANNEL_ID=your-announcement-channel-id   # optional: where "Announce" in the Valo app posts
```

Start it:

```sh
npm install
node register-commands.js     # registers the slash commands (once, and after command changes)
node server.js
```

or with Docker: `docker compose up -d --build` (the compose file in the bot's folder), then
`docker exec media-bot node register-commands.js`.

Then, in your Discord server, run `/setup` (or skip it if you set the optional variables above).

### What your Valo server must do

The bot only talks to a handful of routes, all protected by `Authorization: Bearer <BOT_API_SECRET>`
(compare it in constant time, e.g. `hash_equals`). If you are adding the feature to your own server, implement:

| Route                                    | Purpose |
| ---------------------------------------- | ------- |
| `GET /api/info`                          | Any JSON object (used to confirm it is a Valo server) |
| `POST /api/requests`                     | Create a request. Body: `title, kind (movie\|series), year, imdb_id, tmdb_id, poster, requested_by, requester_id, message_id, status?`. `201` new; `200` with the existing row if the title is already open. Accept `status: "available"` for titles already on disk; anything else becomes `pending` |
| `GET /api/requests?updated_after=<unix>` | Requests changed after that time, each with `id, status, notified, message_id, requester_id, title, kind, note, updated` |
| `GET /api/requests/{id}`                 | One request |
| `PATCH /api/requests/{id}`               | The bot writes `notified` (bool) and clears `message_id`; don't bump `updated` for these |
| `DELETE /api/requests/{id}`              | Used by `/clear` |
| `POST /api/log`                          | Optional `{level, message}` into the admin log |

Statuses are `pending | approved | available | denied`. Whenever an admin sets a final status
(`available` or `denied`) set `notified = null` so the bot picks it up on its next poll.
The full contract is in [Requests and the Discord bot](admin-requests.md).

A minimal Laravel shape:

```php
// routes/api.php
Route::middleware('bot_secret')->prefix('requests')->group(function () {
    Route::post('/', [RequestController::class, 'botStore']);
    Route::get('/', [RequestController::class, 'botIndex']);          // ?updated_after=
    Route::patch('/{id}', [RequestController::class, 'botUpdate']);   // notified, message_id
    Route::delete('/{id}', [RequestController::class, 'botDestroy']);
});

// middleware: 401 unless hash_equals(config('services.bot.secret'), $request->bearerToken())
```

## Troubleshooting

| Symptom                                         | Fix |
| ----------------------------------------------- | --- |
| Slash commands don't show up                    | Wait a few minutes after `register-commands.js`; re-invite the bot with the `applications.commands` scope |
| "Valo is not set up on this server yet"         | An admin must run `/setup` and finish the three steps |
| `/settings server`: "private network"           | The address must be public HTTPS, not `localhost` or `192.168.x.x` |
| `/settings server`: "rejected the bot secret"   | `BOT_API_SECRET` on the server and the secret you typed differ; restart the server after changing `.env` |
| `/settings server`: "no bot request API"        | The server is too old or lacks the request routes |
| "I can no longer see the request channel"       | Give the bot View Channel, Send Messages and Embed Links on that channel, then `/settings channel` again |
| Member didn't get a DM                          | They have DMs from server members closed; the request is still updated |
| Request cards not deleted                       | Check the bot can still see the channel; failures are logged in the Valo admin log |
