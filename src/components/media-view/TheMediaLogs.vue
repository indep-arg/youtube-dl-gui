<template>
  <section class="py-4 px-8">
    <h2 class="text-lg font-semibold mb-2">
      {{ t('media.view.logs.errors.count', { count: errors.length }) }}
    </h2>
    <ul v-if="errors.length > 0" class="flex flex-col gap-4">
      <li v-for="(error, index) in errors" :key="error.id + error.code + error.timestamp">
        <diagnostic-card :open="index === 0" :diagnostic="error"/>
      </li>
    </ul>
    <div v-else class="alert alert-success alert-soft">
      <check-circle-icon class="h-5 w-5"/>
      <span class="text-base-content">{{ t('media.view.logs.errors.empty') }}</span>
    </div>
  </section>
  <section v-if="skippedGroups.length > 0" class="py-4 px-8">
    <h2 class="text-lg font-semibold mb-2">
      {{ t('media.view.logs.skipped.count', { count: skippedCount }) }}
    </h2>
    <ul class="flex flex-col gap-4">
      <li v-for="group in skippedGroups" :key="group.message">
        <div class="alert alert-warning alert-soft flex items-center justify-between gap-4">
          <span>{{ group.message }}</span>
          <span class="badge badge-warning badge-soft">
            {{ t('media.view.logs.skipped.items', { count: group.count }) }}
          </span>
        </div>
      </li>
    </ul>
  </section>
  <section v-if="warnings.length > 0" class="py-4 px-8">
    <h2 class="text-lg font-semibold mb-2">
      {{ t('media.view.logs.warnings.count', { count: warnings.length }) }}
    </h2>
    <ul class="flex flex-col gap-4">
      <li v-for="warning in warnings" :key="warning.id + warning.code + warning.timestamp">
        <diagnostic-card :diagnostic="warning"/>
      </li>
    </ul>
  </section>
  <section class="py-4 px-8">
    <h2 class="text-lg font-semibold mb-2">
      {{ t('media.view.logs.all.title') }}
    </h2>
    <div class="text-sm flex flex-col gap-2">
      <div
          v-if="hasLog"
          role="log"
          aria-live="off"
          class="max-h-64 overflow-y-auto bg-base-300 border border-base-200 rounded-box py-3 px-4 font-mono whitespace-pre-wrap text-xs leading-6"
      >
        <!-- One element per line: a new or repeated line only updates its own element. -->
        <div
            v-for="(line, index) in groupLog.lines.value"
            :key="groupLog.start.value + index"
            v-html="linkifyLine(line)"
        />
      </div>
      <div v-else class="alert alert-info alert-soft">
        <information-circle-icon class="h-5 w-5"/>
        <span class="text-base-content">{{ t('media.view.logs.all.empty') }}</span>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { useMediaDiagnosticsStore } from '../../stores/media/diagnostics.ts';
import { computed, toRefs } from 'vue';
import { CheckCircleIcon, InformationCircleIcon } from '@heroicons/vue/24/solid';
import DiagnosticCard from './DiagnosticCard.vue';
import { useI18n } from 'vue-i18n';
import { useGroupLog } from '../../composables/useGroupLog';
import { useLinkify } from '../../composables/useLinkify';
import { groupSkippedDiagnostics, isSkippedDiagnostic } from '../../helpers/skippedDiagnostics.ts';

const { t } = useI18n();
const props = defineProps({
  groupId: {
    type: String,
    required: true,
  },
});
const { groupId } = toRefs(props);

const diagnosticsStore = useMediaDiagnosticsStore();
const groupLog = useGroupLog(groupId);
const { linkify } = useLinkify();

// Lines rarely change once written, so each one is linkified once.
const linkified = new Map<string, string>();
const linkifyLine = (line: string): string => {
  let html = linkified.get(line);
  if (html === undefined) {
    html = linkify(line);
    if (linkified.size > 10_000) linkified.clear();
    linkified.set(line, html);
  }
  return html;
};

const hasLog = computed(() => groupLog.lines.value.length > 0);

const errors = computed((): MediaDiagnostic[] => {
  const diagnostics = diagnosticsStore.findDiagnosticsByGroupId(groupId.value);
  const errorDiagnostics: MediaDiagnostic[] = diagnostics.filter(diagnostic => diagnostic.level === 'error');
  const fatalDiagnostics: MediaDiagnostic[] = diagnosticsStore.findFatalsByGroupId(groupId.value).map(fatal => ({
    ...fatal,
    level: 'error',
    code: 'unknown',
    raw: fatal.details != null ? `\n${fatal.details}` : '',
    component: null,
  }));
  // The final error of a run is often the same line a diagnostic was made from: show it once.
  const covered = (fatal: MediaDiagnostic) => errorDiagnostics.some((diagnostic) => {
    const raw = diagnostic.raw.trim();
    return raw !== '' && (raw.includes(fatal.message.trim()) || fatal.message.includes(raw));
  });
  return [...fatalDiagnostics.filter(fatal => !covered(fatal)), ...errorDiagnostics];
});

const warnings = computed(() => {
  const diagnostics = diagnosticsStore.findDiagnosticsByGroupId(groupId.value);
  return diagnostics.filter(diagnostic => diagnostic.level === 'warning' && !isSkippedDiagnostic(diagnostic));
});

const skippedGroups = computed(() => {
  const diagnostics = diagnosticsStore.findDiagnosticsByGroupId(groupId.value);
  return groupSkippedDiagnostics(diagnostics);
});

const skippedCount = computed(() => skippedGroups.value.reduce((sum, group) => sum + group.count, 0));

</script>
