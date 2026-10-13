# LocalPlayer v0.2.0

A big redesign of the movie and series pages, and a new page for each episode.

### New: movie page
- A large backdrop, with the poster on the left ("A film by ..." on top).
- The title logo, genres, tagline, rating, year, runtime and quality.
- **Play movie**, a bookmark button and how much you've watched.
- Below: **Top billed** cast with photos, and **You might also like** from your library.
- Side panels: a **File · Metadata** panel (released, budget, box office, quality, codecs, bitrate, file size, file name and path) and **From critics** quotes.

### New: series page
- A full-width backdrop with the show's logo, genres, years and number of seasons.
- **Play S01 E01** (or the next episode you haven't finished), a bookmark button and a season picker.
- Three tabs:
  - **Episodes**: a grid of episode cards with stills, descriptions, rating, runtime and air date.
  - **Related**: shows like this one.
  - **Details**: info, network, status, creators and cast.

### New: episode page
Click an episode to open its page:
- Its still and a big play button.
- The synopsis, runtime, air date, rating and quality.
- File info and the series cast.
- **Up next**, and a link back to all episodes.

Hovering over an episode card still shows a play button that starts it right away.

### New: My list
The bookmark button saves a movie or show to a **My list** row on Home.

### For server owners
All the new information is optional: the app shows what your server sends and hides the rest. The README's API section lists the new fields (`logo`, `director`, `creators`, `cast`, `released`, `budget`, `revenue`, `score`, `reviews`, `network`, `status`, `file`, and `aired` for episodes). The bundled Python server sends file info automatically and reads the rest from `info.json`.

### Downloads
| File | For |
|---|---|
| `localplayer-v0.2.0-x86_64.AppImage` | any distro (`chmod +x` and run, updates itself) |
| `localplayer-v0.2.0-x86_64.flatpak` | any distro with Flatpak (added a few minutes after the others) |
| `localplayer_0.2.0-1_amd64.deb` | Debian / Ubuntu / Mint |
| `localplayer-0.2.0-linux-x86_64.tar.gz` | manual install |

If you're using the AppImage, click **Update now** to get this version.

Checksums for the AppImage, `.deb` and `.tar.gz` are in `SHA256SUMS.txt`.
