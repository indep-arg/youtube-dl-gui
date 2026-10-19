import { Ref, onBeforeUnmount, onMounted, ref, shallowRef, triggerRef, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';

interface LogAppendEvent {
  groupId: string;
  /** Position of the line in the log. A repeated line comes again with the same index. */
  index: number;
  line: string;
}

interface LogSnapshot {
  start: number;
  lines: string[];
}

/** How many lines the viewer keeps: older ones are dropped. */
const MAX_LINES = 5000;

export function useGroupLog(groupId: Ref<string>) {
  const lines = shallowRef<string[]>([]);
  /** Index of `lines[0]` in the log of the group. */
  const start = ref(0);
  const isLoading = ref(false);
  const error = ref<string | null>(null);

  let unlisten: UnlistenFn | null = null;
  /** Bumped on every subscription change, so a late `listen` knows it is stale. */
  let generation = 0;

  function applyLine(index: number, line: string) {
    const position = index - start.value;
    if (position < 0) return;
    if (position < lines.value.length) {
      lines.value[position] = line;
    } else {
      lines.value.push(line);
      const extra = lines.value.length - MAX_LINES;
      if (extra > 0) {
        lines.value.splice(0, extra);
        start.value += extra;
      }
    }
  }

  async function subscribe(id: string) {
    const current = ++generation;
    isLoading.value = true;
    error.value = null;

    // Listen first and keep the lines that arrive before the snapshot: they
    // carry their index, so applying them again after it changes nothing.
    let pending: LogAppendEvent[] | null = [];
    try {
      const stop = await listen<LogAppendEvent>('logging_append', (event) => {
        if (event.payload.groupId !== id) return;
        if (pending) {
          pending.push(event.payload);
          return;
        }
        applyLine(event.payload.index, event.payload.line);
        triggerRef(lines);
      });
      if (current !== generation) {
        stop();
        return;
      }
      unlisten = stop;

      const snapshot = await invoke<LogSnapshot>('logging_subscribe', { groupId: id });
      if (current !== generation) return;
      start.value = snapshot.start;
      lines.value = [...snapshot.lines];
      for (const event of pending) {
        applyLine(event.index, event.line);
      }
      pending = null;
      triggerRef(lines);
    } catch (e: any) {
      console.error('Failed to subscribe to logs', e);
      error.value = e?.message ?? String(e);
    } finally {
      if (current === generation) {
        isLoading.value = false;
      }
    }
  }

  async function unsubscribe(id: string) {
    generation++;
    if (unlisten) {
      unlisten();
      unlisten = null;
    }
    try {
      await invoke('logging_unsubscribe', { groupId: id });
    } catch {
      // best-effort, ignore
    }
  }

  onMounted(() => {
    const id = groupId.value;
    if (id) {
      void subscribe(id);
    }
  });

  onBeforeUnmount(() => {
    const id = groupId.value;
    if (id) {
      void unsubscribe(id);
    } else if (unlisten) {
      unlisten();
      unlisten = null;
    }
  });

  watch(
    groupId,
    async (newId, oldId) => {
      if (newId === oldId) return;

      if (oldId) {
        await unsubscribe(oldId);
        lines.value = [];
        start.value = 0;
      }
      if (newId) {
        await subscribe(newId);
      }
    },
  );

  return {
    lines,
    start,
    isLoading,
    error,
  };
}
