/**
 * E2E tests for the export flow (ExportButton component).
 *
 * Uses Playwright page.route() to mock all backend API responses.
 */
import { test, expect, type Page } from '@playwright/test';

const MOCK_SESSION_ID = 'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee';
const MOCK_PDF_BASE64 = 'JVBERi0xLjQKMSAwIG9iago8PAovVHlwZSAvQ2F0YWxvZwovUGFnZXMgMiAwIFIKPj4KZW5kb2JqCg==';

function baseSnapshot(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    session_id: MOCK_SESSION_ID,
    state: 'CHAPTER_LEARNING',
    goal: { description: 'Learn Python', domain: 'programming' },
    feasibility_result: null,
    profile: null,
    profile_rounds: 0,
    curriculum: {
      title: 'Python Course',
      description: 'A test curriculum',
      chapters: [
        { id: 'ch1', title: 'Intro', order: 1, objectives: [], prerequisites: [] },
      ],
      estimated_duration: '2 weeks',
    },
    current_chapter_id: 'ch1',
    chapter_contents: { ch1: '# Intro\n\nWelcome to Python.' },
    messages: [],
    messages_total: 0,
    ...overrides,
  };
}

async function installExportMocks(page: Page) {
  await page.route(
    (url) => url.pathname.startsWith('/api/'),
    async (route) => {
      const request = route.request();
      const url = new URL(request.url());
      const method = request.method();
      const pathname = url.pathname;

      // Create session
      if (method === 'POST' && pathname === '/api/session') {
        return route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ session_id: MOCK_SESSION_ID, state: 'IDLE' }),
        });
      }

      // List sessions
      if (method === 'GET' && pathname === '/api/sessions') {
        return route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify([]),
        });
      }

      // Session snapshot
      if (method === 'GET' && pathname === `/api/session/${MOCK_SESSION_ID}`) {
        return route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify(baseSnapshot()),
        });
      }

      // Curriculum
      if (method === 'GET' && pathname === `/api/session/${MOCK_SESSION_ID}/curriculum`) {
        return route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify(baseSnapshot().curriculum),
        });
      }

      // Chapter content
      if (method === 'GET' && pathname.startsWith(`/api/session/${MOCK_SESSION_ID}/chapter/`)) {
        const chId = pathname.split('/').pop();
        return route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            id: 'mock-msg',
            role: 'assistant',
            content: `# ${chId}\n\nChapter content for ${chId}.\n\n\`\`\`python\nprint("hello")\n\`\`\``,
            timestamp: new Date().toISOString(),
          }),
        });
      }

      // Typst export - chapter
      if (method === 'POST' && pathname.includes('/export/chapter/') && pathname.endsWith('/typst')) {
        return route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            filename: 'Intro.typ',
            typst_source: '#set page()\n= Intro\nWelcome to Python.',
            checksum: 'abc123',
          }),
        });
      }

      // Typst export - curriculum
      if (method === 'POST' && pathname.includes('/export/curriculum/typst')) {
        return route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            filename: 'Python_Course.typ',
            typst_source: '#set page()\n= Python Course',
            checksum: 'def456',
          }),
        });
      }

      // PDF export - chapter (SSE)
      if (method === 'POST' && pathname.includes('/export/chapter/') && pathname.endsWith('/pdf')) {
        const sseBody = [
          { event: 'status', data: { state: 'rendering', message: 'Rendering chapter to Typst...' } },
          { event: 'status', data: { state: 'compiling', message: 'Compiling Typst to PDF...' } },
          { event: 'done', data: { result: { filename: 'Intro.pdf', pdf_base64: MOCK_PDF_BASE64, checksum: 'pdf123', size_bytes: 100, page_count: 1 } } },
        ].map(e => `event: ${e.event}\ndata: ${JSON.stringify(e.data)}\n\n`).join('');

        return route.fulfill({
          status: 200,
          contentType: 'text/event-stream',
          body: sseBody,
        });
      }

      // PDF export - curriculum (SSE)
      if (method === 'POST' && pathname.includes('/export/curriculum/pdf')) {
        const sseBody = [
          { event: 'status', data: { state: 'rendering', message: 'Rendering curriculum to Typst...' } },
          { event: 'status', data: { state: 'compiling', message: 'Compiling Typst to PDF...' } },
          { event: 'done', data: { result: { filename: 'Python_Course.pdf', pdf_base64: MOCK_PDF_BASE64, checksum: 'pdf456', size_bytes: 200, page_count: 2 } } },
        ].map(e => `event: ${e.event}\ndata: ${JSON.stringify(e.data)}\n\n`).join('');

        return route.fulfill({
          status: 200,
          contentType: 'text/event-stream',
          body: sseBody,
        });
      }

      // Fallback
      return route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({}),
      });
    },
  );
}

test.describe('Export flow (mocked API)', () => {
  test.beforeEach(async ({ context }) => {
    await context.clearCookies();
    await context.addInitScript(() => localStorage.clear());
  });

  test('export button is visible in curriculum sidebar after goal submission', async ({ page }) => {
    await installExportMocks(page);

    // Set up localStorage to simulate a completed session
    await page.addInitScript(() => {
      const planId = 'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee';
      localStorage.setItem('blup_plans', JSON.stringify([{
        id: planId, title: 'Learn Python', domain: 'programming',
        state: 'CHAPTER_LEARNING', createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(),
      }]));
      localStorage.setItem('blup_active_plan_id', planId);
      localStorage.setItem('blup_current_chapter_id', 'ch1');
    });

    await page.goto('/');

    // Export button should be in the sidebar (use .first() — there are now two buttons)
    const exportBtn = page.locator('.export-button').first();
    await expect(exportBtn).toBeVisible({ timeout: 15000 });
  });

  test('clicking export button opens dropdown with PDF and Typst options', async ({ page }) => {
    await installExportMocks(page);

    await page.addInitScript(() => {
      const planId = 'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee';
      localStorage.setItem('blup_plans', JSON.stringify([{
        id: planId, title: 'Learn Python', domain: 'programming',
        state: 'CHAPTER_LEARNING', createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(),
      }]));
      localStorage.setItem('blup_active_plan_id', planId);
      localStorage.setItem('blup_current_chapter_id', 'ch1');
    });

    await page.goto('/');

    // Wait for export button to be visible
    const trigger = page.locator('.export-trigger').first();
    await expect(trigger).toBeVisible({ timeout: 15000 });

    // Click to open dropdown
    await trigger.click();

    // Dropdown should show both options
    await expect(page.getByText('Export as PDF')).toBeVisible({ timeout: 5000 });
    await expect(page.getByText('Export as Typst')).toBeVisible({ timeout: 5000 });
  });

  test('clicking Export as Typst triggers a download', async ({ page }) => {
    await installExportMocks(page);

    await page.addInitScript(() => {
      const planId = 'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee';
      localStorage.setItem('blup_plans', JSON.stringify([{
        id: planId, title: 'Learn Python', domain: 'programming',
        state: 'CHAPTER_LEARNING', createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(),
      }]));
      localStorage.setItem('blup_active_plan_id', planId);
      localStorage.setItem('blup_current_chapter_id', 'ch1');
      // Force legacy download path (anchor click) so Playwright can intercept it
      Object.defineProperty(window, 'showSaveFilePicker', { value: undefined });
    });

    await page.goto('/');

    // Open export dropdown
    const trigger = page.locator('.export-trigger').first();
    await expect(trigger).toBeVisible({ timeout: 15000 });
    await trigger.click();

    // Set up download listener
    const downloadPromise = page.waitForEvent('download', { timeout: 10000 });

    // Click Export as Typst
    await page.getByText('Export as Typst').click();

    // Verify download starts
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toContain('.typ');
  });

  test('clicking Export as PDF triggers SSE flow and download', async ({ page }) => {
    await installExportMocks(page);

    await page.addInitScript(() => {
      const planId = 'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee';
      localStorage.setItem('blup_plans', JSON.stringify([{
        id: planId, title: 'Learn Python', domain: 'programming',
        state: 'CHAPTER_LEARNING', createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(),
      }]));
      localStorage.setItem('blup_active_plan_id', planId);
      localStorage.setItem('blup_current_chapter_id', 'ch1');
      // Force legacy download path (anchor click) so Playwright can intercept it
      Object.defineProperty(window, 'showSaveFilePicker', { value: undefined });
    });

    await page.goto('/');

    const trigger = page.locator('.export-trigger').first();
    await expect(trigger).toBeVisible({ timeout: 15000 });
    await trigger.click();

    const downloadPromise = page.waitForEvent('download', { timeout: 10000 });

    // Click Export as PDF
    await page.getByText('Export as PDF').click();

    // Verify the download happens
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toContain('.pdf');
  });

  test('export button in completion screen', async ({ page }) => {
    await installExportMocks(page);

    // Simulate a completed session
    await page.addInitScript(() => {
      const planId = 'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee';
      localStorage.setItem('blup_plans', JSON.stringify([{
        id: planId, title: 'Learn Python', domain: 'programming',
        state: 'COMPLETED', createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(),
      }]));
      localStorage.setItem('blup_active_plan_id', planId);
    });

    // Override the session state to COMPLETED
    await page.route(`/api/session/${MOCK_SESSION_ID}`, async (route) => {
      return route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(baseSnapshot({ state: 'COMPLETED' })),
      });
    });

    // Also need to mock curriculum
    await page.route(`/api/session/${MOCK_SESSION_ID}/curriculum`, async (route) => {
      return route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(baseSnapshot().curriculum),
      });
    });

    await page.goto('/');

    // Should see completion screen
    await expect(page.getByText('Congratulations!')).toBeVisible({ timeout: 15000 });

    // Export button should be there
    const exportBtn = page.locator('.export-button');
    await expect(exportBtn).toBeVisible({ timeout: 5000 });
  });
});
