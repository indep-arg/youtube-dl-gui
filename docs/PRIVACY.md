# Privacy

Last updated: 2026-10-17

The original app sent error reports to an outside service (Sentry). This fork took that out, together
with the "Report" button. There is no telemetry, no account and nothing that counts what you download.

## What stays on your computer

Your settings, the passwords you save for signed-in downloads, the logs and the downloads themselves.
Passwords are kept encrypted. The clipboard is only read if you turn on the option to fill in copied
links or the shortcuts, and it is never sent anywhere.

## What it connects to

- **GitHub**, to look for updates and to download yt-dlp, ffmpeg and the other tools.
- **The sites you download from**, because that is the point of the app.
- **The SponsorBlock server**, only if you use SponsorBlock.
- **Your proxy**, only if you set one.

As with any download, those servers can see your IP address.

## Updates

Updates of the app are signed, and the app refuses one that isn't. The tools it downloads are checked
before they are used. Every installer on the releases page has a `.sig` file next to it if you want to
check it yourself.

The Windows installers are not code-signed, so Windows may warn about an unknown publisher.

## Questions

Open an [issue](https://github.com/indep-arg/youtube-dl-gui/issues/new/choose). To report a security
problem privately, see [SECURITY.md](../SECURITY.md).
