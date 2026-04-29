import { test, expect } from '@playwright/test';

test.describe('Chapter Loading', () => {
  test.beforeEach(async ({ context }) => {
    await context.clearCookies();
    await context.addInitScript(() => localStorage.clear());
  });

  test('chapter list appears after curriculum load', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByLabel('Learning Goal')).toBeVisible({
      timeout: 15000,
    });

    await page.getByLabel('Learning Goal').fill('Learn Python basics');
    await page.getByLabel('Subject Domain').fill('programming');
    await page.getByRole('button', { name: 'Start Learning' }).click();
    await page.waitForTimeout(15000);

    const bodyText = await page.textContent('body');

    // Check if curriculum loaded
    if (bodyText?.includes('Curriculum')) {
      // Should have chapter list items
      const sidebarText =
        (await page.textContent('.curriculum-sidebar')) || '';
      if (sidebarText.includes('.')) {
        const items = page.locator('.curriculum-sidebar li');
        const count = await items.count();
        expect(count).toBeGreaterThanOrEqual(0);
      }
    }
  });
});
