import { ensureInvokeArgsObject, IPCHandler } from '../tauriMock';
import { InvokeArgs } from '@tauri-apps/api/core';
import { emit } from '@tauri-apps/api/event';
import { EntryItem, MediaAddPayload, MediaFormat } from '../../../src/tauri/types/media';

const sampleFormats: MediaFormat[] = [
  { id: 'f1080', height: 1080, fps: 60, audioCodecs: [], videoCodecs: [] },
  { id: 'f720', height: 720, fps: 30, audioCodecs: [], videoCodecs: [] },
  { id: 'audio', abr: 120, audioCodecs: [], videoCodecs: [] },
];

export const mediaHandlers: Record<string, IPCHandler> = {
  media_info: (_cmd: string, args: InvokeArgs | undefined): string => {
    const { groupId, id, url } = ensureInvokeArgsObject<{ groupId: string; id: string; url: string }>(args, 'media_info');

    if (url.includes('playlist-small')) {
      void emitPlaylistLeader(id, url, groupId, 2);
      return groupId;
    }
    if (url.includes('playlist-large')) {
      void emitPlaylistLeader(id, url, groupId, 50);
      return groupId;
    }
    if (url.includes('live')) {
      void emit<MediaAddPayload>('media_add', {
        groupId,
        total: 1,
        item: { id, url, title: 'Live Video', audioCodecs: ['aac'], formats: sampleFormats, filesize: 0, isLeader: true, isLive: true },
      });
      return groupId;
    }
    void emit<MediaAddPayload>('media_add', {
      groupId,
      total: 1,
      item: { id, url, title: 'Test Video', audioCodecs: ['aac'], formats: sampleFormats, filesize: 123, isLeader: true },
    });
    return 'test-group';
  },
  // Links with "finish" in them finish downloading right away; others keep downloading.
  media_download: (_cmd: string, args: InvokeArgs | undefined): null => {
    const { groupId, items } = ensureInvokeArgsObject<{ groupId: string; items: { id: string; url: string }[] }>(
      args,
      'media_download',
    );
    for (const item of items) {
      if (item.url.includes('finish')) {
        void finishDownload(groupId, item.id);
      }
    }
    return null;
  },
  media_playlist_expand: (_cmd: string, args: InvokeArgs | undefined): string => {
    const { groupId, entries } = ensureInvokeArgsObject<{ groupId: string; entries: EntryItem[] }>(
      args,
      'media_playlist_expand',
    );

    void emitExpandedPlaylist(groupId, entries);
    return groupId;
  },
};

const emitPlaylistLeader = async (id: string, url: string, groupId: string, total: number): Promise<string> => {
  const entries = Array.from({ length: total }, (_, index) => ({
    index,
    videoUrl: `${url}?v=${index + 1}`,
  }));

  await emit<MediaAddPayload>('media_add', {
    groupId,
    total,
    item: {
      id: `${id}-pl`,
      url,
      title: 'Playlist',
      audioCodecs: ['aac'],
      formats: sampleFormats,
      filesize: 0,
      entries,
      playlistCount: total,
    },
  });
  return groupId;
};

async function emitExpandedPlaylist(groupId: string, entries: EntryItem[]): Promise<void> {
  for (const entry of entries) {
    await emit<MediaAddPayload>('media_add', {
      groupId,
      total: entries.length,
      item: {
        id: `${groupId}-${entry.index}`,
        url: entry.videoUrl,
        title: `Item ${entry.index + 1}`,
        audioCodecs: ['aac'],
        formats: sampleFormats,
        filesize: 0,
      },
    });
  }
}

async function finishDownload(groupId: string, id: string): Promise<void> {
  await emit('media_destination', {
    groupId,
    id,
    destination: { path: '/downloads/Test Video.mp4', confidence: 1 },
  });
  await emit('media_complete', { groupId, id });
}
