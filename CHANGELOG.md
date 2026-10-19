# Changelog

This is an independent fork of [jely2002/youtube-dl-gui](https://github.com/jely2002/youtube-dl-gui).
Versions of this fork do not follow the numbering of the original project.

## Versioning

Releases use calendar versioning: `YEAR.MONTH.N`.

- `YEAR` is the four-digit year of the release, e.g. `2026`.
- `MONTH` is the month of the release without a leading zero, e.g. `10` for October.
- `N` is the release counter inside that month, starting at `0`.

So `2026.10.0` is the first release of October 2026 and `2026.10.1` is the second one.
Releases are tagged `app-vYEAR.MONTH.N` and the version is kept equal in
`package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`.

The number does not say whether a release breaks anything. Breaking changes are listed in
this file under **Breaking changes**.

## 2026.10.6

### Changes

- Pausing, resuming and cancelling are more reliable: resuming no longer runs the work that was queued
  before the pause a second time, cancelling also stops a link that is still loading, and a playlist no
  longer loses its download or gets stuck using a processor core with no way to cancel it.
- Fixed a crash that could close the app when a long error message was shortened.
- On Linux, yt-dlp no longer keeps running after the app is closed.
- Passwords and login headers reach yt-dlp through a private temporary file instead of the command line,
  where other programs on the computer could read them.
- Logs: a line that repeats is shown once with a count, live lines no longer run together or get lost,
  and a long log keeps its latest lines.
- Turning on the tray icon or the global shortcuts works right away.
- Links restored from the last session are loaded with the settings they had, such as cookies.
- The window position is saved when the app closes.
- Downloads of tools and app updates stop with an error instead of hanging when the connection stalls.
- The Linux AppImage is built with the same version of Tauri as the rest of the app.

## 2026.10.5

### Changes

- Tabs no longer jump sideways when another tab is chosen, in the settings, the preferences and the
  information of a download, and the date filter.

## 2026.10.4

### New

- Choose between the stable and the nightly builds of yt-dlp (**Settings > System**). Nightly builds get
  fixes for sites sooner. When a download fails in a way that usually means the site changed, the card
  offers to try them.
- Stop a live recording and keep what was recorded so far, as a normal video file. While it records, the
  card shows how much it has recorded.
- Live streams that have not started yet can be queued: they show when they start, and the app waits and
  records them from then on.
- A button in **Settings > About** copies information about the app, the system and the tools, to paste
  into a bug report. It contains no paths or personal data.

### Changes

- Tools whose file is not a program are installed again. This repairs ffmpeg on Linux for those who used
  versions `2026.10.0` or `2026.10.1`.
- yt-dlp is replaced as a whole when it updates, and the old copy is kept if that fails, instead of new
  files being copied over the old ones.
- The chat of a live stream is no longer downloaded as a subtitle.
- The IP address that some sites put in their links is hidden in the logs.
- When a download fails, the bottom bar says so instead of "Download complete", and an error is shown once
  in the logs instead of twice.
- Help texts, links and badges are easier to read, and every control has a name for screen readers.
- Clicking the label of the thumbnail file option no longer changes the cover image option.
- Each language is loaded only when it is used, so the app has less to load when it starts.

## 2026.10.3

### Changes

- Long descriptions in the settings and in the preferences of a download no longer run past the edge of
  the window. This was most visible in languages with longer texts, such as Spanish.
- At the smallest window size the controls at the bottom move to a second row instead of being cut off,
  and the format list no longer touches its arrow.
- The text of the progress bars is readable in the light theme.
- The error of a card is no longer cut in the middle of a line, and the card of a playlist keeps its title.
- The description of a video wraps instead of scrolling sideways.
- Disabled fields have a border, and the placeholder of a video without a picture is visible in the light theme.
- The queue bar at the bottom is hidden while the queue is empty, and the settings button is hidden while
  the settings are open.
- The precise cut option was shown twice in the download settings; only one is left. The logs of a download
  show the sections for skipped items and warnings only when they have something to show.
- The list of subtitle languages uses the names in the language of the app.
- The About page links to this project's guide, privacy page and changelog.
- Installing ffmpeg and ffprobe is faster: the archive is read once for both. The install screen says when a
  tool uses a file that was already downloaded.
- The Linux AppImage is built with newer versions of the tools that package it.
- Updated dependencies.

## 2026.10.2

### Breaking changes

- ffmpeg and ffprobe are no longer published in the format used before. Versions `2026.10.0` and
  `2026.10.1` cannot install the new one on Linux and will show an installation error when they start,
  and they do not offer the update from that screen. Download `2026.10.2` from the releases page and
  install it over them. On Windows they just download the new ffmpeg and keep working.

### New

- Live streams can be recorded, from the start if the site allows it.
- History of finished downloads, with buttons to open the folder, download again or remove an entry.
- Links that were still in the queue come back when the app is opened again (can be turned off).
- Settings per site: a folder and video or audio for each website.
- Limit the download speed, choose how many times to retry and how many parts download at once.
- Save the info file and the description next to a video, or split it into its chapters.
- Export and import the settings as a file.

### Changes

- ffmpeg and ffprobe now come from yt-dlp's FFmpeg builds on Windows and Linux. The Linux one used
  before crashed on some systems (for example Fedora) as soon as it opened a link, which broke live
  streams and partial downloads. They are installed again once after updating, and then replaced
  at most once a month.
- Tools that have no build for the computer (AtomicParsley on Linux ARM) are skipped, instead of
  installing one that cannot run.
- Empty playlists show an error instead of nothing.
- Fixed the light and dark themes losing the indigo colors when chosen by hand in the settings.
- Fixed a white strip that could still appear when the page moved by itself.
- Saving settings no longer leaves the button spinning when it fails, and the queue keeps its order
  when it is restored.
- Screenshots and the new features in the README.

## 2026.10.1

### Changes

- New indigo color scheme, in both the light and the dark theme.
- Fixed a white strip that appeared under the footer when scrolling on Linux: only the content area
  scrolls now, not the whole page.
- Fixed the text of the item count (`{details}`) in ten languages, and Korean can be selected now.
- Releases are now built only after the CI checks pass, including a Windows build and tests.

## 2026.10.0

First release of the fork, based on upstream `3.2.1`.

### Breaking changes

- The app identifier changed to `io.github.indep-arg.youtube-dl-gui`. Settings, history and
  stored credentials from the original app are not carried over.
- macOS is no longer supported and no macOS builds are published.
- The error report button was removed together with the Sentry integration.

### Changes

- The updater and the binaries manifest are served from this repository, signed with new keys.
- The binaries manifest is published as an asset of the `manifest` release instead of GitHub Pages.
- Removed MSIX, Snap, winget and Microsoft Store packaging, and the Apple and Windows code-signing steps.
- Updated all npm and Rust dependencies, Tauri 2.12 and Rust 1.99.
