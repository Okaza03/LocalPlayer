# Testing with curl

[← Back to the API guide](README.md)

These commands repeat the [server check](server-check.md) by hand. Replace `SERVER` with your address.

```sh
SERVER=http://192.168.1.20:8080

# API present? Expect JSON with name and auth.
curl -i $SERVER/api/info

# Sign-in endpoint present? Expect 401.
curl -i -X POST $SERVER/api/login -H 'Content-Type: application/json' \
     -d '{"username":"","password":""}'

# Library protected? With auth on, expect 401.
curl -i $SERVER/api/library

# Sign in and keep the token.
TOKEN=$(curl -s -X POST $SERVER/api/login -H 'Content-Type: application/json' \
        -d '{"username":"user","password":"hunter2"}' | python3 -c 'import json,sys; print(json.load(sys.stdin)["token"])')

# Library loads? Expect JSON.
curl -s -H "Authorization: Bearer $TOKEN" $SERVER/api/library | python3 -m json.tool | head -40

# Streams support Range? Expect "206 Partial Content" and a Content-Range header.
curl -s -o /dev/null -D - -H "Authorization: Bearer $TOKEN" -H 'Range: bytes=0-1' \
     $SERVER/api/stream/movie/42
```

For the admin endpoints, see [Admin API](admin.md#testing-with-curl).
