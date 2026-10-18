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
    if (url.includes('empty-playlist')) {
      void emitEmptyPlaylist(groupId, id);
      return groupId;
    }
    if (url.includes('site-changed')) {
      void emitSiteChanged(groupId, id);
      return groupId;
    }
    if (url.includes('scheduled')) {
      void emit<MediaAddPayload>('media_add', {
        groupId,
        total: 1,
        item: {
          id, url, title: 'Scheduled Stream', audioCodecs: [], formats: [], filesize: 0, isLeader: true,
          isLive: true, scheduledAt: Date.UTC(2030, 0, 15, 18, 0) / 1000,
        },
      });
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
      } else if (item.url.includes('live')) {
        void startRecording(groupId, item.id);
      } else if (item.url.includes('scheduled')) {
        void emit('media_progress_stage', { groupId, id: item.id, stage: 'waiting' });
      }
    }
    return null;
  },
  // Stops a live recording and saves it, like the backend does.
  group_stop: (_cmd: string, args: InvokeArgs | undefined): null => {
    const { groupId } = ensureInvokeArgsObject<{ groupId: string }>(args, 'group_stop');
    const id = recordings.get(groupId);
    if (id) {
      recordings.delete(groupId);
      void saveRecording(groupId, id);
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

const recordings = new Map<string, string>();

async function startRecording(groupId: string, id: string): Promise<void> {
  recordings.set(groupId, id);
  await emit('media_progress_stage', { groupId, id, stage: 'downloading' });
  await emit('media_progress', {
    groupId, id, category: 'video', speedBps: 104_857, downloadedBytes: 3_145_728, recordedSecs: 75,
  });
}

async function saveRecording(groupId: string, id: string): Promise<void> {
  await emit('media_progress_stage', { groupId, id, stage: 'finalizing' });
  await new Promise(resolve => setTimeout(resolve, 300));
  await emit('media_destination', {
    groupId,
    id,
    destination: { path: '/downloads/Live Video.mp4', confidence: 100 },
  });
  await emit('media_complete', { groupId, id });
}

async function finishDownload(groupId: string, id: string): Promise<void> {
  await emit('media_destination', {
    groupId,
    id,
    destination: { path: '/downloads/Test Video.mp4', confidence: 1 },
  });
  await emit('media_complete', { groupId, id });
}

// What the backend sends when yt-dlp no longer understands a site.
async function emitSiteChanged(groupId: string, id: string): Promise<void> {
  const raw = 'ERROR: [youtube] abc: Unable to extract initial data';
  await emit('media_diagnostic', {
    groupId, id, level: 'error', code: 'unableToExtractInitialData', component: 'youtube',
    message: 'Unable to extract initial data', raw, timestamp: Date.now(),
  });
  await emit('media_fatal', {
    groupId, id, exitCode: 1, internal: false, message: raw, details: null, timestamp: Date.now(),
  });
}

// What the backend sends for a playlist without entries.
async function emitEmptyPlaylist(groupId: string, id: string): Promise<void> {
  const message = 'This playlist has no videos, or none of them can be downloaded.';
  await emit('media_diagnostic', {
    groupId, id, level: 'error', code: 'playlistEmpty', component: null, message, raw: '', timestamp: Date.now(),
  });
  await emit('media_fatal', {
    groupId, id, exitCode: 1, internal: false, message, details: null, timestamp: Date.now(),
  });
}
