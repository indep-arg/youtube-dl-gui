<template>
  <div class="flex flex-col">
    <base-fieldset :legend="t('about.title')" label="">
      <div class="flex flex-col gap-2">
        <div class="flex flex-wrap items-center gap-x-2 gap-y-1 mb-2 w-full">
          <template v-for="(link, index) in links" :key="link.label">
            <hr v-if="index > 0" class="divider divider-horizontal mx-0 border-none">
            <a :href="link.href" class="link" target="_blank" rel="noopener">{{ t(link.label) }}</a>
          </template>
        </div>
        <p>{{ t('about.credit') }}</p>
        <p>{{ t('about.fork', { maintainer: 'indep-arg' }) }}</p>
        <div class="flex gap-2 items-center">
          <p>{{ t('about.poweredBy') }}</p>
          <button type="button" @click="openLicenses()" class="btn btn-soft btn-xs">
            {{ t('about.viewLicenses') }}
          </button>
        </div>
        <p>{{ t('about.version', { version: appVersion }) }}</p>
      </div>
    </base-fieldset>
    <div class="divider my-0" />
    <base-fieldset :legend="t('about.diagnostics.title')" :label="t('about.diagnostics.hint')">
      <div class="flex flex-col items-start gap-3">
        <button
          id="copy-diagnostics"
          type="button"
          class="btn btn-soft"
          :disabled="isCollecting"
          @click="copyDiagnostics"
        >
          <span v-if="isCollecting" class="loading loading-spinner loading-xs" aria-hidden="true" />
          {{ t('about.diagnostics.copy') }}
        </button>
        <!-- Shown as well as copied, so people see what they are about to share. -->
        <pre
          v-if="diagnostics"
          id="diagnostics-text"
          class="w-full max-w-3xl overflow-x-auto rounded-box bg-base-300 p-4 text-xs leading-5 whitespace-pre-wrap"
        >{{ diagnostics }}</pre>
      </div>
    </base-fieldset>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import BaseFieldset from '../../../components/base/BaseFieldset.vue';
import { useOpener } from '../../../composables/useOpener.ts';
import { invoke } from '@tauri-apps/api/core';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { ref } from 'vue';
import { useToastStore } from '../../../stores/toast.ts';

defineOptions({
  inheritAttrs: false,
});

const { t } = useI18n();
const { openInternalPath } = useOpener();
const appVersion = typeof __APP_VERSION__ === 'undefined' ? '' : __APP_VERSION__;

const repository = 'https://github.com/indep-arg/youtube-dl-gui';
const links = [
  { label: 'about.links.github', href: repository },
  { label: 'about.links.guide', href: `${repository}/blob/main/docs/GUIDE.md` },
  { label: 'about.links.privacy', href: `${repository}/blob/main/docs/PRIVACY.md` },
  { label: 'about.links.changelog', href: `${repository}/blob/main/CHANGELOG.md` },
  { label: 'about.links.reportABug', href: `${repository}/issues/new?template=bug_report.yml` },
];

const toastStore = useToastStore();
const diagnostics = ref('');
const isCollecting = ref(false);

const copyDiagnostics = async () => {
  isCollecting.value = true;
  try {
    diagnostics.value = await invoke<string>('app_diagnostics');
    await writeText(diagnostics.value);
    toastStore.showToast(t('about.diagnostics.copied'), { style: 'success' });
  } catch (e) {
    console.error(e);
    toastStore.showToast(t('about.diagnostics.failed'), { style: 'error' });
  } finally {
    isCollecting.value = false;
  }
};

const openLicenses = async () => {
  await openInternalPath('licenses/3rdpartylicenses.txt');
};
</script>
