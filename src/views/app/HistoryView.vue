<template>
  <div>
    <base-sub-nav>
      <template v-slot:default>
        <base-button
            class="btn-soft"
            :disabled="historyStore.entries.length === 0"
            @click="clearHistory"
        >
          {{ isConfirmingClear ? t('history.confirmClear') : t('history.clear') }}
        </base-button>
      </template>
      <template v-slot:title>
        <h1 class="text-lg font-semibold self-center">{{ t('history.title') }}</h1>
      </template>
    </base-sub-nav>
    <section class="flex flex-col gap-3 py-4 px-8">
      <div v-if="isLoading" class="flex justify-center py-12">
        <span class="loading loading-spinner"></span>
      </div>
      <p v-else-if="historyStore.entries.length === 0" class="text-center opacity-70 py-12">
        {{ t('history.empty') }}
      </p>
      <article
          v-for="entry in historyStore.entries"
          v-else
          :key="entry.id"
          class="flex gap-4 items-center bg-base-100 rounded-box p-3"
          data-testid="history-entry"
      >
        <img
            :src="entry.thumbnail ?? placeholderUrl"
            alt=""
            class="w-28 aspect-video object-cover rounded-md shrink-0"
            @error="setPlaceholder"
        />
        <div class="flex flex-col min-w-0 grow gap-1">
          <p class="font-semibold truncate" :title="entry.title ?? entry.url">{{ entry.title ?? entry.url }}</p>
          <p class="text-sm opacity-70 truncate">{{ details(entry) }}</p>
        </div>
        <div class="flex gap-1 shrink-0">
          <button
              v-if="entry.path"
              type="button"
              class="btn btn-ghost btn-sm btn-square"
              :title="t('history.actions.show')"
              @click="show(entry)"
          >
            <span class="sr-only">{{ t('history.actions.show') }}</span>
            <folder-open-icon class="w-5 h-5"/>
          </button>
          <button
              type="button"
              class="btn btn-ghost btn-sm btn-square"
              :title="t('history.actions.addAgain')"
              @click="addAgain(entry)"
          >
            <span class="sr-only">{{ t('history.actions.addAgain') }}</span>
            <arrow-path-icon class="w-5 h-5"/>
          </button>
          <button
              type="button"
              class="btn btn-ghost btn-sm btn-square"
              :title="t('history.actions.remove')"
              @click="remove(entry)"
          >
            <span class="sr-only">{{ t('history.actions.remove') }}</span>
            <trash-icon class="w-5 h-5"/>
          </button>
        </div>
      </article>
    </section>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { useRouter } from 'vue-router';
import { ArrowPathIcon, FolderOpenIcon, TrashIcon } from '@heroicons/vue/24/outline';
import BaseSubNav from '../../components/base/BaseSubNav.vue';
import BaseButton from '../../components/base/BaseButton.vue';
import placeholderUrl from '../../assets/placeholder.png';
import { useHistoryStore } from '../../stores/history.ts';
import { useMediaStore } from '../../stores/media/media.ts';
import { useToastStore } from '../../stores/toast.ts';
import { useOpener } from '../../composables/useOpener.ts';
import type { HistoryEntry } from '../../tauri/types/library.ts';

const { t, locale } = useI18n();
const router = useRouter();
const historyStore = useHistoryStore();
const mediaStore = useMediaStore();
const toastStore = useToastStore();
const { revealPath } = useOpener();

const isLoading = ref(true);
const isConfirmingClear = ref(false);

onMounted(async () => {
  try {
    await historyStore.load();
  } catch (e) {
    toastStore.showToast(t('history.toasts.error', { error: String(e) }), { style: 'error' });
  } finally {
    isLoading.value = false;
  }
});

function details(entry: HistoryEntry): string {
  const parts = [new Date(entry.finishedAt).toLocaleString(locale.value)];
  if (entry.uploader) parts.push(entry.uploader);
  if (entry.items > 1) parts.push(t('history.items', entry.items));
  if (entry.isLive) parts.push(t('history.live'));
  return parts.join(' · ');
}

function setPlaceholder(event: Event) {
  (event.target as HTMLImageElement).src = placeholderUrl;
}

async function show(entry: HistoryEntry) {
  if (!entry.path) return;
  try {
    await revealPath(entry.path);
  } catch {
    toastStore.showToast(t('history.toasts.notFound'), { style: 'warning' });
  }
}

async function addAgain(entry: HistoryEntry) {
  await mediaStore.addUrlBatch([entry.url]);
  toastStore.showToast(t('history.toasts.added'), { style: 'success' });
  await router.push({ name: 'home' });
}

async function remove(entry: HistoryEntry) {
  try {
    await historyStore.remove(entry.id);
  } catch (e) {
    toastStore.showToast(t('history.toasts.error', { error: String(e) }), { style: 'error' });
  }
}

// Clearing asks for a second click, like resetting the settings.
async function clearHistory() {
  if (!isConfirmingClear.value) {
    isConfirmingClear.value = true;
    setTimeout(() => {
      isConfirmingClear.value = false;
    }, 3000);
    return;
  }
  isConfirmingClear.value = false;
  try {
    await historyStore.clear();
  } catch (e) {
    toastStore.showToast(t('history.toasts.error', { error: String(e) }), { style: 'error' });
  }
}
</script>
