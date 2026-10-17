<template>
  <base-fieldset
      :legend="t('settings.sitePresets.legend')"
      :label="t('settings.sitePresets.legendLabel')"
  >
    <p v-if="presets.length === 0" class="label mb-2">{{ t('settings.sitePresets.empty') }}</p>
    <div
        v-for="(preset, index) in presets"
        :key="index"
        class="bg-base-100 rounded-box p-4 mb-3 flex flex-col gap-3"
        data-testid="site-preset"
    >
      <div class="flex gap-3 items-end flex-wrap">
        <div class="flex flex-col gap-1">
          <label class="font-semibold" :for="`site-preset-domain-${index}`">
            {{ t('settings.sitePresets.domain.label') }}
          </label>
          <input
              :id="`site-preset-domain-${index}`"
              v-model="preset.domain"
              type="text"
              class="input w-56"
              :placeholder="t('settings.sitePresets.domain.placeholder')"
              @blur="preset.domain = normalizeDomain(preset.domain)"
          />
        </div>
        <div class="flex flex-col gap-1">
          <label class="font-semibold" :for="`site-preset-type-${index}`">
            {{ t('settings.sitePresets.trackType.label') }}
          </label>
          <select :id="`site-preset-type-${index}`" v-model="preset.trackType" class="select w-48">
            <option :value="null">{{ t('settings.sitePresets.trackType.any') }}</option>
            <option :value="TrackType.both">{{ t('settings.sitePresets.trackType.both') }}</option>
            <option :value="TrackType.video">{{ t('settings.sitePresets.trackType.video') }}</option>
            <option :value="TrackType.audio">{{ t('settings.sitePresets.trackType.audio') }}</option>
          </select>
        </div>
        <button
            type="button"
            class="btn btn-ghost btn-square ml-auto"
            :title="t('settings.sitePresets.remove')"
            @click="removePreset(index)"
        >
          <span class="sr-only">{{ t('settings.sitePresets.remove') }}</span>
          <trash-icon class="w-5 h-5"/>
        </button>
      </div>
      <base-file-input
          v-model="preset.downloadDir"
          :label="t('settings.sitePresets.folder.label')"
          directory
      />
      <p class="label">{{ t('settings.sitePresets.folder.hint') }}</p>
    </div>
    <button type="button" class="btn btn-soft w-fit" @click="addPreset">
      {{ t('settings.sitePresets.add') }}
    </button>
  </base-fieldset>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { TrashIcon } from '@heroicons/vue/24/outline';
import BaseFieldset from '../base/BaseFieldset.vue';
import BaseFileInput from '../base/BaseFileInput.vue';
import { Settings } from '../../tauri/types/config.ts';
import { TrackType } from '../../tauri/types/media.ts';
import { normalizeDomain } from '../../helpers/sitePresets.ts';

const { t } = useI18n();
const settings = defineModel<Settings>({ required: true });

const presets = computed(() => settings.value.input.sitePresets ?? []);

function addPreset() {
  settings.value.input.sitePresets = [...presets.value, { domain: '', trackType: null, downloadDir: null }];
}

function removePreset(index: number) {
  settings.value.input.sitePresets = presets.value.filter((_, i) => i !== index);
}
</script>
