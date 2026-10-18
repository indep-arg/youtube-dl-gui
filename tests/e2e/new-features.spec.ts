import { expect, type Page } from '@playwright/test';
import { test } from './utils/fixtures';

async function addLink(page: Page, url: string) {
  await page.fill('input[type="text"]', url);
  await page.getByRole('button', { name: 'Add', exact: true }).click();
}

async function saveSettings(page: Page) {
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(page.getByText('Settings saved!')).toBeVisible();
}

function settingsStore(page: Page) {
  return page.evaluate(() => {
    const debug = (window as unknown as {
      __OVD_DEBUG__: { getStore: (id: string) => { settings: Record<string, Record<string, unknown>> } };
    }).__OVD_DEBUG__;
    return JSON.parse(JSON.stringify(debug.getStore('settings').settings)) as Record<string, Record<string, unknown>>;
  });
}

test.describe('download options', () => {
  test('the new options are in the settings and are saved', async ({ page }) => {
    await page.goto('/settings');
    for (const label of [
      'Save info file:',
      'Save description:',
      'Split chapters into files:',
      'Record live streams from the start:',
      'Limit download speed:',
      'Retries:',
      'Parts downloaded at once:',
    ]) {
      await expect(page.getByText(label, { exact: true }).first()).toBeAttached();
    }

    await page.locator('select[aria-label="Limit download speed:"]').selectOption('mb');
    await page.fill('#rateLimit', '2');
    await page.fill('#retries', '3');
    await saveSettings(page);

    const settings = await settingsStore(page);
    expect(settings.performance.rateLimitKib).toBe(2048);
    expect(settings.performance.retries).toBe(3);
  });

  test('settings can be exported and imported from the save menu', async ({ page }) => {
    await page.goto('/settings');
    await page.evaluate(() => {
      (globalThis as unknown as { __E2E_IMPORTED_SETTINGS__: unknown }).__E2E_IMPORTED_SETTINGS__ = {
        performance: { retries: 7 },
      };
    });
    const menu = page.locator('form').getByRole('button', { name: 'More queue actions' });

    await menu.click();
    await page.getByRole('button', { name: 'Export settings…' }).click();
    await expect(page.getByText('Settings exported.')).toBeVisible();

    await menu.click();
    await page.getByRole('button', { name: 'Import settings…' }).click();
    await expect(page.getByText('Settings imported.')).toBeVisible();
    await expect(page.locator('#retries')).toHaveValue('7');
  });
});

test('a live stream gets a LIVE badge and no size', async ({ page }) => {
  await page.goto('/');
  await addLink(page, 'https://example.com/live');
  await expect(page.getByText('LIVE', { exact: true })).toBeVisible();
  await expect(page.getByText('Live stream: it is recorded until it ends.')).toBeVisible();
  await expect(page.getByText('Size:')).toHaveCount(0);
});

test('a live recording shows what it recorded and can be stopped and saved', async ({ page }) => {
  await page.goto('/');
  await addLink(page, 'https://example.com/live');
  await expect(page.getByText('Live Video')).toBeVisible();
  await page.getByRole('button', { name: 'Download', exact: true }).click();

  await expect(page.getByText('Recording live stream… 3.0 MB, 1:15')).toBeVisible();
  await expect(page.getByTitle('Pause download')).toHaveCount(0);
  await page.getByTitle('Stop and save recording').click();

  await expect(page.getByText('Download complete – 1 item downloaded.')).toBeVisible();
  await page.getByRole('link', { name: 'Download history' }).click();
  await expect(page.getByTestId('history-entry').first()).toContainText('Live Video');
});

test.describe('yt-dlp channel', () => {
  test('choosing nightly installs yt-dlp again', async ({ page }) => {
    await page.goto('/settings/system');
    await expect(page.getByText('Nightly builds get fixes for sites sooner')).toBeVisible();
    await page.selectOption('#ytDlpChannel', 'nightly');
    await saveSettings(page);
    await expect(page).toHaveURL(/\/install$/);
    await expect(page.getByText('yt-dlp')).toBeVisible();
  });

  test('with a download going on, the change waits for the next start', async ({ page }) => {
    await page.goto('/');
    await addLink(page, 'https://example.com/video');
    await expect(page.getByText('Test Video')).toBeVisible();
    await page.getByRole('button', { name: 'Download', exact: true }).click();

    // Inside the app: loading the page again would end the download.
    await page.getByRole('link', { name: 'App Settings' }).click();
    await page.getByRole('tab', { name: 'System', exact: true }).click();
    await page.selectOption('#ytDlpChannel', 'nightly');
    await saveSettings(page);
    await expect(page.getByText('The new version of yt-dlp will be installed the next time you open the app.')).toBeVisible();
    await expect(page).toHaveURL(/\/settings\/system$/);
  });

  test('a site that changed suggests the nightly builds', async ({ page }) => {
    await page.goto('/');
    await addLink(page, 'https://example.com/site-changed');
    await page.getByRole('link', { name: 'Try yt-dlp nightly' }).click();
    await expect(page.locator('#ytDlpChannel')).toBeVisible();
  });

  test('the channel cannot be chosen without tool updates', async ({ page }) => {
    await page.goto('/settings/system');
    await page.locator('#updateBinaries').uncheck();
    await expect(page.locator('#ytDlpChannel')).toBeDisabled();
  });
});

test.describe('history', () => {
  test('a finished download shows up in the history and can be removed', async ({ page }) => {
    await page.goto('/');
    await addLink(page, 'https://example.com/finish-video');
    await expect(page.getByText('Test Video')).toBeVisible();
    await page.getByRole('button', { name: 'Download', exact: true }).click();

    await page.getByRole('link', { name: 'Download history' }).click();
    const entries = page.getByTestId('history-entry');
    await expect(entries).toHaveCount(1);
    await expect(entries.first()).toContainText('Test Video');
    await expect(page.getByRole('button', { name: 'Show in folder' })).toBeVisible();

    await page.getByRole('button', { name: 'Remove from history' }).click();
    await expect(page.getByText('Finished downloads will show up here.')).toBeVisible();
  });

  test('clearing the history asks for a second click', async ({ page }) => {
    await page.addInitScript(() => {
      (globalThis as unknown as { __E2E_LIBRARY__: unknown }).__E2E_LIBRARY__ = {
        history: [{
          id: 'h1', url: 'https://example.com/old', title: 'Old video', thumbnail: null, uploader: null,
          path: null, items: 1, finishedAt: Date.now(), isLive: false,
        }],
        queue: [],
      };
    });
    await page.goto('/history');
    await expect(page.getByTestId('history-entry')).toHaveCount(1);

    await page.getByRole('button', { name: 'Clear history' }).click();
    await expect(page.getByTestId('history-entry')).toHaveCount(1);
    await page.getByRole('button', { name: 'Click again to clear' }).click();
    await expect(page.getByTestId('history-entry')).toHaveCount(0);
  });
});

test.describe('queue between sessions', () => {
  test('the links of the last session come back', async ({ page }) => {
    await page.addInitScript(() => {
      (globalThis as unknown as { __E2E_LIBRARY__: unknown }).__E2E_LIBRARY__ = {
        history: [],
        queue: [{ url: 'https://example.com/restored', overrides: null, options: null }],
      };
    });
    await page.goto('/');
    await expect(page.getByText('Restored 1 link from the last session.')).toBeVisible();
    await expect(page.getByText('Test Video')).toBeVisible();
  });

  test('the queue is saved when it changes', async ({ page }) => {
    await page.goto('/');
    await addLink(page, 'https://example.com/saved');
    await expect(page.getByText('Test Video')).toBeVisible();

    await expect.poll(() => page.evaluate(() => {
      const library = (globalThis as unknown as { __E2E_LIBRARY__?: { queue: { url: string }[] } }).__E2E_LIBRARY__;
      return library?.queue.map(link => link.url) ?? [];
    })).toEqual(['https://example.com/saved']);
  });
});

test('a site preset sets the download type of its links', async ({ page }) => {
  await page.goto('/settings');
  await page.getByRole('button', { name: 'Add a site' }).click();
  await page.fill('#site-preset-domain-0', 'https://www.Example.com/');
  await page.locator('#site-preset-domain-0').blur();
  await expect(page.locator('#site-preset-domain-0')).toHaveValue('example.com');
  await page.selectOption('#site-preset-type-0', 'audio');
  await saveSettings(page);

  await page.getByRole('link', { name: 'Back' }).click();
  await addLink(page, 'https://example.com/video1');
  await expect(page.getByText('Test Video')).toBeVisible();

  const trackTypes = await page.evaluate(() => {
    const debug = (window as unknown as {
      __OVD_DEBUG__: { getStore: (id: string) => { groupOptions: Record<string, { trackType: string }> } };
    }).__OVD_DEBUG__;
    return Object.values(debug.getStore('media-options').groupOptions).map(options => options.trackType);
  });
  expect(trackTypes).toContain('audio');
});

test('an empty playlist is shown as an error', async ({ page }) => {
  await page.goto('/');
  await addLink(page, 'https://example.com/empty-playlist');
  await expect(page.getByText('Error - Empty playlist')).toBeVisible();
  await expect(page.getByText('This playlist has no videos, or none of them can be downloaded.')).toBeVisible();
});
