import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { HistoryEntry, Library } from '../tauri/types/library.ts';
import type { Group } from '../tauri/types/group.ts';
import { useMediaDestinationStore } from './media/destination.ts';

export const useHistoryStore = defineStore('history', () => {
  const entries = ref<HistoryEntry[]>([]);
  const destinationStore = useMediaDestinationStore();

  async function load(): Promise<HistoryEntry[]> {
    const library = await invoke<Library>('library_get');
    entries.value = library.history;
    return entries.value;
  }

  /** Adds a finished download. A group that is downloaded again replaces its old entry. */
  async function record(group: Group): Promise<void> {
    const files = Object.values(group.items).filter(item => !item.isLeader).length;
    const entry: HistoryEntry = {
      id: group.id,
      url: group.url,
      title: group.playlistTitle ?? group.title ?? null,
      thumbnail: group.thumbnail ?? null,
      uploader: group.playlistUploader ?? group.uploader ?? null,
      path: destinationStore.getPrimaryDestination(group.id)?.path ?? null,
      items: group.isCombined ? Math.max(files, 1) : 1,
      finishedAt: Date.now(),
      isLive: group.isLive ?? false,
    };
    entries.value = await invoke<HistoryEntry[]>('history_add', { entry });
  }

  async function remove(id: string): Promise<void> {
    entries.value = await invoke<HistoryEntry[]>('history_remove', { id });
  }

  async function clear(): Promise<void> {
    entries.value = await invoke<HistoryEntry[]>('history_clear');
  }

  return { entries, load, record, remove, clear };
});
