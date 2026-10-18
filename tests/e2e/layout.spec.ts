import { expect, type Page } from '@playwright/test';
import { test } from './utils/fixtures';

type Box = { x: number; y: number; width: number; height: number };

/** Where every tab of a row is. */
function tabBoxes(page: Page, row: string): Promise<Box[]> {
  return page.locator(`${row} .tab`).evaluateAll(tabs => tabs.map((tab) => {
    const r = tab.getBoundingClientRect();
    return { x: Math.round(r.x * 10) / 10, y: Math.round(r.y * 10) / 10, width: Math.round(r.width * 10) / 10, height: Math.round(r.height * 10) / 10 };
  }));
}

/** Clicks every tab of a row: no tab may move or change size. */
async function expectTabsToStayPut(page: Page, row: string) {
  const tabs = page.locator(`${row} .tab`);
  const count = await tabs.count();
  expect(count).toBeGreaterThan(1);
  const first = await tabBoxes(page, row);
  for (let i = 0; i < count; i++) {
    await tabs.nth(i).click();
    expect(await tabBoxes(page, row), `after clicking tab ${i}`).toEqual(first);
  }
}

async function addLink(page: Page, url: string) {
  await page.fill('header input[type="text"]', url);
  await page.getByRole('button', { name: 'Add', exact: true }).click();
}

test.describe('tabs do not move when another one is chosen', () => {
  test('settings', async ({ page }) => {
    await page.goto('/settings');
    await expectTabsToStayPut(page, 'main nav [role=tablist]');
    // The video/audio switch of the output settings.
    await page.getByRole('tab', { name: 'Downloads', exact: true }).click();
    await expectTabsToStayPut(page, 'main section [role=tablist]');
  });

  test('a video and its preferences', async ({ page }) => {
    await page.goto('/');
    await addLink(page, 'https://example.com/video');
    await page.getByRole('link', { name: 'Show metadata' }).click();
    await expectTabsToStayPut(page, 'main nav [role=tablist]');

    await page.goto('/');
    await addLink(page, 'https://example.com/video');
    await page.getByRole('link', { name: 'Download preferences' }).first().click();
    await expectTabsToStayPut(page, 'main nav [role=tablist]');
  });

  test('the date filter', async ({ page }) => {
    await page.goto('/input-filters');
    await expectTabsToStayPut(page, 'main .tabs.tabs-box');
  });
});
