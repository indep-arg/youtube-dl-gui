<template>
  <base-fieldset
      :legend="t('settings.performance.legend')"
      :label="t('settings.performance.legendLabel')"
  >
    <label class="font-semibold" for="maxConcurrency">{{ t('settings.performance.maxConcurrency.label') }}</label>
    <div class="flex w-full h-full items-center gap-4">
      <input
          v-model.number="settings.performance.maxConcurrency"
          type="range"
          :min="1"
          :max="32"
          id="maxConcurrency"
          class="range range-primary block"
      />
      <p class="w-4">
        {{ t('common.parenthesis', { content: settings.performance.maxConcurrency }) }}
      </p>
    </div>
    <label class="font-semibold mt-4" for="splitPlaylistThreshold">
      {{ t('settings.performance.splitPlaylistThreshold.label') }}
    </label>
    <select
        id="splitPlaylistThreshold"
        v-model="settings.performance.splitPlaylistThreshold"
        class="select mb-2"
    >
      <option :value="1">{{ t('settings.performance.splitPlaylistThreshold.options.never') }}</option>
      <option v-for="interval in splitIntervals" :key="interval" :value="interval">
        {{ t('settings.performance.splitPlaylistThreshold.options.lessThan', { number: interval }) }}
      </option>
    </select>
    <label class="font-semibold mt-2" for="autoLoadSize">
      {{ t('settings.performance.autoLoadSize.label') }}
    </label>
    <input
        id="autoLoadSize"
        type="checkbox"
        v-model="settings.performance.autoLoadSize"
        class="toggle toggle-primary"
    />
    <label class="font-semibold mt-4" for="rateLimit">
      {{ t('settings.performance.rateLimit.label') }}
    </label>
    <div class="join">
      <input
          id="rateLimit"
          v-model.number="rateLimitValue"
          type="number"
          min="0"
          step="any"
          :placeholder="t('settings.performance.rateLimit.placeholder')"
          class="input join-item w-40"
      />
      <select
          v-model="rateLimitUnit"
          :aria-label="t('settings.performance.rateLimit.label')"
          class="select join-item w-28"
      >
        <option value="kb">{{ t('settings.performance.rateLimit.units.kb') }}</option>
        <option value="mb">{{ t('settings.performance.rateLimit.units.mb') }}</option>
      </select>
    </div>
    <p class="label">{{ t('settings.performance.rateLimit.hint') }}</p>
    <label class="font-semibold mt-4" for="retries">
      {{ t('settings.performance.retries.label') }}
    </label>
    <input
        id="retries"
        v-model.number="settings.performance.retries"
        type="number"
        min="0"
        max="100"
        class="input w-28"
    />
    <p class="label">{{ t('settings.performance.retries.hint') }}</p>
    <label class="font-semibold mt-4" for="concurrentFragments">
      {{ t('settings.performance.concurrentFragments.label') }}
    </label>
    <div class="flex w-full h-full items-center gap-4">
      <input
          v-model.number="settings.performance.concurrentFragments"
          type="range"
          :min="1"
          :max="16"
          id="concurrentFragments"
          class="range range-primary block"
      />
      <p class="w-4">
        {{ t('common.parenthesis', { content: settings.performance.concurrentFragments }) }}
      </p>
    </div>
    <p class="label">{{ t('settings.performance.concurrentFragments.hint') }}</p>
  </base-fieldset>
</template>

<script setup lang="ts">
import BaseFieldset from '../base/BaseFieldset.vue';
import { Settings } from '../../tauri/types/config.ts';
import { useI18n } from 'vue-i18n';
import { computed, ref, watch } from 'vue';

const { t } = useI18n();
const settings = defineModel<Settings>({ required: true });

const splitIntervals = [50, 100, 150, 200];

// The limit is stored in KiB/s; the field shows it in KB/s or MB/s.
const rateLimitUnit = ref<'kb' | 'mb'>(
  (settings.value.performance.rateLimitKib ?? 0) >= 1024 && (settings.value.performance.rateLimitKib ?? 0) % 1024 === 0
    ? 'mb'
    : 'kb',
);
const unitFactor = computed(() => (rateLimitUnit.value === 'mb' ? 1024 : 1));
const rateLimitValue = computed<number | ''>({
  get: () => {
    const kib = settings.value.performance.rateLimitKib;
    return kib ? kib / unitFactor.value : '';
  },
  set: (value) => {
    const kib = typeof value === 'number' && value > 0 ? Math.round(value * unitFactor.value) : null;
    settings.value.performance.rateLimitKib = kib;
  },
});

// Changing the unit keeps the number on screen and converts the stored limit.
watch(rateLimitUnit, (unit, previous) => {
  const kib = settings.value.performance.rateLimitKib;
  if (!kib) return;
  const shown = kib / (previous === 'mb' ? 1024 : 1);
  settings.value.performance.rateLimitKib = Math.round(shown * (unit === 'mb' ? 1024 : 1));
});
</script>
