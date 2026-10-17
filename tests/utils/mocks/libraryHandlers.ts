import { ensureInvokeArgsObject, IPCHandler } from '../tauriMock';
import { InvokeArgs } from '@tauri-apps/api/core';
import type { HistoryEntry, Library, QueuedLink } from '../../../src/tauri/types/library.ts';

declare global {
  // Lets an e2e test start with a saved history or queue (set it with page.addInitScript).
  var __E2E_LIBRARY__: Library | undefined;
}

// Tauri sends the arguments as JSON, so the mock copies them the same way.
const copy = <T>(value: T): T => JSON.parse(JSON.stringify(value)) as T;

function library(): Library {
  globalThis.__E2E_LIBRARY__ ??= { history: [], queue: [] };
  return globalThis.__E2E_LIBRARY__;
}

export const libraryHandlers: Record<string, IPCHandler> = {
  library_get: () => copy(library()),
  history_add: (_cmd, args: InvokeArgs | undefined) => {
    const { entry } = ensureInvokeArgsObject<{ entry: HistoryEntry }>(args, 'history_add');
    const lib = library();
    lib.history = [copy(entry), ...lib.history.filter(e => e.id !== entry.id)];
    return copy(lib.history);
  },
  history_remove: (_cmd, args: InvokeArgs | undefined) => {
    const { id } = ensureInvokeArgsObject<{ id: string }>(args, 'history_remove');
    const lib = library();
    lib.history = lib.history.filter(e => e.id !== id);
    return copy(lib.history);
  },
  history_clear: () => {
    library().history = [];
    return [];
  },
  queue_save: (_cmd, args: InvokeArgs | undefined) => {
    const { queue } = ensureInvokeArgsObject<{ queue: QueuedLink[] }>(args, 'queue_save');
    library().queue = copy(queue);
    return null;
  },
};
