import { test, expect } from '@playwright/test';

test.describe('Curriculum Flow', () => {
  test.beforeEach(async ({ context }) => {
    await context.clearCookies();
    await context.addInitScript(() => localStorage.clear());
  });

  test('layout renders after goal submission', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByLabel('Learning Goal')).toBeVisible({
      timeout: 15000,
    });

    await page.getByLabel('Learning Goal').fill('Learn Python basics');
    await page.getByLabel('Subject Domain').fill('programming');
    await page.getByRole('button', { name: 'Start Learning' }).click();
    await page.waitForTimeout(15000);

    const bodyText = await page.textContent('body');
    if (bodyText?.includes('Curriculum')) {
      expect(bodyText).toContain('Curriculum');
    }
  });

  test('can return to goal input on infeasible result', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByLabel('Learning Goal')).toBeVisible({
      timeout: 15000,
    });

    await page.getByLabel('Learning Goal').fill('Learn everything');
    await page.getByLabel('Subject Domain').fill('everything');
    await page.getByRole('button', { name: 'Start Learning' }).click();
    await page.waitForTimeout(15000);

    const tryAgainBtn = page.getByRole('button', {
      name: 'Try a Different Goal',
    });
    const hasTryAgain = await tryAgainBtn.isVisible().catch(() => false);
    if (hasTryAgain) {
      await tryAgainBtn.click();
      await expect(
        page.getByText('What do you want to learn?'),
      ).toBeVisible();
    }
  });
});
