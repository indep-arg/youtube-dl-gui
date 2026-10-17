import { mount } from '@vue/test-utils';
import { nextTick, ref } from 'vue';
import { describe, expect, it } from 'vitest';
import SettingsPerformance from '../../src/components/settings/SettingsPerformance.vue';
import { i18n } from '../../src/i18n';
import { defaultSettings, type Settings } from '../../src/tauri/types/config';

function mountWith(rateLimitKib: number | null) {
  const settings = ref<Settings>(structuredClone(defaultSettings));
  settings.value.performance.rateLimitKib = rateLimitKib;
  const wrapper = mount(SettingsPerformance, {
    props: {
      'modelValue': settings.value,
      'onUpdate:modelValue': (value: Settings) => {
        settings.value = value;
      },
    },
    global: { plugins: [i18n] },
  });
  return { wrapper, settings };
}

describe('SettingsPerformance speed limit', () => {
  it('stores what is typed in KB/s as KiB/s', async () => {
    const { wrapper, settings } = mountWith(null);

    await wrapper.find('#rateLimit').setValue('500');

    expect(settings.value.performance.rateLimitKib).toBe(500);
  });

  it('converts MB/s to KiB/s', async () => {
    const { wrapper, settings } = mountWith(null);

    await wrapper.find('select[aria-label]').setValue('mb');
    await wrapper.find('#rateLimit').setValue('2');

    expect(settings.value.performance.rateLimitKib).toBe(2048);
  });

  it('keeps the number on screen when the unit changes', async () => {
    const { wrapper, settings } = mountWith(300);

    await wrapper.find('select[aria-label]').setValue('mb');
    await nextTick();

    expect(settings.value.performance.rateLimitKib).toBe(300 * 1024);
    expect((wrapper.find('#rateLimit').element as HTMLInputElement).value).toBe('300');
  });

  it('shows a whole number of MB/s in MB/s', () => {
    const { wrapper } = mountWith(4096);

    expect((wrapper.find('select[aria-label]').element as HTMLSelectElement).value).toBe('mb');
    expect((wrapper.find('#rateLimit').element as HTMLInputElement).value).toBe('4');
  });

  it('an empty field means no limit', async () => {
    const { wrapper, settings } = mountWith(1000);

    await wrapper.find('#rateLimit').setValue('');

    expect(settings.value.performance.rateLimitKib).toBeNull();
  });
});
