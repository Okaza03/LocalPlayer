#!/usr/bin/env python3
"""Minimal reference media server for LocalPlayer (Python 3.9+, standard library only).

    python3 localplayer_server.py ~/Videos                  # open library, port 8080
    python3 localplayer_server.py ~/Videos --add-user noah  # enable sign-in

Expected layout (anything else is ignored):

    Movies/Inception (2010)/Inception.mkv   (+ poster.jpg, fanart.jpg, info.json)
    Series/The Office/Season 1/The.Office.S01E01.Pilot.mkv   (+ poster.jpg, fanart.jpg, info.json)

info.json may set title, year, overview, genres, rating, duration; for a series
also "episodes": {"S01E01": {"title": ..., "overview": ...}}.
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

VIDEO = {".mp4", ".mkv", ".webm", ".avi", ".mov", ".m4v", ".flv", ".wmv", ".mpg", ".mpeg", ".ogv", ".ts", ".3gp"}
IMAGE = (".jpg", ".jpeg", ".png", ".webp")
MOVIE_DIRS = {"movies", "films"}
SERIES_DIRS = {"series", "tv", "tv shows", "shows"}
EPISODE = re.compile(r"(?i)(?:^|[^a-z0-9])s(\d{1,2})[ ._-]?e(\d{1,3})(.*)$|(?:^|\D)(\d{1,2})x(\d{2,3})(.*)$")
JUNK = re.compile(r"(?i)\b(19\d\d|20\d\d|2160p|1080p|720p|480p|4k|bluray|webrip|web-dl|hdtv|x26[45]|h26[45]|hevc)\b.*$")


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
        return [d for d in self.root.iterdir() if d.is_dir() and d.name.lower() in names]

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
                poster = find_art(folder, [f.stem, f"{f.stem}-poster"] + (["poster", "folder", "cover"] if own else []))
                backdrop = find_art(folder, [f"{f.stem}-fanart"] + (["fanart", "backdrop", "background"] if own else []))
                movies.append({
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
                })
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
                    s, e, rest = (m.group(1), m.group(2), m.group(3)) if m.group(1) else (m.group(4), m.group(5), m.group(6))
                    s, e = int(s), int(e)
                    meta = eps_info.get(f"S{s:02d}E{e:02d}", {})
                    still = find_art(f.parent, [f.stem, f"{f.stem}-thumb"])
                    seasons.setdefault(s, []).append({
                        "id": "e:" + str(f.relative_to(self.root)),
                        "number": e,
                        "title": meta.get("title") or clean(rest)[0] or f"Episode {e}",
                        "overview": meta.get("overview", ""),
                        "duration": meta.get("duration"),
                        "still": still and self.url("art", still),
                        "stream": self.url("media", f),
                    })
                if not seasons:
                    continue
                title, year = clean(show.name)
                poster = find_art(show, ["poster", "folder", "cover"])
                backdrop = find_art(show, ["fanart", "backdrop", "background"])
                newest = max(p.stat().st_mtime for p in show.rglob("*") if p.suffix.lower() in VIDEO)
                series.append({
                    "id": "s:" + show.name,
                    "title": info.get("title", title),
                    "year": info.get("year", year),
                    "overview": info.get("overview", ""),
                    "genres": info.get("genres", []),
                    "rating": info.get("rating"),
                    "added": int(newest),
                    "poster": poster and self.url("art", poster),
                    "backdrop": backdrop and self.url("art", backdrop),
                    "seasons": [{"number": n, "episodes": eps} for n, eps in sorted(seasons.items())],
                })
        return {"movies": movies, "series": series}


class Users:
    """users.json: {"name": {"salt": hex, "hash": hex}}; tokens.json: {"token": "name"}."""

    def __init__(self, folder):
        self.folder = folder
        self.users = read_info(folder / "users.json")
        self.tokens = read_info(folder / "tokens.json")
        self.lock = threading.Lock()

    @staticmethod
    def hash(password, salt):
        return hashlib.pbkdf2_hmac("sha256", password.encode(), bytes.fromhex(salt), 200_000).hex()

    def save(self, name, data):
        self.folder.mkdir(parents=True, exist_ok=True)
        path = self.folder / name
        path.write_text(json.dumps(data, indent=2))
        path.chmod(0o600)

    def add(self, name, password):
        salt = secrets.token_hex(16)
        self.users[name] = {"salt": salt, "hash": self.hash(password, salt)}
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

    def logout(self, token):
        with self.lock:
            if self.tokens.pop(token, None):
                self.save("tokens.json", self.tokens)


def make_handler(lib, users, name):
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
                return self.send_json({"token": token, "name": req["username"]})
            if path == "/api/logout":
                users.logout(self.token())
                return self.send_json({})
            self.send_json({"error": "not found"}, 404)

        def do_HEAD(self):
            self.do_GET(head=True)

        def do_GET(self, head=False):
            path = urlparse(self.path).path
            if path == "/api/info":
                return self.send_json({"name": name, "auth": bool(users.users), "version": 1})
            if not self.authorized():
                return self.send_json({"error": "sign in required"}, 401)
            if path == "/api/library":
                return self.send_json(lib.get())
            for prefix in ("/media/", "/art/"):
                if path.startswith(prefix):
                    f = (lib.root / unquote(path[len(prefix):])).resolve()
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
            self.send_header("Content-Type", mimetypes.guess_type(f.name)[0] or "application/octet-stream")
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
    ap.add_argument("--add-user", metavar="NAME", help="create/update a user, then exit")
    a = ap.parse_args()
    root = a.media.expanduser().resolve()
    users = Users(root / ".localplayer")
    if a.add_user:
        pw = getpass.getpass(f"Password for {a.add_user}: ")
        if pw != getpass.getpass("Again: "):
            raise SystemExit("passwords don't match")
        users.add(a.add_user, pw)
        print(f"User {a.add_user} saved. Sign-in is now required.")
        return
    name = a.name or f"{os.uname().nodename} media"
    srv = ThreadingHTTPServer((a.host, a.port), make_handler(Library(root), users, name))
    print(f"Serving {root} on http://{a.host}:{a.port}  (sign-in {'on' if users.users else 'off'})")
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    main()
