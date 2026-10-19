import { beforeEach, describe, expect, it } from 'vitest';
import { installTauriMock } from '../utils/tauriMock';
import { mediaHandlers } from '../utils/mocks/mediaHandlers';
import { libraryHandlers } from '../utils/mocks/libraryHandlers';
import { useQueueStore } from '../../src/stores/queue';
import { useHistoryStore } from '../../src/stores/history';
import { useMediaGroupStore } from '../../src/stores/media/group';
import { MediaState, useMediaStateStore } from '../../src/stores/media/state';
import { useMediaOptionsStore } from '../../src/stores/media/options';
import { useMediaDestinationStore } from '../../src/stores/media/destination';
import { useSettingsStore } from '../../src/stores/settings';
import { TrackType } from '../../src/tauri/types/media';
import type { Group } from '../../src/tauri/types/group';

function makeGroup(id: string, url: string, extra: Partial<Group> = {}): Group {
  return {
    id,
    url,
    total: 1,
    processed: 0,
    errored: 0,
    isCombined: false,
    audioCodecs: [],
    formats: [],
    filesize: 0,
    items: {
      [`${id}-leader`]: { id: `${id}-leader`, url, isLeader: true, audioCodecs: [], formats: [], filesize: 0 },
    },
    ...extra,
  };
}

function addGroup(group: Group, state: MediaState) {
  useMediaGroupStore().createGroup(group);
  useMediaStateStore().setState(`${group.id}-leader`, state);
}

beforeEach(() => {
  globalThis.__E2E_LIBRARY__ = { history: [], queue: [] };
  installTauriMock({ ...mediaHandlers, ...libraryHandlers, notify: () => null });
});

describe('queue store', () => {
  it('keeps the links that are not finished and leaves out failed ones', () => {
    addGroup(makeGroup('a', 'https://example.com/a'), MediaState.configure);
    addGroup(makeGroup('b', 'https://example.com/b'), MediaState.done);
    addGroup(makeGroup('c', 'https://example.com/c'), MediaState.error);
    addGroup(makeGroup('d', 'https://example.com/d'), MediaState.downloading);
    useMediaOptionsStore().setOptions('a', { trackType: TrackType.audio });

    const links = useQueueStore().pendingLinks();

    expect(links.map(link => link.url)).toEqual(['https://example.com/a', 'https://example.com/d']);
    expect(links[0].options).toEqual({ trackType: TrackType.audio });
  });

  it('saves an empty queue when keeping it is turned off', async () => {
    addGroup(makeGroup('a', 'https://example.com/a'), MediaState.configure);
    useSettingsStore().settings.input.restoreQueue = false;

    await useQueueStore().save();

    expect(globalThis.__E2E_LIBRARY__?.queue).toEqual([]);
  });

  it('brings back the saved links with their options', async () => {
    globalThis.__E2E_LIBRARY__ = {
      history: [],
      queue: [{ url: 'https://example.com/back', overrides: { output: { addMetadata: false } }, options: { trackType: 'audio' } }],
    };

    const restored = await useQueueStore().restore();

    expect(restored).toBe(1);
    const group = useMediaGroupStore().orderedGroups[0];
    expect(group.url).toBe('https://example.com/back');
    expect(useMediaOptionsStore().getOptions(group.id)?.trackType).toBe('audio');
    expect(useMediaOptionsStore().getOverrides(group.id)?.output?.addMetadata).toBe(false);
  });

  it('fetches a restored link with its saved settings', async () => {
    const fetched: unknown[] = [];
    installTauriMock({
      ...mediaHandlers,
      ...libraryHandlers,
      notify: () => null,
      media_info: (cmd, args) => {
        fetched.push((args as { overrides?: unknown }).overrides);
        return mediaHandlers.media_info(cmd, args);
      },
    });
    globalThis.__E2E_LIBRARY__ = {
      history: [],
      queue: [{ url: 'https://example.com/private', overrides: { auth: { cookieFile: '/cookies.txt' } }, options: null }],
    };

    await useQueueStore().restore();

    expect(fetched).toEqual([{ auth: { cookieFile: '/cookies.txt' } }]);
  });

  it('does not bring anything back when keeping the queue is turned off', async () => {
    globalThis.__E2E_LIBRARY__ = { history: [], queue: [{ url: 'https://example.com/x', overrides: null, options: null }] };
    useSettingsStore().settings.input.restoreQueue = false;

    expect(await useQueueStore().restore()).toBe(0);
    expect(useMediaGroupStore().orderedGroups).toHaveLength(0);
  });
});

describe('history store', () => {
  it('records a finished video with where it was saved', async () => {
    const group = makeGroup('v', 'https://example.com/v', { title: 'A video', uploader: 'Someone', isLive: true });
    useMediaDestinationStore().addDestination('v', { path: '/downloads/a.mp4', confidence: 1 });

    await useHistoryStore().record(group);

    const [entry] = useHistoryStore().entries;
    expect(entry).toMatchObject({
      id: 'v',
      url: 'https://example.com/v',
      title: 'A video',
      uploader: 'Someone',
      path: '/downloads/a.mp4',
      items: 1,
      isLive: true,
    });
  });

  it('records a playlist once, with its title and number of files', async () => {
    const group = makeGroup('p', 'https://example.com/list', {
      isCombined: true,
      title: 'First item',
      playlistTitle: 'My list',
      items: {
        leader: { id: 'leader', url: 'u', isLeader: true, audioCodecs: [], formats: [], filesize: 0 },
        i1: { id: 'i1', url: 'u1', audioCodecs: [], formats: [], filesize: 0 },
        i2: { id: 'i2', url: 'u2', audioCodecs: [], formats: [], filesize: 0 },
      },
    });

    await useHistoryStore().record(group);
    await useHistoryStore().record(group);

    expect(useHistoryStore().entries).toHaveLength(1);
    expect(useHistoryStore().entries[0]).toMatchObject({ title: 'My list', items: 2 });
  });
});

describe('queue order', () => {
  it('is the same after saving and restoring', async () => {
    for (const name of ['first', 'second', 'third']) {
      addGroup(makeGroup(name, `https://example.com/${name}`), MediaState.configure);
    }
    const shownBefore = useMediaGroupStore().orderedGroups.map(group => group.url);
    await useQueueStore().save();

    for (const group of [...useMediaGroupStore().orderedGroups]) {
      useMediaGroupStore().deleteGroup(group.id);
    }
    await useQueueStore().restore();

    expect(useMediaGroupStore().orderedGroups.map(group => group.url)).toEqual(shownBefore);
  });
});
