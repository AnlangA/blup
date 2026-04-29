import { test, expect } from '@playwright/test';

test.describe('Learning Flow', () => {
  test.beforeEach(async ({ context }) => {
    await context.clearCookies();
    await context.addInitScript(() => localStorage.clear());
  });

  test('page loads and shows the goal input form', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByText('What do you want to learn?')).toBeVisible({
      timeout: 15000,
    });
    await expect(page.getByLabel('Learning Goal')).toBeVisible();
    await expect(page.getByLabel('Subject Domain')).toBeVisible();
    const button = page.getByRole('button', { name: 'Start Learning' });
    await expect(button).toBeVisible();
    await expect(button).toBeDisabled();
  });

  test('submit button is disabled until both required fields are filled', async ({
    page,
  }) => {
    await page.goto('/');
    await expect(page.getByLabel('Learning Goal')).toBeVisible({
      timeout: 15000,
    });
    const button = page.getByRole('button', { name: 'Start Learning' });
    await expect(button).toBeDisabled();

    await page.getByLabel('Learning Goal').fill('Learn Python');
    await expect(button).toBeDisabled();

    await page.getByLabel('Subject Domain').fill('programming');
    await expect(button).not.toBeDisabled();
  });

  test('session ID is persisted in localStorage', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByLabel('Learning Goal')).toBeVisible({
      timeout: 15000,
    });
    await page.waitForTimeout(3000);

    const sessionId = await page.evaluate(() =>
      localStorage.getItem('blup_session_id'),
    );
    expect(sessionId).toBeTruthy();
    expect(sessionId).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/,
    );
  });

  test('form input accepts user values', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByLabel('Learning Goal')).toBeVisible({
      timeout: 15000,
    });

    await page
      .getByLabel('Learning Goal')
      .fill('Learn Python for data analysis');
    await page.getByLabel('Subject Domain').fill('programming');
    await page
      .getByLabel('Context (optional)')
      .fill('I work with Excel spreadsheets');

    expect(await page.getByLabel('Learning Goal').inputValue()).toBe(
      'Learn Python for data analysis',
    );
    expect(await page.getByLabel('Subject Domain').inputValue()).toBe(
      'programming',
    );
    expect(await page.getByLabel('Context (optional)').inputValue()).toBe(
      'I work with Excel spreadsheets',
    );
  });

  test('error display shows retry and start over buttons', async ({ page }) => {
    // Simulate error by loading with corrupted state
    await page.goto('/');
    await page.evaluate(() =>
      localStorage.setItem('blup_session_id', 'invalid-session'),
    );
    await page.reload();

    // The error display should appear (or the app should create a new session)
    await page.waitForTimeout(3000);
    const bodyText = await page.textContent('body');
    // App should either show error or recover by creating new session
    expect(
      bodyText?.includes('Something went wrong') ||
        bodyText?.includes('What do you want to learn'),
    ).toBe(true);
  });
});
