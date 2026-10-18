import { expect } from '@playwright/test';
import { test } from './utils/fixtures';

test('the diagnostic information is shown and copied', async ({ page }) => {
  await page.goto('/settings/about');

  await page.getByRole('button', { name: 'Copy diagnostic information' }).click();

  await expect(page.locator('#diagnostics-text')).toContainText('Open Video Downloader 2026.10.4');
  await expect(page.locator('#diagnostics-text')).toContainText('- yt-dlp 2026.08.19: runs');
  await expect(page.getByText('Diagnostic information copied.')).toBeVisible();
  const copied = await page.evaluate(() => globalThis.__E2E_CLIPBOARD__);
  expect(copied).toContain('System: linux x86_64');
});

test('the about page links to this project', async ({ page }) => {
  await page.goto('/settings/about');

  for (const [name, href] of [
    ['User guide', '/docs/GUIDE.md'],
    ['Privacy', '/docs/PRIVACY.md'],
    ['Changelog', '/CHANGELOG.md'],
  ]) {
    const link = page.getByRole('link', { name, exact: true });
    await expect(link).toHaveAttribute('href', new RegExp(`indep-arg/youtube-dl-gui/blob/main${href}$`));
  }
});
