import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import ToolCard from '../../src/components/tool-card/ToolCard.vue';
import { i18n } from '../../src/i18n';
import type { BinaryProgress } from '../../src/tauri/types/binaries';

function mountCard(tool: BinaryProgress) {
  return mount(ToolCard, { props: { name: 'ffprobe', tool }, global: { plugins: [i18n] } });
}

describe('ToolCard', () => {
  it('shows the amount downloaded while a tool downloads', () => {
    const text = mountCard({ received: 5 * 1024 * 1024, total: 10 * 1024 * 1024, percent: 50 }).text();

    expect(text).toContain('5');
    expect(text).toContain('10');
  });

  it('says so when the file was already downloaded for another tool', () => {
    const text = mountCard({ received: 100, total: 100, percent: 100, reused: true }).text();

    expect(text).toContain('Already downloaded together with another tool.');
    expect(text).not.toContain('/');
  });

  it('shows neither an amount nor an error before the download starts', () => {
    const wrapper = mountCard({ received: 0, total: 0, percent: 0 });

    expect(wrapper.text()).not.toContain('bytes');
    expect(wrapper.find('details').exists()).toBe(false);
  });

  it('shows the error only when the tool failed', () => {
    const wrapper = mountCard({ received: 0, total: 0, percent: 0, error: '[download] timeout' });

    expect(wrapper.find('details').exists()).toBe(true);
    expect(wrapper.text()).toContain('timeout');
  });
});
