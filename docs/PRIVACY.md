# Privacy and trust

Last updated: 2026-10-17

## In short

- **No telemetry, no accounts, no ads.** The app does not collect statistics about how you use it.
- **No error reports.** When something fails, the details stay on your computer, in the log of the
  download. Nothing is sent anywhere unless you copy it into an issue yourself.
- **Updates and tools are verified** with signatures and hashes before they are used.
- **Open source.** The code is licensed under the [AGPL-3.0](../LICENSE) and every release is built by
  the public workflows of this repository.

The original project sent error reports to a third-party service. This fork removed it.

## What stays on your computer

- Your settings and preferences (including the folders you used recently and the size and position of
  the window), as JSON files in the app's data folder.
- User names and passwords you add for signed-in downloads, in an encrypted vault in the same folder.
  The key of the vault is kept in the system's key store (Windows Credential Manager, or the Secret
  Service on Linux).
- The downloads and the logs of each download.

The clipboard is only read when you turn on **Automatically fill in copied links** or the keyboard
shortcuts, and what it contains is never sent anywhere.

## What the app connects to

| To | Why | When |
|---|---|---|
| **GitHub** (`github.com`) | Looking for new versions of the app and downloading them, and downloading the signed list of tools (`manifest.json`) | At start and from time to time, unless you turn the automatic updates off |
| **The hosts of the tools** (yt-dlp, ffmpeg, Deno, AtomicParsley, on GitHub) | Downloading and updating the tools | The first time, and when a new version exists |
| **The sites you download from** | Downloading what you ask for. yt-dlp runs on your computer and talks to the site directly. Thumbnails are loaded from the site as well | When you add or download a link |
| **The SponsorBlock server** (address configurable) | Finding the parts to remove or mark | Only if you use SponsorBlock |
| **Your proxy**, if you set one | Sending your traffic through it | Only if you turn it on |

Like any download, these servers can see your IP address. If you give the app cookies or a password, they
are used for the requests to that site, the same way your browser would. Links in the app (for
example the wiki) only open in your browser when you click them.

## How updates and tools are verified

- **The app.** Every update is signed with a key of this project. The app has the public key built in and
  rejects an update whose signature does not match.
- **The tools.** The list of tools is signed (Ed25519) and the app has the public key built in. Each tool
  is checked against the SHA-256 hash in that list before it is installed, and an archive that tries to
  write outside its folder is rejected.
- **The app itself.** The webview is isolated: it can only call the commands the app lists, the allowed
  permissions are limited, and a content security policy blocks anything else.

This does not remove trust in the projects behind the tools (yt-dlp, ffmpeg…): a hash proves that you got
the same file the list was made from, not that the file is harmless. And the Windows installers are **not
code-signed**, so SmartScreen may warn about an unknown publisher.

### Check a download yourself

Every installer of a release has a `.sig` file next to it. With [rsign2](https://github.com/jedisct1/rsign2)
(or minisign) and the public key of the project:

```bash
echo 'untrusted comment: minisign public key: F0727D72176D3F26
RWQmP20Xcn1y8AVs1UvejABamstd/f7B702mwaPP69ktVfcaEGsNulw3' > project.pub
base64 -d < Open.Video.Downloader_2026.10.0_amd64.AppImage.sig > file.minisig
rsign verify -p project.pub -x file.minisig Open.Video.Downloader_2026.10.0_amd64.AppImage
```

It prints `Signature and comment signature verified` when the file is the one that was published.

## Questions

Open an [issue](https://github.com/indep-arg/youtube-dl-gui/issues/new/choose). To report a security
problem privately, see [SECURITY.md](../SECURITY.md).
