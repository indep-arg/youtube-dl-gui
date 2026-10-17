export interface BundleInfo {
  /** Keep the extracted directory rather than collapsing to a single file */
  keep_folder: boolean;
  /** Optional: rename the top-level extracted dir to this */
  folder_name?: string;
  /** Executable path inside the extracted root (posix style, no leading slash) */
  entry: string;
  /** Optional: rename the entry file (inside the folder) after extraction */
  rename_entry_to?: string;
}

export interface ToolSourceFile {
  url: string;
  sha256?: string;
  /**
   * For single-file archives: extract this entry to the canonical path.
   * (Ignored if `bundle` is present)
   */
  entry?: string;
  /** Bundle mode: keep folder; link canonical path to this entry inside it */
  bundle?: BundleInfo;
}

export interface ToolSource {
  name: string;
  version: string;
  files: Record<string, ToolSourceFile>;
}

export interface ToolLicense {
  license: string;
  url: string;
}

interface GitHubAsset {
  name: string;
  browser_download_url: string;
  /** e.g. `sha256:…`, when GitHub provides it. */
  digest?: string | null;
}

interface GitHubRelease {
  tag_name: string;
  assets: GitHubAsset[];
  published_at: string;
}

async function latestRelease(owner: string, repo: string): Promise<GitHubRelease> {
  console.log(`[sources] Fetching latest release for ${owner}/${repo}`);
  const headers: Record<string, string> = {};
  const githubToken = process.env.GITHUB_TOKEN ?? process.env.GH_TOKEN ?? null;
  if (githubToken) {
    headers['Authorization'] = `Bearer ${githubToken}`;
  }
  const res = await fetch(
    `https://api.github.com/repos/${owner}/${repo}/releases/latest`,
    { headers },
  );
  if (!res.ok) throw new Error(`failed to fetch ${owner}/${repo}: ${res.status}`);
  const release = (await res.json()) as GitHubRelease;
  console.log(
    `[sources] Latest ${owner}/${repo} release ${release.tag_name} published at ${release.published_at}`,
  );
  return release;
}

/**
 * The newest dated build of yt-dlp/FFmpeg-Builds. Its files never change, unlike
 * the ones of the `latest` release, which are replaced every day under the same
 * name (so a manifest hash would stop matching until the next manifest).
 */
async function latestAutobuild(owner: string, repo: string): Promise<GitHubRelease> {
  console.log(`[sources] Fetching the newest dated build of ${owner}/${repo}`);
  const headers: Record<string, string> = {};
  const githubToken = process.env.GITHUB_TOKEN ?? process.env.GH_TOKEN ?? null;
  if (githubToken) {
    headers['Authorization'] = `Bearer ${githubToken}`;
  }
  const res = await fetch(`https://api.github.com/repos/${owner}/${repo}/releases?per_page=20`, { headers });
  if (!res.ok) throw new Error(`failed to fetch ${owner}/${repo}: ${res.status}`);
  const releases = (await res.json()) as (GitHubRelease & { draft: boolean; prerelease: boolean })[];
  const release = releases.find(r => r.tag_name.startsWith('autobuild-') && !r.draft);
  if (!release) throw new Error(`no dated build found in ${owner}/${repo}`);
  console.log(`[sources] Newest ${owner}/${repo} build ${release.tag_name} published at ${release.published_at}`);
  return release;
}

/**
 * A file of an FFmpeg-Builds archive: the archive is downloaded and `binary` is
 * extracted from its `bin` folder. The hash comes from GitHub when it has one.
 */
function ffmpegBuildsFile(rel: GitHubRelease, suffix: string, binary: string): ToolSourceFile {
  const asset = rel.assets.find(a => a.name.endsWith(suffix) && !a.name.includes('shared'));
  if (!asset) throw new Error(`missing asset *${suffix}`);
  const folder = asset.name.replace(/\.(tar\.xz|zip)$/, '');
  const sha256 = asset.digest?.startsWith('sha256:') ? asset.digest.slice('sha256:'.length) : undefined;
  return { url: asset.browser_download_url, sha256, entry: `${folder}/bin/${binary}` };
}

function assetUrl(rel: GitHubRelease, name: string): string {
  const asset = rel.assets.find(a => a.name === name);
  if (!asset) throw new Error(`missing asset ${name}`);
  return asset.browser_download_url;
}

export async function fetchSources(): Promise<ToolSource[]> {
  console.log('[sources] Resolving tool sources');
  const ytRel = await latestRelease('yt-dlp', 'yt-dlp');
  const ffRel = await latestRelease('eugeneware', 'ffmpeg-static');
  const ffBuildsRel = await latestAutobuild('yt-dlp', 'FFmpeg-Builds');
  const denoRel = await latestRelease('denoland', 'deno');
  const apRel = await latestRelease('wez', 'atomicparsley');

  console.log('[sources] Successfully resolved tool metadata');

  return [
    {
      name: 'yt-dlp',
      version: ytRel.tag_name,
      files: {
        'linux-x86_64': {
          url: assetUrl(ytRel, 'yt-dlp_linux.zip'),
          bundle: {
            keep_folder: true,
            folder_name: 'yt-dlp_linux',
            entry: 'yt-dlp_linux',
            rename_entry_to: 'yt-dlp',
          },
        },
        'linux-aarch64': {
          url: assetUrl(ytRel, 'yt-dlp_linux_aarch64.zip'),
          bundle: {
            keep_folder: true,
            folder_name: 'yt-dlp_linux_aarch64',
            entry: 'yt-dlp_linux_aarch64',
            rename_entry_to: 'yt-dlp',
          },
        },
        'windows-x86_64': {
          url: assetUrl(ytRel, 'yt-dlp_win.zip'),
          bundle: {
            keep_folder: true,
            folder_name: 'yt-dlp',
            entry: 'yt-dlp.exe',
            rename_entry_to: 'yt-dlp.exe',
          },
        },
        'windows-aarch64': {
          url: assetUrl(ytRel, 'yt-dlp_win_arm64.zip'),
          bundle: {
            keep_folder: true,
            folder_name: 'yt-dlp',
            entry: 'yt-dlp_arm64.exe',
            rename_entry_to: 'yt-dlp.exe',
          },
        },
      },
    },
    {
      name: 'ffmpeg',
      version: ffRel.tag_name,
      files: {
        // Old versions of the app read these. On Linux this static build crashes
        // as soon as it resolves a host name, so newer versions use the -v2 files.
        'linux-x86_64': { url: assetUrl(ffRel, 'ffmpeg-linux-x64') },
        'linux-aarch64': { url: assetUrl(ffRel, 'ffmpeg-linux-arm64') },
        // .tar.xz archives, which only versions from 2026.10.2 on can open.
        'linux-x86_64-v2': ffmpegBuildsFile(ffBuildsRel, '-linux64-gpl.tar.xz', 'ffmpeg'),
        'linux-aarch64-v2': ffmpegBuildsFile(ffBuildsRel, '-linuxarm64-gpl.tar.xz', 'ffmpeg'),
        'windows-x86_64': { url: assetUrl(ffRel, 'ffmpeg-win32-x64') },
        'windows-aarch64': ffmpegBuildsFile(ffBuildsRel, '-winarm64-gpl.zip', 'ffmpeg.exe'),
      },
    },
    {
      name: 'ffprobe',
      version: ffRel.tag_name,
      files: {
        'linux-x86_64': { url: assetUrl(ffRel, 'ffprobe-linux-x64') },
        'linux-aarch64': { url: assetUrl(ffRel, 'ffprobe-linux-arm64') },
        'linux-x86_64-v2': ffmpegBuildsFile(ffBuildsRel, '-linux64-gpl.tar.xz', 'ffprobe'),
        'linux-aarch64-v2': ffmpegBuildsFile(ffBuildsRel, '-linuxarm64-gpl.tar.xz', 'ffprobe'),
        'windows-x86_64': { url: assetUrl(ffRel, 'ffprobe-win32-x64') },
        'windows-aarch64': ffmpegBuildsFile(ffBuildsRel, '-winarm64-gpl.zip', 'ffprobe.exe'),
      },
    },
    {
      name: 'deno',
      version: denoRel.tag_name,
      files: {
        'linux-x86_64': { entry: 'deno', url: assetUrl(denoRel, 'deno-x86_64-unknown-linux-gnu.zip') },
        'linux-aarch64': { entry: 'deno', url: assetUrl(denoRel, 'deno-aarch64-unknown-linux-gnu.zip') },
        'windows-x86_64': { entry: 'deno.exe', url: assetUrl(denoRel, 'deno-x86_64-pc-windows-msvc.zip') },
        'windows-aarch64': { entry: 'deno.exe', url: assetUrl(denoRel, 'deno-aarch64-pc-windows-msvc.zip') },
      },
    },
    {
      name: 'AtomicParsley',
      version: apRel.tag_name,
      files: {
        'linux-x86_64': { entry: 'AtomicParsley', url: assetUrl(apRel, 'AtomicParsleyLinux.zip') },
        'windows-x86_64': { entry: 'AtomicParsley.exe', url: assetUrl(apRel, 'AtomicParsleyWindows.zip') },
        'windows-aarch64': { entry: 'AtomicParsley.exe', url: assetUrl(apRel, 'AtomicParsleyWindows.zip') },
      },
    },
  ];
}

export const TOOL_LICENSES: Record<string, ToolLicense> = {
  'yt-dlp': {
    license: 'Unlicense',
    url: 'https://github.com/yt-dlp/yt-dlp',
  },
  'ffmpeg': {
    license: 'GPL-3.0-or-later',
    url: 'https://ffmpeg.org/',
  },
  'ffprobe': {
    license: 'GPL-3.0-or-later',
    url: 'https://ffmpeg.org/',
  },
  'Deno': {
    license: 'MIT',
    url: 'https://deno.land/',
  },
  'AtomicParsley': {
    license: 'GPL-2.0-or-later',
    url: 'https://github.com/wez/atomicparsley',
  },
};
