import { IPCHandler } from '../tauriMock';
import { emit } from '@tauri-apps/api/event';
import {
  BinaryCheckPayload,
  BinaryDownloadCompletePayload,
  BinaryDownloadProgressPayload,
  BinaryDownloadStartPayload,
} from '../../../src/tauri/types/binaries';
import { savedSettings } from './configHandlers';

export type BinariesMockData = {
  check?: string[];
  ensure?: boolean;
};

// The yt-dlp channel that is installed: choosing the other one installs yt-dlp again.
let installedChannel = 'stable';

export const binaryHandlers: Record<string, IPCHandler> = {
  binaries_check: (): BinaryCheckPayload => {
    const tools = [...(window.E2E.binaries?.check || [])];
    if (savedSettings().update.ytDlpChannel !== installedChannel && !tools.includes('yt-dlp')) tools.push('yt-dlp');
    return { tools };
  },
  binaries_ensure: async (): Promise<void> => {
    installedChannel = savedSettings().update.ytDlpChannel;
    if (window.E2E.binaries?.ensure) {
      return new Promise((resolve): void => {
        void emit<BinaryDownloadStartPayload>('binary_download_start', {
          tool: window.E2E.binaries?.check[0] ?? 'yt-dlp',
          version: '1.2.3',
        });
        setTimeout((): void => {
          void emit<BinaryDownloadProgressPayload>('binary_download_progress', {
            tool: window.E2E.binaries?.check[0] ?? 'yt-dlp',
            received: 1024 * 1024 * 5,
            total: 1024 * 1024 * 10,
            reused: false,
          });
        }, 1000);
        setTimeout((): void => {
          void emit<BinaryDownloadCompletePayload>('binary_download_complete', {
            tool: window.E2E.binaries?.check[0] ?? 'yt-dlp',
          });
        }, 2000);
        setTimeout(resolve, 3000);
      });
    } else {
      return Promise.resolve();
    }
  },
};
