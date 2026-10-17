<template>
  <component
      :is="hasDetails ? 'details' : 'section'"
      class="collapse bg-base-100 border-base-300 border"
      :class="{ 'collapse-arrow': hasDetails, 'cursor-auto': !hasDetails }"
  >
    <component
        :is="hasDetails ? 'summary' : 'div'"
        class="collapse-title text-sm flex flex-row items-center gap-2"
        :class="{ 'cursor-auto': !hasDetails }"
    >
      <span
          v-if="diagnostic.component"
          class="badge badge-soft"
          :class="diagnosticBadgeClass"
      >
        {{ diagnostic.component }}
      </span>
      <span
          v-if="relatedFatal && diagnostic.level ===  'error'"
          class="badge badge-soft badge-error"
      >
        <exclamation-circle-icon class="w-5 h-5"/>
        {{ t('media.view.logs.failure') }}
      </span>
      <span v-html="diagnosticDisplay.message"/>
    </component>
    <div v-if="hasDetails" class="collapse-content text-sm flex flex-col gap-2">
      <p
          class="max-h-44 text-wrap overflow-y-scroll bg-base-200 p-4 rounded-box"
          v-if="diagnostic.raw"
          v-html="diagnosticDisplay.raw"
      />
    </div>
  </component>
</template>

<script setup lang="ts">
import { computed, PropType, ref } from 'vue';
import { ExclamationCircleIcon } from '@heroicons/vue/24/solid';
import { useI18n } from 'vue-i18n';
import { useDiagnostic } from '../../composables/useDiagnostic.ts';

const { t } = useI18n();

const { diagnostic } = defineProps({
  diagnostic: {
    type: Object as PropType<MediaDiagnostic>,
    required: true,
  },
});

const diagnosticBadgeClass = computed(() => {
  return {
    'badge-warning': diagnostic.level === 'warning',
    'badge-error': diagnostic.level === 'error',
  };
});

const hasDetails = computed(() => diagnostic.raw != '');

const {
  diagnosticDisplay,
  relatedFatal,
} = useDiagnostic(ref(diagnostic));

</script>
