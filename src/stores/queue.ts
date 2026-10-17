import { defineStore } from 'pinia';
import { watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { Library, QueuedLink } from '../tauri/types/library.ts';
import type { DownloadOptions, DownloadOverrides } from '../tauri/types/media.ts';
import { MediaState, useMediaStateStore } from './media/state.ts';
import { useMediaGroupStore } from './media/group.ts';
import { useMediaOptionsStore } from './media/options.ts';
import { useMediaStore } from './media/media.ts';
import { useSettingsStore } from './settings.ts';

const SAVE_DELAY_MS = 1000;

/** Keeps the links of the queue between sessions. */
export const useQueueStore = defineStore('queue', () => {
  const groupStore = useMediaGroupStore();
  const stateStore = useMediaStateStore();
  const optionsStore = useMediaOptionsStore();
  const settingsStore = useSettingsStore();

  let isSaving = false;
  let timer: ReturnType<typeof setTimeout> | undefined;

  /**
   * The links that are not finished yet, oldest first; failed ones are left out.
   * The queue shows the newest link on top and {@link restore} adds them one by
   * one, so saving them oldest first keeps the same order after a restart.
   */
  function pendingLinks(): QueuedLink[] {
    return [...groupStore.orderedGroups]
      .reverse()
      .filter((group) => {
        const state = stateStore.getGroupState(group.id);
        return !!group.url && state !== MediaState.done && state !== MediaState.error;
      })
      .map(group => ({
        url: group.url,
        overrides: optionsStore.getOverrides(group.id) ?? null,
        options: optionsStore.getOptions(group.id) ?? null,
      }));
  }

  async function save(): Promise<void> {
    const queue = settingsStore.settings.input.restoreQueue ? pendingLinks() : [];
    await invoke('queue_save', { queue });
  }

  function scheduleSave() {
    clearTimeout(timer);
    timer = setTimeout(() => {
      save().catch(e => console.error('Unable to save the queue:', e));
    }, SAVE_DELAY_MS);
  }

  /** Saves the queue whenever it changes. Start it after {@link restore}, or the saved queue is lost. */
  function startSaving() {
    if (isSaving) return;
    isSaving = true;
    watch(
      [
        () => groupStore.groupOrder,
        () => stateStore.itemStates,
        () => optionsStore.groupOptions,
        () => optionsStore.groupOverrides,
        () => settingsStore.settings.input.restoreQueue,
      ],
      scheduleSave,
      { deep: true },
    );
  }

  /** Adds the links that were in the queue last time. Returns how many there were. */
  async function restore(): Promise<number> {
    if (!settingsStore.settings.input.restoreQueue) return 0;
    const library = await invoke<Library>('library_get');
    const mediaStore = useMediaStore();
    for (const link of library.queue) {
      if (!link.url) continue;
      const groupId = await mediaStore.dispatchMediaInfoFetch(link.url);
      if (link.overrides) optionsStore.setOverrides(groupId, link.overrides as DownloadOverrides);
      if (link.options) optionsStore.setOptions(groupId, link.options as DownloadOptions);
    }
    return library.queue.length;
  }

  return { pendingLinks, save, startSaving, restore };
});
