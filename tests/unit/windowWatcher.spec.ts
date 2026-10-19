import { describe, expect, it } from 'vitest';
import { nextTick } from 'vue';
import { mockWindows } from '@tauri-apps/api/mocks';
import { installTauriMock } from '../utils/tauriMock';
import { startWindowWatcher } from '../../src/tauri/window';
import { useMediaGroupStore } from '../../src/stores/media/group';
import { MediaState, useMediaStateStore } from '../../src/stores/media/state';
import { useMediaProgressStore } from '../../src/stores/media/progress';
import { ProgressCategory } from '../../src/tauri/types/progress';

const flush = async () => {
  await nextTick();
  await new Promise(resolve => setTimeout(resolve, 0));
};

describe('window watcher', () => {
  it('only calls the window when what it shows changes', async () => {
    const calls: string[] = [];
    installTauriMock({
      'plugin:window|set_badge_count': (cmd) => {
        calls.push(cmd);
        return null;
      },
      'plugin:window|set_progress_bar': (cmd, args) => {
        calls.push(`${cmd} ${JSON.stringify(args)}`);
        return null;
      },
    });
    mockWindows('main');

    useMediaGroupStore().createGroup({
      id: 'g',
      url: 'g',
      total: 1,
      processed: 1,
      errored: 0,
      isCombined: false,
      audioCodecs: [],
      formats: [],
      filesize: 0,
      items: { a: { id: 'a', url: 'a', audioCodecs: [], formats: [], filesize: 0 } },
    });
    useMediaStateStore().setState('a', MediaState.downloading);
    startWindowWatcher();
    await flush();
    const afterStart = calls.length;

    const progress = useMediaProgressStore();
    const update = (percentage: number, speedBps: number) => progress.processMediaProgressPayload({
      id: 'a',
      groupId: 'g',
      category: ProgressCategory.video,
      percentage,
      speedBps,
    });

    // Only the speed changes: nothing to update in the taskbar.
    for (const speed of [100, 200, 300]) {
      update(10, speed);
      await flush();
    }
    const afterSpeed = calls.length;
    update(11, 300);
    await flush();

    expect(afterStart).toBeGreaterThan(0);
    expect(calls.slice(afterStart, afterSpeed).filter(c => c.includes('"progress":10'))).toHaveLength(1);
    expect(calls.length - afterSpeed).toBe(1);
    expect(calls.at(-1)).toContain('"progress":11');
  });

  it('ignores a download that finishes after its group was removed', () => {
    const progress = useMediaProgressStore();
    expect(() => progress.processMediaCompletePayload({ id: 'gone', groupId: 'gone' })).not.toThrow();
  });
});
