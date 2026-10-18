import AxeBuilder from '@axe-core/playwright';
import { expect, type Page } from '@playwright/test';
import { test } from './utils/fixtures';

const PAGES = [
  '/settings', '/settings/app', '/settings/network', '/settings/system', '/settings/about',
  '/location', '/authentication', '/subtitles', '/input-filters', '/history',
];

async function addLink(page: Page, url: string) {
  await page.fill('header input[type="text"]', url);
  await page.getByRole('button', { name: 'Add', exact: true }).click();
}

async function setTheme(page: Page, theme: 'light' | 'dark') {
  await page.evaluate(t => document.body.setAttribute('data-theme', t), theme);
}

/** The problems axe finds: rule, impact, and where. */
async function problems(page: Page): Promise<string[]> {
  const results = await new AxeBuilder({ page })
    .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'])
    .analyze();
  return results.violations.flatMap(v => v.nodes.map(n => `${v.impact} ${v.id}: ${n.target.join(' ')}`));
}

for (const theme of ['light', 'dark'] as const) {
  test.describe(`accessibility (${theme})`, () => {
    test('the queue with cards in every state', async ({ page }) => {
      await page.goto('/');
      await setTheme(page, theme);
      await addLink(page, 'https://example.com/video');
      await addLink(page, 'https://example.com/live');
      await addLink(page, 'https://example.com/scheduled');
      await addLink(page, 'https://example.com/site-changed');
      await addLink(page, 'https://example.com/finish-video');
      await expect(page.locator('article')).toHaveCount(5);
      await page.getByRole('article').filter({ hasText: 'Test Video' }).first()
        .getByRole('button', { name: 'Download individually' }).click();
      await expect(page.getByText('Download complete')).toBeVisible();
      expect(await problems(page)).toEqual([]);
    });

    test('a download in progress, its information and its logs', async ({ page }) => {
      await page.goto('/');
      await setTheme(page, theme);
      await addLink(page, 'https://example.com/video');
      await page.getByRole('button', { name: 'Download individually' }).click();
      await expect(page.getByRole('button', { name: 'Pause download' })).toBeVisible();
      expect(await problems(page)).toEqual([]);

      await page.getByRole('link', { name: 'Show metadata' }).click();
      await expect(page).toHaveURL(/\/group\//);
      expect(await problems(page)).toEqual([]);
      await page.getByRole('tab', { name: 'Logs' }).click();
      await expect(page).toHaveURL(/\/logs$/);
      expect(await problems(page)).toEqual([]);
    });

    test('the preferences of a download', async ({ page }) => {
      await page.goto('/');
      await setTheme(page, theme);
      await addLink(page, 'https://example.com/video');
      await page.getByRole('link', { name: 'Download preferences' }).click();
      await expect(page).toHaveURL(/\/preferences\//);
      expect(await problems(page)).toEqual([]);
      for (const tab of ['Network', 'Output', 'Subtitles']) {
        await page.getByRole('tab', { name: tab, exact: true }).click();
        await page.waitForLoadState('networkidle');
        expect(await problems(page), tab).toEqual([]);
      }
    });

    for (const path of PAGES) {
      test(`page ${path}`, async ({ page }) => {
        await page.goto(path);
        await setTheme(page, theme);
        await page.waitForLoadState('networkidle');
        expect(await problems(page)).toEqual([]);
      });
    }
  });
}

test.describe('accessibility of the install screen', () => {
  test.use({ mockData: [{ binaries: { check: ['yt-dlp', 'ffmpeg'], ensure: true } }, { scope: 'test' }] });

  for (const theme of ['light', 'dark'] as const) {
    test(theme, async ({ page }) => {
      await page.goto('/');
      await setTheme(page, theme);
      await expect(page.getByText('We\'re getting everything ready.')).toBeVisible();
      expect(await problems(page)).toEqual([]);
    });
  }
});
