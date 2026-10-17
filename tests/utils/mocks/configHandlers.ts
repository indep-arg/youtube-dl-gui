import { ensureInvokeArgsObject, IPCHandler } from '../tauriMock';
import { InvokeArgs } from '@tauri-apps/api/core';
import { defaultSettings, type Settings } from '../../../src/tauri/types/config.ts';

declare global {
  // What config_import returns in an e2e test (set it with page.evaluate).
  var __E2E_IMPORTED_SETTINGS__: Settings | undefined;
}

type Json = Record<string, unknown>;

// Tauri sends the arguments as JSON, so the mock copies them the same way
// (structuredClone would fail on Vue's reactive objects).
const copy = <T>(value: T): T => JSON.parse(JSON.stringify(value)) as T;

// Same merge as the backend (json_merge): objects are merged, anything else replaces.
function merge(base: Json, patch: Json): Json {
  const out: Json = { ...base };
  for (const [key, value] of Object.entries(patch)) {
    const current = out[key];
    const isObject = (v: unknown) => !!v && typeof v === 'object' && !Array.isArray(v);
    out[key] = isObject(current) && isObject(value) ? merge(current as Json, value as Json) : value;
  }
  return out;
}

let current: Settings = copy(defaultSettings);

export const configHandlers: Record<string, IPCHandler> = {
  config_get: () => copy(current),
  config_set: (_cmd, args: InvokeArgs | undefined) => {
    const { patch } = ensureInvokeArgsObject<{ patch: Json }>(args, 'config_set');
    current = merge(current as unknown as Json, patch) as unknown as Settings;
    return copy(current);
  },
  config_reset: () => {
    current = copy(defaultSettings);
    return copy(current);
  },
  config_export: () => true,
  config_import: () => {
    const imported = globalThis.__E2E_IMPORTED_SETTINGS__;
    if (!imported) return null;
    current = merge(current as unknown as Json, imported as unknown as Json) as unknown as Settings;
    return copy(current);
  },
};
