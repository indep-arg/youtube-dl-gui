<template>
  <div class="relative block w-full">
    <progress
        :id="composedId"
        class="progress w-full h-7 [&::-webkit-progress-value]:transition-all [&::-moz-progress-bar]:transition-all"
        :class="progressClass"
        :max="max"
        :value="value"
    />
    <!-- One text that reads on both the empty and the filled part: a halo in the colour of the surface around it. -->
    <label
        :for="composedId"
        class="absolute text-base-content flex justify-center items-center text-sm align-middle text-center h-7 w-full top-0 pointer-events-none [text-shadow:0_0_3px_var(--color-base-100),0_0_6px_var(--color-base-100)]"
    >
      <slot/>
    </label>
  </div>
</template>

<script setup lang="ts">
import { computed, PropType } from 'vue';

type ProgressVariant = 'primary' | 'warning' | 'error';

const PROGRESS_VARIANT_CLASS: Record<ProgressVariant, string> = {
  primary: 'progress-primary',
  warning: 'progress-warning',
  error: 'progress-error',
};

const composedId = computed(() => `${id}-progress`);

const { id, max, value, variant } = defineProps({
  id: {
    type: String,
    required: true,
  },
  max: {
    type: Number,
  },
  value: {
    type: Number,
  },
  variant: {
    type: String as PropType<ProgressVariant>,
    default: 'primary',
  },
});

const progressClass = computed(() => PROGRESS_VARIANT_CLASS[variant]);

</script>
