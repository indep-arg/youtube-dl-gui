<template>
  <form @submit.prevent="saveSettings">
    <base-sub-nav>
      <template #default>
        <base-button-dropdown
            placement="bottom"
            align="end"
            btnClass="btn-primary"
            :mainDisabled="!hasChanges || isResetting"
            :mainLoading="isSaving"
            :caretAriaLabel="t('layout.footer.queue.more')"
            caretClass="px-2"
            mainType="submit"
        >
          <template #main>
            {{ t('common.save') }}
          </template>

          <li>
            <button type="button" class="gap-2 text-nowrap" @click="resetSettings">
              {{ t('settings.reset.label') }}
            </button>
          </li>
          <li>
            <button type="button" class="gap-2 text-nowrap" @click="exportSettings">
              {{ t('settings.export.label') }}
            </button>
          </li>
          <li>
            <button type="button" class="gap-2 text-nowrap" @click="importSettings">
              {{ t('settings.import.label') }}
            </button>
          </li>
        </base-button-dropdown>
      </template>
      <template v-slot:title>
        <div role="tablist" class="tabs tabs-box flex gap-1">
          <router-link exactActiveClass="tab-active" role="tab" :to="{ name: 'settings.downloads' }" class="tab">
            {{ t('settings.tabs.downloads') }}
          </router-link>
          <router-link exactActiveClass="tab-active" role="tab" :to="{ name: 'settings.app' }" class="tab">
            {{ t('settings.tabs.app') }}
          </router-link>
          <router-link exactActiveClass="tab-active" role="tab" :to="{ name: 'settings.network' }" class="tab">
            {{ t('settings.tabs.network') }}
          </router-link>
          <router-link exactActiveClass="tab-active" role="tab" :to="{ name: 'settings.system' }" class="tab">
            {{ t('settings.tabs.system') }}
          </router-link>
          <router-link exactActiveClass="tab-active" role="tab" :to="{ name: 'settings.about' }" class="tab">
            {{ t('settings.tabs.about') }}
          </router-link>
        </div>
      </template>
    </base-sub-nav>
    <section class="flex flex-col py-4 px-8">
      <router-view v-slot="{ Component }">
        <component :is="Component" v-model="draft" />
      </router-view>
    </section>
  </form>
</template>

<script setup lang="ts">
import { computed, ref, toRaw } from 'vue';
import { useI18n } from 'vue-i18n';
import BaseSubNav from '../../components/base/BaseSubNav.vue';
import { useSettingsStore } from '../../stores/settings';
import { useToastStore } from '../../stores/toast';
import { useTheme } from '../../composables/useTheme.ts';
import { Settings } from '../../tauri/types/config.ts';
import BaseButtonDropdown from '../../components/base/BaseButtonDropdown.vue';

const settingsStore = useSettingsStore();
const toastStore = useToastStore();
const { setTheme } = useTheme();
const { t } = useI18n();

const draft = ref<Settings>(structuredClone<Settings>(toRaw(settingsStore.settings)));

const isSaving = ref(false);
const isResetting = ref(false);
const hasChanges = computed(() => JSON.stringify(draft.value) !== JSON.stringify(settingsStore.settings));

const applyThemeOf = (settings: Settings) => {
  if (settings.appearance.theme) {
    setTheme(settings.appearance.theme);
  }
};

// Shows settings that come from the backend in the form.
const showInForm = (settings: Settings) => {
  draft.value = structuredClone<Settings>(toRaw(settings));
  applyThemeOf(settings);
};

const saveSettings = async () => {
  isSaving.value = true;
  try {
    await settingsStore.patch(draft.value);
    applyThemeOf(draft.value);
    toastStore.showToast(t('settings.toasts.saved'), { style: 'success' });
  } catch (e) {
    toastStore.showToast(t('settings.toasts.saveFailed', { error: String(e) }), { style: 'error' });
  } finally {
    isSaving.value = false;
  }
};

// Exports what is saved, not unsaved changes in the form.
const exportSettings = async () => {
  try {
    if (await settingsStore.exportSettings()) {
      toastStore.showToast(t('settings.toasts.exported'), { style: 'success' });
    }
  } catch (e) {
    toastStore.showToast(t('settings.toasts.exportFailed', { error: String(e) }), { style: 'error' });
  }
};

const importSettings = async () => {
  try {
    const imported = await settingsStore.importSettings();
    if (!imported) return;
    showInForm(imported);
    toastStore.showToast(t('settings.toasts.imported'), { style: 'success' });
  } catch (e) {
    toastStore.showToast(t('settings.toasts.importFailed', { error: String(e) }), { style: 'error' });
  }
};

const resetSettings = async () => {
  isResetting.value = true;
  try {
    showInForm(await settingsStore.reset());
    toastStore.showToast(t('settings.toasts.reset'), { style: 'success' });
  } finally {
    isResetting.value = false;
  }
};
</script>
