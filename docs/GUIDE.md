# User guide

Open Video Downloader is a desktop app that downloads videos, audio, subtitles and metadata from
hundreds of sites, with a window instead of a command line. It runs on
[yt-dlp](https://github.com/yt-dlp/yt-dlp), so it works wherever yt-dlp does.

This is the guide of the [indep-arg fork](../README.md#credits-and-fork-notice).

## Features

- Video or audio downloads, with the quality, format and tracks you choose.
- Playlists, with a selection of the items to download.
- Subtitles, including automatic ones, and embedded subtitles.
- SponsorBlock: remove or mark sponsors, intros and other parts.
- Partial downloads: only a time range, or only some chapters.
- Filters by size and date, and raw yt-dlp match filters.
- Private videos, through browser cookies, a cookie file or a user and password.
- A queue that balances several downloads at once.
- Windows and Linux (x64 and ARM64). Light and dark themes, and 13 languages.

## Installing

Download the file for your system from the
[releases page](https://github.com/indep-arg/youtube-dl-gui/releases/latest).

| System | File |
|---|---|
| Windows | `…_x64-setup.exe` or `…_arm64-setup.exe` (installer), or `…-portable.zip` |
| Linux, any distribution | `….AppImage` (make it executable and run it) |
| Debian, Ubuntu | `.deb` |
| Fedora, RHEL | `.rpm` |

The Windows installers are not code-signed, so Windows SmartScreen may warn about an unknown
publisher: choose *More info*, then *Run anyway*. The updates the app downloads later are signed and
checked.

This is a separate app from the original one. Settings, history and saved credentials of the
original app are not carried over.

### Portable mode

On Windows, extract the `-portable.zip` and keep the `ovd-portable` folder next to the `.exe`. The app
then keeps all its data in that folder instead of the user's data folder, so you can move it around.

## First launch

The first time, the app downloads the tools it needs: yt-dlp, ffmpeg and ffprobe, Deno and
AtomicParsley. Each download is checked against a signed manifest and a hash before it is used. This
needs an internet connection and takes a minute or two.

The tools are kept up to date automatically unless you turn that off in **Settings > App**.

## Downloading

1. **Add a link.** Paste it in the field at the top, drag it onto the window, or copy it and
   let the app fill it in (**Settings > App > Input**). You can also drop a `.txt` or `.csv` file with
   several links.
2. **Choose what to download.** The card shows the title and the size. Pick video or audio, the
   quality and the format. For playlists, pick the items first.
3. **Download.** Press the download button of the card, or the one for the whole queue.

Keyboard shortcuts (they work even when the app is in the background; turn them on in
**Settings > App > Input**):

| Shortcut | Action |
|---|---|
| `Alt+Shift+V` | Add the link in the clipboard |
| `Alt+Shift+D` | Add the link in the clipboard and download it |
| `Alt+Shift+Enter` | Download the whole queue |

Each download also has its own preferences, metadata and logs. Changes there only affect that
download; the global defaults are in the settings.

## Settings

The tabs of **Settings**:

- **Downloads:** output (cover image, metadata, thumbnail file, precise cuts, post-processing, location
  and file names), extra files (an `.info.json` file, the description, and each chapter as its own file),
  subtitles, SponsorBlock and performance (how many downloads run at once, when to split playlists,
  whether to load the size automatically, a speed limit, retries, and how many parts of a video are
  downloaded at once).
- **App:** appearance (theme, language), input, notifications and updates.
- **Network:** proxy, skipping certificate verification (only if you trust the proxy), browser
  impersonation and yt-dlp extractor arguments.
- **System:** tray icon, start with the system and what happens when you close the window.
- **About:** version, links and licenses.

The menu next to **Save** can also reset the settings, or export them to a file and import them again,
for example to move them to another computer.

Other screens in the menu: **Location** (folders and file name templates), **Subtitles**,
**Authentication** and **Input filters**.

## Things worth knowing

- **Your data.** Settings and preferences are JSON files in the app's data folder (on Linux
  `~/.local/share/io.github.indep-arg.youtube-dl-gui`, on Windows
  `%APPDATA%\io.github.indep-arg.youtube-dl-gui`, or the `ovd-portable` folder in portable mode). The
  downloaded tools are in its `bin` folder.
- **Passwords.** User names and passwords for signed-in downloads are stored in an encrypted vault in
  that folder. Its key is kept in the system's key store.
- **Privacy.** The app has no telemetry and sends no error reports. It only connects to the sites you
  ask it to download from, and to GitHub for updates of the app and the tools. Details in
  [PRIVACY.md](./PRIVACY.md).
- **File names.** Templates use yt-dlp fields such as `%(title)s`; the file name is cleaned of
  characters the system does not allow.

## When something goes wrong

1. **Update first.** An old yt-dlp is the usual reason a site stops working. Check that **Automatically
   update tools** is on in **Settings > App**, then restart the app.
2. **Read the log.** Open the item and look at its **Logs** tab: it shows what yt-dlp said, and some
   errors come with an explanation and a shortcut to fix them.
3. **Is it just that site?** If the same link also fails with the latest yt-dlp on the command line, the
   problem is in yt-dlp: report it in [its tracker](https://github.com/yt-dlp/yt-dlp/issues).
4. **Still stuck?** [Open an issue](https://github.com/indep-arg/youtube-dl-gui/issues/new?template=bug_report.yml)
   with the app version, your system, how you installed it, and the log.

## Building from source

You need [Node.js](https://nodejs.org/) 24 and [Rust](https://rustup.rs/) (stable), plus the
[system libraries Tauri needs](https://tauri.app/start/prerequisites/) on Linux.

```bash
npm install
npm run tauri dev          # run in development mode
npm run build              # build the frontend
npx tauri build            # build the installers
```

Before sending a change, run the checks listed in [CONTRIBUTING.md](../CONTRIBUTING.md#testing).
`npm run licenses` needs [cargo-about](https://github.com/EmbarkStudios/cargo-about) installed.

## Contributing

Bug reports, ideas and pull requests are welcome. See [CONTRIBUTING.md](../CONTRIBUTING.md), and
[TRANSLATING.md](./TRANSLATING.md) if you want to help with a translation.
