import { computed, ref, watch } from 'vue';
import { getCurrentWindow, ProgressBarStatus, UserAttentionType } from '@tauri-apps/api/window';
import { useMediaProgressStore } from '../stores/media/progress';
import { MediaState, useMediaStateStore } from '../stores/media/state';
import { notify } from './notifications';
import { NotificationKind } from './types/app';

export function startWindowWatcher(): void {
  const windowHandle = getCurrentWindow();
  const progressStore = useMediaProgressStore();
  const stateStore = useMediaStateStore();

  const previousActiveDownloadCount = ref(0);
  const attentionCooldownUntilMs = ref(0);

  function clampPercent0to100(value: number): number {
    return Math.max(0, Math.min(100, value));
  }

  function computeAggregatePercent(done: number, total: number): number | undefined {
    if (total <= 0) return undefined;
    return Math.round(clampPercent0to100((done / total) * 100));
  }

  function findActiveItemPercentIfSingleDownloading(): number | undefined {
    const entries = Object.entries(progressStore.progress);

    for (const [mediaId, itemProgress] of entries) {
      const mediaState = stateStore.getState(mediaId);

      if (mediaState === MediaState.downloading || mediaState === MediaState.downloadingList) {
        const percent = itemProgress.percentage;

        if (percent !== undefined && Number.isFinite(percent)) {
          return Math.round(clampPercent0to100(percent));
        }

        return undefined;
      }
    }

    return undefined;
  }

  // The last values sent to the window: each one is a call to the system.
  let lastBadge: number | undefined | null = null;
  let lastProgressBar: string | null = null;

  async function setDockOrTaskbarBadge(activeDownloadCount: number): Promise<void> {
    const badge = activeDownloadCount > 0 ? activeDownloadCount : undefined;
    if (badge === lastBadge) return;
    lastBadge = badge;
    try {
      await windowHandle.setBadgeCount(badge);
    } catch {
      // ignore (some platforms may not support this)
    }
  }

  async function applyProgressBar(state: { status: ProgressBarStatus; progress?: number }): Promise<void> {
    const key = `${state.status}:${state.progress ?? ''}`;
    if (key === lastProgressBar) return;
    lastProgressBar = key;
    try {
      await windowHandle.setProgressBar(state);
    } catch {
      // ignore (some platforms may not support this)
    }
  }

  async function setProgressBarState(params: {
    activeDownloadCount: number;
    doneCount: number;
    totalCount: number;
    singleItemPercent?: number;
  }): Promise<void> {
    const { activeDownloadCount, doneCount, totalCount, singleItemPercent } = params;

    if (activeDownloadCount <= 0) {
      await applyProgressBar({ status: ProgressBarStatus.None });
      return;
    }

    if (activeDownloadCount === 1 && typeof singleItemPercent === 'number') {
      await applyProgressBar({ status: ProgressBarStatus.Normal, progress: singleItemPercent });
      return;
    }

    const aggregatePercent = computeAggregatePercent(doneCount, totalCount);
    if (typeof aggregatePercent === 'number') {
      await applyProgressBar({ status: ProgressBarStatus.Normal, progress: aggregatePercent });
      return;
    }

    await applyProgressBar({ status: ProgressBarStatus.Indeterminate });
  }

  async function maybeRequestUserAttention(params: {
    activeDownloadCount: number;
    readyCount: number;
  }): Promise<void> {
    const { activeDownloadCount, readyCount } = params;

    const wasPreviouslyDownloading = previousActiveDownloadCount.value > 0;
    const queueIsFinished = activeDownloadCount === 0 && readyCount === 0;

    if (!wasPreviouslyDownloading || !queueIsFinished) return;

    const nowMs = Date.now();
    if (nowMs < attentionCooldownUntilMs.value) return;

    attentionCooldownUntilMs.value = nowMs + 2000;

    if (previousActiveDownloadCount.value > 1) {
      await notify(NotificationKind.QueueFinished, { n: previousActiveDownloadCount.value.toString() });
    }
    try {
      await windowHandle.requestUserAttention(UserAttentionType.Informational);
    } catch {
      // ignore
    }
  }

  /** What the window shows: badge, taskbar progress and the end-of-queue alert. */
  const windowState = computed(() => {
    const aggregate = progressStore.findAllProgress();
    const activeDownloadCount = aggregate.downloading ?? 0;
    return {
      activeDownloadCount,
      doneCount: aggregate.done ?? 0,
      totalCount: aggregate.total ?? 0,
      readyCount: aggregate.ready ?? 0,
      singleItemPercent: activeDownloadCount === 1 ? findActiveItemPercentIfSingleDownloading() : undefined,
    };
  });

  async function syncWindowState(state: typeof windowState.value): Promise<void> {
    const { activeDownloadCount, doneCount, totalCount, readyCount, singleItemPercent } = state;

    await setDockOrTaskbarBadge(activeDownloadCount + readyCount);

    await setProgressBarState({
      activeDownloadCount,
      doneCount,
      totalCount,
      singleItemPercent,
    });

    await maybeRequestUserAttention({
      activeDownloadCount,
      readyCount,
    });

    previousActiveDownloadCount.value = activeDownloadCount;
  }

  // Progress changes many times a second, but what the window shows rarely does.
  watch(
    windowState,
    (state, previous) => {
      if (previous && JSON.stringify(state) === JSON.stringify(previous)) return;
      void syncWindowState(state);
    },
    { immediate: true },
  );
}
