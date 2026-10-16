# Security Policy

## Supported versions

Only the latest release gets security fixes. Valo can update itself (AppImage and Windows installer) or tells you when an update is out, so please update before reporting.

| Version         | Supported |
| --------------- | --------- |
| Latest release  | Yes       |
| Older releases  | No        |

## Reporting a vulnerability

**Please don't report security problems in public issues, discussions or pull requests.**

Report them privately through GitHub: go to the [Security tab](https://github.com/Okaza03/Valo/security/advisories/new) and choose **Report a vulnerability**. You can also email [Support@jhns.no](mailto:Support@jhns.no).

Please include:

- What the problem is and what an attacker could do with it
- Which part is affected: the desktop app, the Android TV app, the bundled server (`server/valo_server.py`), or the update / packaging process
- The Valo version and your platform
- Steps to reproduce, or a proof of concept

## What to expect

- You'll get a reply within 7 days confirming the report was received.
- I'll keep you updated while it's being looked into and fixed.
- Once a fix is released, the advisory is published. You'll be credited unless you'd rather not be.

## Scope

Things that are in scope include:

- The self-update process (download, `SHA256SUMS.txt` check, replacing the app)
- How the apps store and send sign-in tokens and server addresses
- The bundled server's sign-in, admin API and file serving (for example, reading files outside the library folder)
- Crashes or code execution caused by a crafted video, subtitle or server response

Out of scope:

- Problems in third-party media servers that implement the Valo API
- Vulnerabilities in FFmpeg or other dependencies that are already public, unless Valo uses them in a way that makes them worse
- The Windows SmartScreen warning (the installer isn't code-signed)
