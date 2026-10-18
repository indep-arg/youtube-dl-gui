import { IPCHandler } from '../tauriMock';
import { InvokeArgs } from '@tauri-apps/api/core';

declare global {
  // What the app last wrote to the clipboard, so an e2e test can read it.
  var __E2E_CLIPBOARD__: string | undefined;
}

const E2E_DIAGNOSTICS = [
  'Open Video Downloader 2026.10.4',
  'System: linux x86_64 (Test Linux)',
  'Install: installed',
  '',
  'Tools:',
  '- yt-dlp 2026.08.19: runs (2026.08.19)',
].join('\n');

export const appHandlers: Record<string, IPCHandler> = {
  'app_diagnostics': () => E2E_DIAGNOSTICS,
  'plugin:clipboard-manager|write_text': (_cmd, args: InvokeArgs | undefined) => {
    globalThis.__E2E_CLIPBOARD__ = (args as { text?: string } | undefined)?.text;
    return null;
  },
  // The backend returns what is logged so far (a string, maybe empty).
  'logging_subscribe': () => '',
  'logging_unsubscribe': () => null,
};
