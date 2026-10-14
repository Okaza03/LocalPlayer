#!/usr/bin/env python3
"""Minimal reference media server for LocalPlayer (Python 3.9+, standard library only).

    python3 localplayer_server.py ~/Videos                  # open library, port 8080
    python3 localplayer_server.py ~/Videos --add-user user  # enable sign-in

Expected layout (anything else is ignored):

    Movies/Inception (2010)/Inception.mkv   (+ poster.jpg, fanart.jpg, info.json)
    Series/The Office/Season 1/The.Office.S01E01.Pilot.mkv   (+ poster.jpg, fanart.jpg, info.json)

info.json may set title, year, overview, genres, rating, duration, and the
details-page extras: director, creators, cast ([{"name", "character", "photo"}],
photo being a URL or an image file next to info.json), released, end_year,
network, status, budget, revenue, score, score_source, reviews. A series can add
"episodes": {"S01E01": {"title", "overview", "aired", "score"}}. A logo.png
next to the video is used as title artwork.

With sign-in on, each user's watch progress is kept in .localplayer/progress.json
and served from /api/progress, so it follows them between devices.
"""

import argparse
import getpass
import hashlib
import hmac
import json
import mimetypes
import os
import re
import secrets
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, quote, unquote, urlparse

VIDEO = {
    ".mp4",
    ".mkv",
    ".webm",
    ".avi",
    ".mov",
    ".m4v",
    ".flv",
    ".wmv",
    ".mpg",
    ".mpeg",
    ".ogv",
    ".ts",
    ".3gp",
}
IMAGE = (".jpg", ".jpeg", ".png", ".webp")
MOVIE_DIRS = {"movies", "films"}
SERIES_DIRS = {"series", "tv", "tv shows", "shows"}
EPISODE = re.compile(
    r"(?i)(?:^|[^a-z0-9])s(\d{1,2})[ ._-]?e(\d{1,3})(.*)$|(?:^|\D)(\d{1,2})x(\d{2,3})(.*)$"
)
JUNK = re.compile(
    r"(?i)\b(19\d\d|20\d\d|2160p|1080p|720p|480p|4k|bluray|webrip|web-dl|hdtv|x26[45]|h26[45]|hevc)\b.*$"
)


def clean(name):
    name = re.sub(r"\[.*?\]", "", name)
    m = re.search(r"\b(19\d\d|20\d\d)\b", name[1:])
    year = int(m.group(1)) if m else None
    return re.sub(r"[._()]+", " ", name[:1] + JUNK.sub("", name[1:])).strip(" -"), year


def find_art(folder, names):
    for n in names:
        for ext in IMAGE:
            if (folder / f"{n}{ext}").is_file():
                return folder / f"{n}{ext}"
    return None


QUALITY = re.compile(r"(?i)\b(2160p|4k|1440p|1080p|720p|576p|480p)\b")
EXTRAS = (
    "director",
    "creators",
    "released",
    "end_year",
    "network",
    "status",
    "budget",
    "revenue",
    "score",
    "score_source",
    "reviews",
)


def file_info(lib, f):
    q = QUALITY.search(f.stem)
    return {
        "quality": q and q.group(1).lower().replace("4k", "2160p"),
        "container": f.suffix.lstrip(".").lower(),
        "size": f.stat().st_size,
        "name": f.name,
        "path": "/" + str(f.parent.relative_to(lib.root)),
    }


def extras(lib, info, folder):
    out = {k: info[k] for k in EXTRAS if k in info}
    cast = []
    for c in info.get("cast", []):
        c = dict(c)
        photo = c.get("photo")
        if photo and "://" not in photo and (folder / photo).is_file():
            c["photo"] = lib.url("art", folder / photo)
        cast.append(c)
    if cast:
        out["cast"] = cast
    logo = find_art(folder, ["logo", "clearlogo"])
    if logo:
        out["logo"] = lib.url("art", logo)
    return out


def read_info(path):
    try:
        return json.loads(path.read_text())
    except (OSError, ValueError):
        return {}


class Library:
    def __init__(self, root):
        self.root = root
        self.cache, self.stamp, self.lock = None, 0, threading.Lock()

    def url(self, kind, path):
        return f"/{kind}/" + quote(str(path.relative_to(self.root)))

    def get(self):
        with self.lock:
            if self.cache is None or time.time() - self.stamp > 60:
                self.cache, self.stamp = self.scan(), time.time()
            return self.cache

    def category(self, names):
        return [
            d for d in self.root.iterdir() if d.is_dir() and d.name.lower() in names
        ]

    def scan(self):
        movies, series = [], []
        for top in self.category(MOVIE_DIRS):
            for f in sorted(top.rglob("*")):
                if f.suffix.lower() not in VIDEO:
                    continue
                own = f.parent != top
                info = read_info(f.with_suffix(".json"))
                if own and not info:
                    info = read_info(f.parent / "info.json")
                title, year = clean(f.parent.name if own else f.stem)
                folder = f.parent
                poster = find_art(
                    folder,
                    [f.stem, f"{f.stem}-poster"]
                    + (["poster", "folder", "cover"] if own else []),
                )
                backdrop = find_art(
                    folder,
                    [f"{f.stem}-fanart"]
                    + (["fanart", "backdrop", "background"] if own else []),
                )
                movies.append(
                    {
                        "id": "m:" + str(f.relative_to(self.root)),
                        "title": info.get("title", title),
                        "year": info.get("year", year),
                        "overview": info.get("overview", ""),
                        "genres": info.get("genres", []),
                        "rating": info.get("rating"),
                        "duration": info.get("duration"),
                        "added": int(f.stat().st_mtime),
                        "poster": poster and self.url("art", poster),
                        "backdrop": backdrop and self.url("art", backdrop),
                        "stream": self.url("media", f),
                        "file": file_info(self, f),
                        **(extras(self, info, folder) if own else {}),
                    }
                )
        for top in self.category(SERIES_DIRS):
            for show in sorted(d for d in top.iterdir() if d.is_dir()):
                info = read_info(show / "info.json")
                eps_info = info.get("episodes", {})
                seasons = {}
                for f in sorted(show.rglob("*")):
                    if f.suffix.lower() not in VIDEO:
                        continue
                    m = EPISODE.search(f.stem)
                    if not m:
                        continue
                    s, e, rest = (
                        (m.group(1), m.group(2), m.group(3))
                        if m.group(1)
                        else (m.group(4), m.group(5), m.group(6))
                    )
                    s, e = int(s), int(e)
                    meta = eps_info.get(f"S{s:02d}E{e:02d}", {})
                    still = find_art(f.parent, [f.stem, f"{f.stem}-thumb"])
                    seasons.setdefault(s, []).append(
                        {
                            "id": "e:" + str(f.relative_to(self.root)),
                            "number": e,
                            "title": meta.get("title")
                            or clean(rest)[0]
                            or f"Episode {e}",
                            "overview": meta.get("overview", ""),
                            "duration": meta.get("duration"),
                            "still": still and self.url("art", still),
                            "stream": self.url("media", f),
                            "aired": meta.get("aired"),
                            "score": meta.get("score"),
                            "file": file_info(self, f),
                        }
                    )
                if not seasons:
                    continue
                title, year = clean(show.name)
                poster = find_art(show, ["poster", "folder", "cover"])
                backdrop = find_art(show, ["fanart", "backdrop", "background"])
                newest = max(
                    p.stat().st_mtime
                    for p in show.rglob("*")
                    if p.suffix.lower() in VIDEO
                )
                series.append(
                    {
                        "id": "s:" + show.name,
                        "title": info.get("title", title),
                        "year": info.get("year", year),
                        "overview": info.get("overview", ""),
                        "genres": info.get("genres", []),
                        "rating": info.get("rating"),
                        "added": int(newest),
                        "poster": poster and self.url("art", poster),
                        "backdrop": backdrop and self.url("art", backdrop),
                        "seasons": [
                            {"number": n, "episodes": eps}
                            for n, eps in sorted(seasons.items())
                        ],
                        **extras(self, info, show),
                    }
                )
        return {"movies": movies, "series": series}


class Users:
    """users.json: {"name": {"salt": hex, "hash": hex, "admin": bool}}; tokens.json: {"token": "name"}."""

    def __init__(self, folder):
        self.folder = folder
        self.users = read_info(folder / "users.json")
        self.tokens = read_info(folder / "tokens.json")
        self.lock = threading.Lock()

    @staticmethod
    def hash(password, salt):
        return hashlib.pbkdf2_hmac(
            "sha256", password.encode(), bytes.fromhex(salt), 200_000
        ).hex()

    def save(self, name, data):
        self.folder.mkdir(parents=True, exist_ok=True)
        path = self.folder / name
        path.write_text(json.dumps(data, indent=2))
        path.chmod(0o600)

    def add(self, name, password, admin=False):
        salt = secrets.token_hex(16)
        self.users[name] = {
            "salt": salt,
            "hash": self.hash(password, salt),
            "admin": admin,
        }
        self.save("users.json", self.users)

    def login(self, name, password):
        u = self.users.get(name)
        if not u or not hmac.compare_digest(u["hash"], self.hash(password, u["salt"])):
            return None
        token = secrets.token_urlsafe(32)
        with self.lock:
            self.tokens[token] = name
            self.save("tokens.json", self.tokens)
        return token

    def is_admin(self, name):
        return bool(self.users.get(name, {}).get("admin"))

    def logout(self, token):
        with self.lock:
            if self.tokens.pop(token, None):
                self.save("tokens.json", self.tokens)


class Progress:
    """progress.json: {"user": {"movie:<id>": {"position", "duration", "updated"}}}."""

    def __init__(self, folder):
        self.path = folder / "progress.json"
        self.data = read_info(self.path)
        self.lock = threading.Lock()

    @staticmethod
    def entry(key, e):
        kind, id = key.split(":", 1)
        pos, dur = e["position"], e["duration"]
        pct = round(100 * pos / dur) if dur > 0 else 0
        return {
            "type": kind,
            "id": id,
            "position": pos,
            "duration": dur,
            "percent": min(pct, 100),
            "completed": pct >= 90,
            "updated": e["updated"],
        }

    def list(self, user):
        mine = self.data.get(user, {})
        out = [self.entry(k, e) for k, e in mine.items()]
        return sorted(out, key=lambda e: -e["updated"])

    def record(self, user, kind, id, position, duration):
        key = f"{kind}:{id}"
        with self.lock:
            e = {
                "position": position,
                "duration": duration,
                "updated": int(time.time()),
            }
            self.data.setdefault(user, {})[key] = e
            self.path.parent.mkdir(parents=True, exist_ok=True)
            self.path.write_text(json.dumps(self.data, indent=2))
            self.path.chmod(0o600)
        return self.entry(key, e)


def library_ids(lib, kind):
    data = lib.get()
    if kind == "movie":
        return {m["id"] for m in data["movies"]}
    return {
        e["id"]
        for s in data["series"]
        for season in s["seasons"]
        for e in season["episodes"]
    }


def make_handler(lib, users, progress, name):
    class Handler(BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.1"

        def log_message(self, fmt, *args):
            if not self.path.startswith(("/media/", "/art/")):
                super().log_message(fmt, *args)

        def send_json(self, obj, status=200):
            body = json.dumps(obj).encode()
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def token(self):
            h = self.headers.get("Authorization", "")
            if h.startswith("Bearer "):
                return h[7:]
            return parse_qs(urlparse(self.path).query).get("token", [None])[0]

        def authorized(self):
            return not users.users or self.token() in users.tokens

        def user(self):
            """The signed-in user; None without sign-in, where progress has no owner."""
            return users.tokens.get(self.token())

        def do_POST(self):
            path = urlparse(self.path).path
            length = int(self.headers.get("Content-Length") or 0)
            body = self.rfile.read(length) if length else b""
            if path == "/api/login":
                try:
                    req = json.loads(body)
                    token = users.login(str(req["username"]), str(req["password"]))
                except (ValueError, KeyError, TypeError):
                    return self.send_json({"error": "bad request"}, 400)
                if not token:
                    time.sleep(1)  # slow down guessing
                    return self.send_json({"error": "wrong username or password"}, 401)
                user = str(req["username"])
                return self.send_json(
                    {"token": token, "name": user, "admin": users.is_admin(user)}
                )
            if path == "/api/logout":
                users.logout(self.token())
                return self.send_json({})
            if path == "/api/progress":
                user = self.user()
                if not user:
                    return self.send_json({"error": "sign in required"}, 401)
                try:
                    req = json.loads(body)
                    kind, id = req["type"], str(req["id"])
                    position, duration = float(req["position"]), float(
                        req.get("duration") or 0
                    )
                    if kind not in ("movie", "episode") or position < 0 or duration < 0:
                        raise ValueError
                except (ValueError, KeyError, TypeError):
                    return self.send_json({"error": "bad request"}, 422)
                if id not in library_ids(lib, kind):
                    return self.send_json({"error": "not found"}, 404)
                return self.send_json(
                    progress.record(user, kind, id, position, duration)
                )
            self.send_json({"error": "not found"}, 404)

        def do_HEAD(self):
            self.do_GET(head=True)

        def do_GET(self, head=False):
            path = urlparse(self.path).path
            if path == "/api/info":
                return self.send_json(
                    {"name": name, "auth": bool(users.users), "version": 1}
                )
            if not self.authorized():
                return self.send_json({"error": "sign in required"}, 401)
            if path == "/api/library":
                return self.send_json(lib.get())
            if path == "/api/progress":
                user = self.user()
                if not user:
                    return self.send_json({"error": "sign in required"}, 401)
                return self.send_json(progress.list(user))
            for prefix in ("/media/", "/art/"):
                if path.startswith(prefix):
                    f = (lib.root / unquote(path[len(prefix) :])).resolve()
                    if lib.root in f.parents and f.is_file():
                        return self.send_file(f, head)
            self.send_json({"error": "not found"}, 404)

        def send_file(self, f, head):
            size = f.stat().st_size
            start, end = 0, size - 1
            m = re.match(r"bytes=(\d*)-(\d*)", self.headers.get("Range", ""))
            if m and (m.group(1) or m.group(2)):
                if m.group(1):
                    start = int(m.group(1))
                    end = int(m.group(2)) if m.group(2) else end
                else:
                    start = max(0, size - int(m.group(2)))
                if start >= size:
                    self.send_response(416)
                    self.send_header("Content-Range", f"bytes */{size}")
                    self.send_header("Content-Length", "0")
                    return self.end_headers()
                end = min(end, size - 1)
                self.send_response(206)
                self.send_header("Content-Range", f"bytes {start}-{end}/{size}")
            else:
                self.send_response(200)
            self.send_header(
                "Content-Type",
                mimetypes.guess_type(f.name)[0] or "application/octet-stream",
            )
            self.send_header("Accept-Ranges", "bytes")
            self.send_header("Content-Length", str(end - start + 1))
            self.end_headers()
            if head:
                return
            with open(f, "rb") as fh:
                fh.seek(start)
                left = end - start + 1
                try:
                    while left > 0:
                        chunk = fh.read(min(1 << 20, left))
                        if not chunk:
                            break
                        self.wfile.write(chunk)
                        left -= len(chunk)
                except (BrokenPipeError, ConnectionResetError):
                    pass  # player seeked or closed

    return Handler


def main():
    ap = argparse.ArgumentParser(description="LocalPlayer reference media server")
    ap.add_argument("media", type=Path, help="folder containing Movies/ and Series/")
    ap.add_argument("--port", type=int, default=8080)
    ap.add_argument("--host", default="0.0.0.0")
    ap.add_argument("--name", default=None, help="name shown in the app")
    ap.add_argument(
        "--add-user", metavar="NAME", help="create/update a user, then exit"
    )
    ap.add_argument(
        "--admin", action="store_true", help="with --add-user: the user may open /admin"
    )
    a = ap.parse_args()
    root = a.media.expanduser().resolve()
    users = Users(root / ".localplayer")
    if a.add_user:
        pw = getpass.getpass(f"Password for {a.add_user}: ")
        if pw != getpass.getpass("Again: "):
            raise SystemExit("passwords don't match")
        users.add(a.add_user, pw, a.admin)
        role = "admin" if a.admin else "user"
        print(f"User {a.add_user} saved ({role}). Sign-in is now required.")
        return
    name = a.name or f"{os.uname().nodename} media"
    progress = Progress(root / ".localplayer")
    srv = ThreadingHTTPServer(
        (a.host, a.port), make_handler(Library(root), users, progress, name)
    )
    print(
        f"Serving {root} on http://{a.host}:{a.port}  (sign-in {'on' if users.users else 'off'})"
    )
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    main()
