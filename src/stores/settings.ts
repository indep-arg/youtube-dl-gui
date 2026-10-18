import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { resolveLocale, setLocale } from '../i18n';
import { defaultSettings } from '../tauri/types/config.ts';
import { Settings } from '../tauri/types/config.ts';

type DeepPartial<T> = T extends object ? {
  [P in keyof T]?: DeepPartial<T[P]>;
} : T;

export const useSettingsStore = defineStore('settings', () => {
  // A copy: the store changes its settings in place, and the defaults must stay as they are.
  const settings = ref<Settings>(structuredClone(defaultSettings));

  async function load(): Promise<Settings> {
    const cfg = await invoke<Settings>('config_get');
    applySettings(cfg);
    return cfg;
  }

  async function patch(partial: DeepPartial<Settings>) {
    const newCfg = await invoke<Settings>('config_set', { patch: partial });
    applySettings(newCfg);
  }

  async function reset() {
    const cfg = await invoke<Settings>('config_reset');
    applySettings(cfg);
    return cfg;
  }

  /** Returns false when the save dialog was cancelled. */
  async function exportSettings(): Promise<boolean> {
    return await invoke<boolean>('config_export');
  }

  /** Returns the new settings, or null when the file dialog was cancelled. */
  async function importSettings(): Promise<Settings | null> {
    const cfg = await invoke<Settings | null>('config_import');
    if (cfg) applySettings(cfg);
    return cfg;
  }

  function hasAuthConfigured() {
    return settings.value.auth.cookieFile !== null || settings.value.auth.cookieBrowser !== 'none';
  }

  function applySettings(cfg: Settings) {
    Object.assign(settings.value, cfg);

    if (cfg.appearance.language) {
      void setLocale(resolveLocale(cfg.appearance.language));
    }
  }

  return {
    settings,
    load,
    patch,
    reset,
    exportSettings,
    importSettings,
    hasAuthConfigured,
  };
});
