// WebDriver tests of the built app, with the tools it installs. Linux only.
// See tests/app/README.md.
//
//   node tests/app/run.ts                 offline tests (needs the tools cache)
//   node tests/app/run.ts --network       also the tests that download the tools
//   node tests/app/run.ts --only name,…   run some tests
import { execFileSync } from 'node:child_process';
import {
  copyFileSync, cpSync, existsSync, linkSync, lstatSync, mkdirSync, readFileSync, readdirSync, rmSync, unlinkSync,
  writeFileSync,
} from 'node:fs';
import { homedir } from 'node:os';
import { delimiter, join, resolve } from 'node:path';
import { Session, TauriDriver } from './webdriver.ts';
import { makeClips, probe, serveMedia, type MediaServer } from './media.ts';

const ROOT = resolve(import.meta.dirname, '../..');
const CACHE = process.env.OVD_TEST_CACHE ?? join(homedir(), '.cache', 'ovd-app-tests');
const TOOLS = join(CACHE, 'tools');
const RUNS = join(CACHE, 'runs');
const RESULTS = join(ROOT, 'test-results', 'app');
const DATA_ID = 'io.github.indep-arg.youtube-dl-gui';
const APP_VERSION = (JSON.parse(readFileSync(join(ROOT, 'package.json'), 'utf8')) as { version: string }).version;

const argv = process.argv.slice(2);
const NETWORK = argv.includes('--network');
const KEEP = argv.includes('--keep');
const ONLY = argv.includes('--only') ? argv[argv.indexOf('--only') + 1].split(',') : null;

function which(name: string): string | null {
  for (const dir of (process.env.PATH ?? '').split(delimiter)) {
    const file = join(dir, name);
    if (existsSync(file)) return file;
  }
  return null;
}

function setup() {
  const app = process.env.OVD_APP ?? join(ROOT, 'src-tauri/target/release/open-video-downloader');
  const tauriDriver = process.env.TAURI_DRIVER ?? which('tauri-driver') ?? join(homedir(), '.cargo/bin/tauri-driver');
  const webkitDriver = process.env.WEBKIT_DRIVER ?? which('WebKitWebDriver') ?? join(CACHE, 'WebKitWebDriver');
  const missing = [
    [app, 'the app: build it with "npx tauri build --no-bundle" or set OVD_APP'],
    [tauriDriver, 'tauri-driver: "cargo install tauri-driver --locked" or set TAURI_DRIVER'],
    [webkitDriver, 'WebKitWebDriver: see tests/app/README.md or set WEBKIT_DRIVER'],
  ].filter(([file]) => !existsSync(file));
  if (missing.length) {
    for (const [, how] of missing) console.error(`missing ${how}`);
    process.exit(2);
  }
  if (appIsRunning()) {
    console.error('Open Video Downloader is running. Close it first: only one copy of the app can run.');
    process.exit(2);
  }
  return { app, tauriDriver, webkitDriver };
}

const env = setup();
mkdirSync(RUNS, { recursive: true });
mkdirSync(RESULTS, { recursive: true });

/** Copies a folder with hard links, so each test gets its own tools almost for free. */
function linkTree(from: string, to: string): void {
  mkdirSync(to, { recursive: true });
  for (const name of readdirSync(from)) {
    const source = join(from, name);
    const target = join(to, name);
    const stat = lstatSync(source);
    if (stat.isDirectory()) linkTree(source, target);
    else if (name === 'metadata.json') copyFileSync(source, target); // the app rewrites it
    else linkSync(source, target);
  }
}

function appIsRunning(): boolean {
  try {
    execFileSync('pgrep', ['-x', 'open-video-down']);
    return true;
  } catch {
    return false;
  }
}

const isProgram = (file: string) => existsSync(file) && readFileSync(file).subarray(0, 4).equals(Buffer.from([0x7f, 0x45, 0x4c, 0x46]));

class Context {
  readonly dir: string;
  readonly data: string;
  readonly appData: string;
  readonly downloads: string;
  readonly config: string;
  readonly driver = new TauriDriver();
  session: Session | null = null;

  readonly name: string;
  readonly media: MediaServer;

  constructor(name: string, media: MediaServer) {
    this.name = name;
    this.media = media;
    this.dir = join(RUNS, `${name}-${process.pid}`);
    rmSync(this.dir, { recursive: true, force: true });
    this.data = join(this.dir, 'data');
    this.appData = join(this.data, DATA_ID);
    this.downloads = join(this.dir, 'downloads');
    this.config = join(this.dir, 'config');
    for (const dir of [this.appData, this.downloads, this.config]) mkdirSync(dir, { recursive: true });
    // Downloads go to the test folder: the app takes its default folder from here.
    writeFileSync(join(this.config, 'user-dirs.dirs'), `XDG_DOWNLOAD_DIR="${this.downloads}"\nXDG_VIDEOS_DIR="${this.downloads}"\nXDG_MUSIC_DIR="${this.downloads}"\n`);
    this.writeConfig({});
  }

  get bin(): string {
    return join(this.appData, 'bin');
  }

  seedTools(): void {
    linkTree(TOOLS, this.bin);
  }

  /** Settings that keep the tests apart from the network and from the user's desktop. */
  writeConfig(extra: Record<string, Record<string, unknown>>): void {
    const config: Record<string, Record<string, unknown>> = {
      appearance: { language: 'en', theme: 'light' },
      input: { autoFillClipboard: false, globalShortcuts: false },
      update: { updateBinaries: false, updateApp: false },
      notifications: { notificationBehavior: 'never' },
    };
    for (const [section, values] of Object.entries(extra)) config[section] = { ...config[section], ...values };
    writeFileSync(join(this.appData, 'config.store.json'), JSON.stringify({ config }));
  }

  async start(): Promise<Session> {
    await this.driver.start({
      tauriDriver: env.tauriDriver,
      webkitDriver: env.webkitDriver,
      env: {
        ...process.env,
        XDG_DATA_HOME: this.data,
        XDG_CONFIG_HOME: this.config,
        LANG: 'en_US.UTF-8',
        LANGUAGE: 'en',
      },
    });
    this.session = new Session(this.driver.port);
    await this.session.open(env.app);
    return this.session;
  }

  async stop(): Promise<void> {
    await this.session?.close();
    this.session = null;
    await this.driver.stop();
    // The app writes to its data folder while it closes: wait until it is gone.
    const end = Date.now() + 10_000;
    while (Date.now() < end && appIsRunning()) await new Promise(r => setTimeout(r, 200));
  }

  async restart(): Promise<Session> {
    await this.stop();
    return this.start();
  }

  downloaded(ext: string): string[] {
    return readdirSync(this.downloads).filter(f => f.endsWith(`.${ext}`)).map(f => join(this.downloads, f));
  }
}

const homeReady = (s: Session) => s.waitForSelector('header input[type="text"]', 60_000);

async function waitForCard(s: Session, title: string, timeoutMs = 60_000): Promise<void> {
  await s.waitFor(`a ready card for ${title}`, () => s.exec<boolean>(`
    const card = [...document.querySelectorAll('article')].find(a => a.querySelector('h2')?.innerText.trim() === arguments[0]);
    return !!card && !!card.querySelector('[title="Download individually"]');`, title), timeoutMs);
}

async function cardTitles(s: Session): Promise<string[]> {
  return s.exec<string[]>('return [...document.querySelectorAll("article h2")].map(h => h.innerText.trim())');
}

function check(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message);
}

interface TestCase {
  name: string;
  network?: boolean;
  run: (ctx: Context) => Promise<void>;
}

const tests: TestCase[] = [
  {
    name: 'install-from-scratch',
    network: true,
    async run(ctx) {
      ctx.writeConfig({ update: { updateBinaries: true } });
      const s = await ctx.start();
      await s.waitForText('yt-dlp', 30_000);
      let sawReuse = false;
      await s.waitFor('the tools to install', async () => {
        const state = await s.exec<{ errors: number; reused: boolean; home: boolean }>(`
          return {
            errors: [...document.querySelectorAll('article details')].length,
            reused: document.body.innerText.includes('Already downloaded together with another tool.'),
            home: !!document.querySelector('header input[type="text"]'),
          };`);
        check(state.errors === 0, 'a tool showed an installation error');
        sawReuse ||= state.reused;
        return state.home;
      }, 300_000);
      check(sawReuse, 'ffprobe did not reuse the ffmpeg archive');
      for (const tool of ['yt-dlp', 'ffmpeg', 'ffprobe', 'deno', 'AtomicParsley']) {
        check(isProgram(join(ctx.bin, tool)), `${tool} is not a program`);
      }
      await ctx.stop();
      rmSync(TOOLS, { recursive: true, force: true });
      cpSync(ctx.bin, TOOLS, { recursive: true });
    },
  },
  {
    name: 'start-and-layout',
    async run(ctx) {
      ctx.seedTools();
      const s = await ctx.start();
      await homeReady(s);
      await s.waitForText('Your queue is empty');
      for (const path of ['/', '/settings', '/settings/app', '/settings/network', '/settings/system', '/settings/about',
        '/location', '/authentication', '/subtitles', '/input-filters', '/history']) {
        await s.go(path);
        await s.waitFor(`route ${path}`, async () => (await s.route()) === path);
        await new Promise(r => setTimeout(r, 400));
        const problems = await s.layoutProblems();
        check(problems.length === 0, `${path}: ${problems.join('; ')}`);
      }
    },
  },
  {
    name: 'download-video',
    async run(ctx) {
      ctx.seedTools();
      const s = await ctx.start();
      await homeReady(s);
      await s.addLink(ctx.media.url('clip'));
      await waitForCard(s, 'clip');
      await s.click('article [title="Download individually"]');
      await s.waitForText('Download complete — 100%', 90_000);
      const files = ctx.downloaded('mp4');
      check(files.length === 1, `expected one mp4, found ${files.length}`);
      const info = probe(join(ctx.bin, 'ffprobe'), files[0]);
      check(Math.abs(info.duration - 6) < 0.5, `duration ${info.duration}`);
      check(info.streams.includes('video') && info.streams.includes('audio'), `streams ${info.streams.join(',')}`);
      await s.go('/history');
      await s.waitForText('clip', 10_000);
    },
  },
  {
    name: 'download-audio',
    async run(ctx) {
      ctx.seedTools();
      const s = await ctx.start();
      await homeReady(s);
      await s.addLink(ctx.media.url('clip'));
      await waitForCard(s, 'clip');
      await s.exec(`
        const select = document.querySelector('article select');
        select.value = 'audio';
        select.dispatchEvent(new Event('change', { bubbles: true }));
        return true;`);
      await s.click('article [title="Download individually"]');
      await s.waitForText('Download complete — 100%', 90_000);
      const files = ctx.downloaded('mp3');
      check(files.length === 1, `expected one mp3, found ${readdirSync(ctx.downloads).join(', ')}`);
      const info = probe(join(ctx.bin, 'ffprobe'), files[0]);
      check(info.codecs.includes('mp3') && !info.streams.includes('video'), `streams ${info.codecs.join(',')}`);
    },
  },
  {
    name: 'error-card',
    async run(ctx) {
      ctx.seedTools();
      const s = await ctx.start();
      await homeReady(s);
      await s.addLink(ctx.media.missingUrl);
      await s.waitForText('Error -', 60_000);
      await s.waitForText('1 download failed.', 10_000);
      await s.click('article a', 'View logs');
      await s.waitForText('Errors (1)', 10_000);
    },
  },
  {
    name: 'queue-restore',
    async run(ctx) {
      ctx.seedTools();
      let s = await ctx.start();
      await homeReady(s);
      await s.addLink(ctx.media.url('clip'));
      await waitForCard(s, 'clip');
      await s.addLink(ctx.media.url('second'));
      await waitForCard(s, 'second');
      const before = await cardTitles(s);
      await new Promise(r => setTimeout(r, 2_500)); // the queue is saved a second after it changes
      s = await ctx.restart();
      await homeReady(s);
      await s.waitForText('Restored 2 links', 30_000);
      await waitForCard(s, 'clip');
      await waitForCard(s, 'second');
      const after = await cardTitles(s);
      check(JSON.stringify(after) === JSON.stringify(before), `order ${after.join(',')} instead of ${before.join(',')}`);
    },
  },
  {
    name: 'language',
    async run(ctx) {
      ctx.seedTools();
      let s = await ctx.start();
      await homeReady(s);
      await s.go('/settings/app');
      await s.waitForText('Language:');
      await s.exec(`
        const select = [...document.querySelectorAll('select')].find(x => [...x.options].some(o => o.value === 'es'));
        select.value = 'es';
        select.dispatchEvent(new Event('change', { bubbles: true }));
        return true;`);
      await s.click('nav button', 'Save');
      const spanish = async () => (await s.exec<string>('return document.querySelector(\'header input[type="text"]\').placeholder')).startsWith('Ingresa');
      await s.waitFor('the Spanish text', spanish, 10_000);
      s = await ctx.restart();
      await homeReady(s);
      await s.waitFor('Spanish after a restart', spanish, 10_000);
      check(await s.exec<string>('return document.documentElement.lang') === 'es', 'the page language is not es');
    },
  },
  {
    name: 'diagnostics',
    async run(ctx) {
      ctx.seedTools();
      const s = await ctx.start();
      await homeReady(s);
      const canPaste = !!process.env.WAYLAND_DISPLAY && !!which('wl-paste') && !!which('wl-copy');
      const previous = canPaste ? execFileSync('wl-paste', ['--no-newline'], { timeout: 3_000 }).toString() : '';
      await s.go('/settings/about');
      await s.click('#copy-diagnostics');
      await s.waitForText('Tools:', 60_000);
      const text = await s.exec<string>('return document.querySelector("#diagnostics-text").innerText');
      check(text.startsWith(`Open Video Downloader ${APP_VERSION}`), `first line: ${text.split('\n')[0]}`);
      for (const tool of ['yt-dlp', 'ffmpeg', 'ffprobe', 'deno', 'AtomicParsley']) {
        check(new RegExp(`^- ${tool} \\S+: runs \\(`, 'm').test(text), `${tool} does not run:\n${text}`);
      }
      check(!text.includes(homedir()), 'the report contains a path');
      if (canPaste) {
        const copied = execFileSync('wl-paste', ['--no-newline'], { timeout: 3_000 }).toString();
        // wl-copy keeps running to serve the clipboard: do not wait for its output.
        execFileSync('wl-copy', [previous], { stdio: 'ignore', timeout: 3_000 });
        check(copied === text.trimEnd() || copied.trimEnd() === text.trimEnd(), 'the clipboard does not hold the report');
      }
    },
  },
  {
    name: 'repair-broken-tool',
    network: true,
    async run(ctx) {
      ctx.seedTools();
      const ffmpeg = join(ctx.bin, 'ffmpeg');
      unlinkSync(ffmpeg); // it is a hard link to the cache: replace it, do not write through it
      writeFileSync(ffmpeg, Buffer.from([0xfd, 0x37, 0x7a, 0x58, 0x5a, 0x00]));
      ctx.writeConfig({ update: { updateBinaries: true } });
      const s = await ctx.start();
      await s.waitFor('the install screen with ffmpeg', async () => {
        const text = await s.text();
        return text.includes('ffmpeg') && !text.includes('Your queue is empty');
      }, 60_000);
      await homeReady(s);
      check(isProgram(ffmpeg), 'ffmpeg was not installed again');
    },
  },
  {
    name: 'small-window',
    async run(ctx) {
      ctx.seedTools();
      const s = await ctx.start();
      await homeReady(s);
      await s.setWindowSize(750, 650);
      await s.addLink(ctx.media.url('clip'));
      await waitForCard(s, 'clip');
      for (const path of ['/', '/settings', '/settings/app', '/settings/about']) {
        await s.go(path);
        await new Promise(r => setTimeout(r, 500));
        const problems = await s.layoutProblems();
        check(problems.length === 0, `${path} at ${await s.exec<number>('return innerWidth')}px: ${problems.join('; ')}`);
      }
    },
  },
];

async function main(): Promise<void> {
  if (!existsSync(join(TOOLS, 'ffmpeg')) && !NETWORK) {
    console.error(`No tools in ${TOOLS}. Run once with --network to install them.`);
    process.exit(2);
  }
  const selected = tests.filter(t => (!ONLY || ONLY.includes(t.name)) && (NETWORK || !t.network));
  // The tool cache must exist before the tests that copy it.
  selected.sort((a, b) => Number(b.name === 'install-from-scratch') - Number(a.name === 'install-from-scratch'));

  let media: MediaServer | null = null;
  const results: { name: string; ok: boolean; seconds: number; error?: string }[] = [];
  for (const test of selected) {
    if (!media && existsSync(join(TOOLS, 'ffmpeg'))) {
      makeClips(join(TOOLS, 'ffmpeg'), join(CACHE, 'media'));
      media = await serveMedia(join(CACHE, 'media'));
    }
    const ctx = new Context(test.name, media ?? { url: () => '', missingUrl: '', close: async () => {} });
    const started = Date.now();
    process.stdout.write(`${test.name} … `);
    try {
      await test.run(ctx);
      results.push({ name: test.name, ok: true, seconds: (Date.now() - started) / 1000 });
      console.log('ok');
    } catch (e) {
      const error = e instanceof Error ? e.message : String(e);
      results.push({ name: test.name, ok: false, seconds: (Date.now() - started) / 1000, error });
      console.log(`FAILED\n  ${error}`);
      try {
        await ctx.session?.screenshot(join(RESULTS, `${test.name}.png`));
        console.log(`  screenshot: ${join(RESULTS, `${test.name}.png`)}`);
      } catch {
        // no window to capture
      }
    } finally {
      await ctx.stop();
      // WebKit can still be writing its cache for a moment after the app closes.
      if (!KEEP) rmSync(ctx.dir, { recursive: true, force: true, maxRetries: 5, retryDelay: 500 });
    }
  }
  await media?.close();

  const failed = results.filter(r => !r.ok);
  console.log(`\n${results.length - failed.length} passed, ${failed.length} failed`);
  for (const r of results) console.log(`  ${r.ok ? 'ok    ' : 'FAILED'} ${r.name} (${r.seconds.toFixed(1)} s)`);
  process.exit(failed.length ? 1 : 0);
}

await main();
