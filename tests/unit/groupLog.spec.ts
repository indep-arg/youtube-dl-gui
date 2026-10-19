import { mount, flushPromises } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import { defineComponent, h, ref } from 'vue';
import { emit } from '@tauri-apps/api/event';
import { useGroupLog } from '../../src/composables/useGroupLog';
import { installTauriMock } from '../utils/tauriMock';

function mountLog(groupId = 'g') {
  const id = ref(groupId);
  let log!: ReturnType<typeof useGroupLog>;
  const wrapper = mount(defineComponent({
    setup() {
      log = useGroupLog(id);
      return () => h('div');
    },
  }));
  return { wrapper, id, log: () => log };
}

describe('useGroupLog', () => {
  it('keeps lines apart and replaces a repeated line', async () => {
    installTauriMock({
      logging_subscribe: () => ({ start: 0, lines: ['first'] }),
      logging_unsubscribe: () => null,
    });
    const { log } = mountLog();
    await flushPromises();

    await emit('logging_append', { groupId: 'g', index: 1, line: 'retry' });
    await emit('logging_append', { groupId: 'g', index: 1, line: 'retry (x2)' });
    await emit('logging_append', { groupId: 'g', index: 2, line: 'done' });
    await emit('logging_append', { groupId: 'other', index: 3, line: 'not mine' });

    expect(log().lines.value).toEqual(['first', 'retry (x2)', 'done']);
  });

  it('applies lines that arrive while the snapshot is loading', async () => {
    installTauriMock({
      logging_subscribe: async () => {
        // Written after the listener exists, but before the snapshot is read.
        await emit('logging_append', { groupId: 'g', index: 1, line: 'second' });
        return { start: 0, lines: ['first'] };
      },
      logging_unsubscribe: () => null,
    });
    const { log } = mountLog();
    await flushPromises();

    expect(log().lines.value).toEqual(['first', 'second']);
  });

  it('starts from the index of the snapshot', async () => {
    installTauriMock({
      logging_subscribe: () => ({ start: 40, lines: ['line 40', 'line 41'] }),
      logging_unsubscribe: () => null,
    });
    const { log } = mountLog();
    await flushPromises();

    await emit('logging_append', { groupId: 'g', index: 41, line: 'line 41 (x2)' });
    await emit('logging_append', { groupId: 'g', index: 12, line: 'too old' });

    expect(log().lines.value).toEqual(['line 40', 'line 41 (x2)']);
  });

  it('stops listening when it is unmounted before the subscription finishes', async () => {
    let unsubscribed = 0;
    installTauriMock({
      logging_subscribe: () => ({ start: 0, lines: [] }),
      logging_unsubscribe: () => {
        unsubscribed++;
        return null;
      },
    });
    const { wrapper, log } = mountLog();
    wrapper.unmount();
    await flushPromises();

    await emit('logging_append', { groupId: 'g', index: 0, line: 'after unmount' });
    expect(log().lines.value).toEqual([]);
    expect(unsubscribed).toBe(1);
  });
});
