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
