<img src="./src-tauri/icons/128x128.png" alt="logo" align="left" height="100"/>

# Open Video Downloader (youtube-dl-gui) <br> ![version badge](https://img.shields.io/github/v/release/indep-arg/youtube-dl-gui?label=latest-release) ![GitHub](https://img.shields.io/github/license/indep-arg/youtube-dl-gui) ![downloads](https://img.shields.io/github/downloads/indep-arg/youtube-dl-gui/total)

> **This is an independent fork.** It is maintained by [indep-arg](https://github.com/indep-arg) and is based on
> [jely2002/youtube-dl-gui](https://github.com/jely2002/youtube-dl-gui) by Jelle Glebbeek and contributors.
> It is not affiliated with or endorsed by the original author. See [Credits](#credits-and-fork-notice).

Open Video Downloader is a simple, cross-platform desktop application that lets you download videos, audio, subtitles and metadata from hundreds of supported websites, not just YouTube.  
It provides an easy-to-use interface around [yt-dlp](https://github.com/yt-dlp/yt-dlp) so you don’t have to touch the command line.


## Features

- **Cross-platform:** works on Windows and Linux.
- **Audio or video downloads:** grab full videos or extract only the audio track.
- **Subtitles and metadata:** automatically saves available captions and video information.
- **Quality control:** choose your preferred resolution, frame rate, and output format like MP4 or MKV.
- **Playlists:** download entire playlists in one go.
- **Custom output:** set your download location and control filenames using presets or custom templates.
- **Smart queueing:** OVD balances multiple downloads automatically so your computer doesn’t slow down.
- **Authentication:** supports browser cookie files, basic auth and video passwords.
- **Automatic updates:** both the app and yt-dlp are kept up to date automatically.
- **Light and dark mode:** adapts to your system theme with clear progress and error handling.
- **Shortcuts:** queue and download videos using shortcuts, stay up-to-date on progress with notifications.

## Download

The latest versions for **Windows** and **Linux** are available on the  
[GitHub Releases page](https://github.com/indep-arg/youtube-dl-gui/releases).

macOS builds are not provided in this fork.

Download the installer or archive for your platform and follow the normal installation steps.  
No command-line setup is required.

#### What file do I download?
| Your Computer                         | Download                                       |
|---------------------------------------|------------------------------------------------|
| **Windows**                           | `Open.Video.Downloader_x.x.x_x64-setup.exe`    |
| **Linux generic (x64)**               | `Open.Video.Downloader_x.x.x_amd64.AppImage`   |
| **Linux generic (aarch64)**           | `Open.Video.Downloader_x.x.x_aarch64.AppImage` |
| **Linux Debian/Ubuntu (x64)**         | `Open.Video.Downloader_x.x.x_amd64.deb`        |
| **Linux Debian/Ubuntu (aarch64)**     | `Open.Video.Downloader_x.x.x_arm64.deb`        |
| **Linux Fedora/RHEL (x64)**           | `Open.Video.Downloader_x.x.x-x_amd64.rpm`      |
| **Linux Fedora/RHEL (aarch64)**       | `Open.Video.Downloader_x.x.x-x_aarch64.rpm`    |

## Documentation

- [User guide](./docs/GUIDE.md): installing, using the app and what to do when something goes wrong.
- [Changelog](./CHANGELOG.md) and [contributing](./CONTRIBUTING.md).
- [Translating](./docs/TRANSLATING.md).

## How it works

Open Video Downloader uses a front-end built with Vue 3 and a Rust backend powered by [Tauri](https://tauri.app/).  
When you add a video or playlist, the app communicates with yt-dlp to fetch information, process options and start the download.  
The download progress and any errors are displayed in the app.

## Contributing

Developers are welcome to contribute.  
You’ll need Node.js (v24+) and Rust installed.

```
npm install
npm run tauri dev
```

For more details, see [CONTRIBUTING.md](./CONTRIBUTING.md).

## License and disclaimer

Open Video Downloader is distributed under the [AGPL-3.0 license](./LICENSE).

### Credits and fork notice

This project is a fork of [youtube-dl-gui / Open Video Downloader](https://github.com/jely2002/youtube-dl-gui),
created by **Jelle Glebbeek ([jely2002](https://github.com/jely2002))** and contributors, who hold the copyright on the original code.
All original copyright and license notices are preserved, as required by the AGPL-3.0.

Changes made in this fork are also released under the AGPL-3.0-or-later, and are
recorded in the git history of this repository. The fork is maintained by [indep-arg](https://github.com/indep-arg).
Documentation such as the [wiki](https://github.com/jely2002/youtube-dl-gui/wiki) is still provided by the original project.

#### Use this application responsibly
The maintainers of Open Video Downloader cannot be held liable for misuse of this application, as stated in the AGPL-3.0 license (section 16).  
We do not condone the use of this software to violate local laws or platform terms of service (including the DMCA).  
Users are personally responsible for ensuring they use this software fairly and within legal boundaries.
