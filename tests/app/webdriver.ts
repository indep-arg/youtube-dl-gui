// A small WebDriver client for the app, through tauri-driver.
import { spawn, type ChildProcess } from 'node:child_process';
import { createServer } from 'node:net';
import { writeFileSync } from 'node:fs';

interface DriverOptions {
  tauriDriver: string;
  webkitDriver: string;
  env: NodeJS.ProcessEnv;
}

async function freePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.once('error', reject);
    server.listen(0, '127.0.0.1', () => {
      const address = server.address();
      server.close(() => {
        if (address && typeof address === 'object') resolve(address.port);
        else reject(new Error('no free port'));
      });
    });
  });
}

const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

/** One tauri-driver process. Its environment is the environment of the app it starts. */
export class TauriDriver {
  private process: ChildProcess | null = null;
  port = 0;

  async start(options: DriverOptions): Promise<void> {
    this.port = await freePort();
    const nativePort = await freePort();
    this.process = spawn(
      options.tauriDriver,
      ['--port', String(this.port), '--native-port', String(nativePort), '--native-driver', options.webkitDriver],
      { env: options.env, stdio: 'ignore' },
    );
    const end = Date.now() + 15_000;
    while (Date.now() < end) {
      try {
        const res = await fetch(`http://127.0.0.1:${this.port}/status`);
        if (res.ok) return;
      } catch {
        // not listening yet
      }
      await sleep(150);
    }
    throw new Error('tauri-driver did not start');
  }

  async stop(): Promise<void> {
    const child = this.process;
    this.process = null;
    if (!child || child.exitCode !== null) return;
    const exited = new Promise(resolve => child.once('exit', resolve));
    child.kill('SIGTERM');
    await Promise.race([exited, sleep(5_000)]);
    if (child.exitCode === null) child.kill('SIGKILL');
  }
}

/** A session with the app: one running app window. */
export class Session {
  private readonly base: string;
  private id = '';

  constructor(port: number) {
    this.base = `http://127.0.0.1:${port}`;
  }

  private async call<T>(method: string, path: string, body?: unknown): Promise<T> {
    const res = await fetch(this.base + path, {
      method,
      headers: { 'content-type': 'application/json' },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
    const json = await res.json() as { value: T & { error?: string; message?: string } };
    if (json.value && typeof json.value === 'object' && 'error' in json.value && json.value.error) {
      throw new Error(`${method} ${path}: ${json.value.error} ${json.value.message ?? ''}`);
    }
    return json.value;
  }

  async open(app: string): Promise<void> {
    const value = await this.call<{ sessionId: string }>('POST', '/session', {
      capabilities: { alwaysMatch: { 'tauri:options': { application: app } } },
    });
    this.id = value.sessionId;
  }

  async close(): Promise<void> {
    if (!this.id) return;
    try {
      await this.call('DELETE', `/session/${this.id}`);
    } catch {
      // the app may already be gone
    }
    this.id = '';
  }

  async setWindowSize(width: number, height: number): Promise<void> {
    await this.call('POST', `/session/${this.id}/window/rect`, { width, height });
    await this.waitFor(`a window of ${width} px`, async () => Math.abs(await this.exec<number>('return innerWidth') - width) < 20, 5_000);
  }

  exec<T>(script: string, ...args: unknown[]): Promise<T> {
    return this.call<T>('POST', `/session/${this.id}/execute/sync`, { script, args });
  }

  text(): Promise<string> {
    return this.exec<string>('return document.body.innerText');
  }

  async screenshot(file: string): Promise<void> {
    await this.exec('document.activeElement && document.activeElement.blur(); return true;');
    // The first capture can show a block that is not painted yet.
    await this.call('GET', `/session/${this.id}/screenshot`);
    await sleep(800);
    const png = await this.call<string>('GET', `/session/${this.id}/screenshot`);
    writeFileSync(file, Buffer.from(png, 'base64'));
  }

  /** Polls `check` until it returns something truthy. */
  async waitFor<T>(what: string, check: () => Promise<T>, timeoutMs = 30_000): Promise<T> {
    const end = Date.now() + timeoutMs;
    let last: unknown;
    while (Date.now() < end) {
      try {
        const value = await check();
        if (value) return value;
      } catch (e) {
        last = e;
      }
      await sleep(250);
    }
    throw new Error(`timed out waiting for ${what}${last ? ` (${String(last)})` : ''}`);
  }

  waitForText(text: string, timeoutMs?: number): Promise<boolean> {
    return this.waitFor(`text "${text}"`, async () => (await this.text()).includes(text), timeoutMs);
  }

  waitForSelector(selector: string, timeoutMs?: number): Promise<boolean> {
    return this.waitFor(
      `element ${selector}`,
      () => this.exec<boolean>('return !!document.querySelector(arguments[0])', selector),
      timeoutMs,
    );
  }

  /**
   * Clicks the first element matching `selector`, or the one whose text is `text` among them.
   * Waits for it to appear: right after `go()` the new page may not be shown yet.
   */
  async click(selector: string, text?: string): Promise<void> {
    await this.waitFor(`something to click: ${selector} ${text ?? ''}`, () => this.exec<boolean>(`
      const all = [...document.querySelectorAll(arguments[0])];
      const el = arguments[1] === null ? all[0] : all.find(e => e.innerText.trim() === arguments[1]);
      if (!el) return false;
      el.click();
      return true;`, selector, text ?? null), 10_000);
  }

  go(path: string): Promise<unknown> {
    return this.exec('document.getElementById("app").__vue_app__.config.globalProperties.$router.push(arguments[0]); return true;', path);
  }

  route(): Promise<string> {
    return this.exec<string>('return document.getElementById("app").__vue_app__.config.globalProperties.$router.currentRoute.value.path');
  }

  async addLink(url: string): Promise<void> {
    await this.exec(`
      const input = document.querySelector('header input[type="text"]');
      input.value = arguments[0];
      input.dispatchEvent(new Event('input', { bubbles: true }));
      return true;`, url);
    await this.click('header button', 'Add');
  }

  /** Elements that reach past the right edge of the window, and whether the page itself scrolled. */
  layoutProblems(): Promise<string[]> {
    return this.exec<string[]>(`
      const problems = [];
      if (document.documentElement.scrollWidth > innerWidth + 1) problems.push('page is wider than the window');
      if (document.body.scrollTop || document.documentElement.scrollTop) problems.push('page scrolled');
      for (const e of document.querySelectorAll('body *')) {
        const style = getComputedStyle(e);
        if (style.display === 'none' || style.visibility === 'hidden' || e.closest('.sr-only, [aria-hidden="true"], .dropdown-content')) continue;
        const box = e.getBoundingClientRect();
        if (box.width && box.height && box.right > innerWidth + 1) {
          problems.push(e.tagName.toLowerCase() + '.' + String(e.className).split(' ').slice(0, 3).join('.') + ' ends at ' + Math.round(box.right));
        }
      }
      return problems.slice(0, 5);`);
  }
}
