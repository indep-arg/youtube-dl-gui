<template>
  <router-view/>
  <the-toaster/>
</template>

<script setup lang="ts">
import TheToaster from './components/TheToaster.vue';
import { useBinariesStore } from './stores/binaries';
import { useRouter } from 'vue-router';
import { useUpdaterStore } from './stores/updater';
import { useStrongholdStore } from './stores/stronghold';
import { useDragDrop } from './composables/useDragDrop.ts';
import { useQueueStore } from './stores/queue.ts';
import { useToastStore } from './stores/toast.ts';
import { useI18n } from 'vue-i18n';

const router = useRouter();
const binariesStore = useBinariesStore();
const updaterStore = useUpdaterStore();
const strongholdStore = useStrongholdStore();
const queueStore = useQueueStore();
const toastStore = useToastStore();
const { t } = useI18n();

useDragDrop();

const checkTools = async () => {
  const toolsToEnsure = await binariesStore.check();
  if (toolsToEnsure.length > 0) {
    await router.push('/install');
    return;
  }
  await restoreQueue();
};

// Only once the tools are installed: the links are fetched again. Saving starts
// afterwards, so the saved queue is not overwritten by the empty one at startup.
const restoreQueue = async () => {
  try {
    const restored = await queueStore.restore();
    if (restored > 0) {
      toastStore.showToast(t('media.restored', restored), { style: 'info' });
    }
  } catch (e) {
    console.error('Unable to restore the queue:', e);
  } finally {
    queueStore.startSaving();
  }
};

const checkUpdates = async () => {
  try {
    await updaterStore.check();
  } catch (e) {
    console.warn('Unable to check for updates:', e);
  }
};

try {
  void checkTools();
} catch (e) {
  console.error(e);
}

try {
  void strongholdStore.loadStatus();
} catch (e) {
  console.error(e);
}

void checkUpdates();
</script>
