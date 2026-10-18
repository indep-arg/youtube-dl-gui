<template>
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
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import BaseFieldset from '../../../components/base/BaseFieldset.vue';
import { useOpener } from '../../../composables/useOpener.ts';

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

const openLicenses = async () => {
  await openInternalPath('licenses/3rdpartylicenses.txt');
};
</script>
