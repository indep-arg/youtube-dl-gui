# WebDriver tests

These tests run the built app through tauri-driver, with the yt-dlp and ffmpeg it installs.
Linux only.

```sh
npx tauri build --no-bundle   # the app to test
npm run test:app -- --network # first time: also installs the tools into the cache
npm run test:app              # after that: no internet needed
```

`--only download-video,language` runs some of the tests, and `--keep` keeps their folders.

## What you need

- **tauri-driver**: `cargo install tauri-driver --locked`.
- **WebKitWebDriver** of the same version as the system WebKitGTK:
  - Debian and Ubuntu: `sudo apt install webkit2gtk-driver`.
  - Fedora: it comes in the `webkitgtk6.0` package. Without installing it:
    `dnf download webkitgtk6.0 --arch x86_64`, then
    `rpm2cpio webkitgtk6.0-*.rpm | cpio -idm ./usr/bin/WebKitWebDriver` and copy the file to
    `~/.cache/ovd-app-tests/WebKitWebDriver`.

`OVD_APP`, `TAURI_DRIVER` and `WEBKIT_DRIVER` point to other files, and `OVD_TEST_CACHE` to another cache
folder (by default `~/.cache/ovd-app-tests`).

## How they work

- Every test has its own data and download folders, so nothing touches your settings or your files.
  Close your own copy of the app first: only one can run at a time.
- The videos are made with ffmpeg and served from this computer, so the tests do not depend on any website.
- The tools are installed once (`install-from-scratch`, needs `--network`) and copied into each test.
- A failed test saves a screenshot in `test-results/app/`.
- The app window shows on the screen while the tests run. The `diagnostics` test copies text to the
  clipboard and puts back what was there.
