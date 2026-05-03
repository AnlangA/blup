/**
 * E2E tests for SandboxRunner component.
 *
 * Uses Playwright page.route() to mock all backend API responses,
 * including sandbox SSE execution.
 */
import { test, expect, type Page } from '@playwright/test';

const MOCK_SESSION_ID = 'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee';

function baseSnapshot(): Record<string, unknown> {
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
    chapter_contents: {
      ch1: [
        '# Intro to Python',
        '',
        'Here is a simple Python example:',
        '',
        '```python',
        'print("Hello, World!")',
        'for i in range(3):',
        '    print(f"Count: {i}")',
        '```',
        '',
        'And here is a bash script (no Run button expected):',
        '',
        '```bash',
        'echo "no run button"',
        '```',
      ].join('\n'),
    },
    messages: [],
    messages_total: 0,
  };
}

async function installSandboxMocks(page: Page) {
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
        return route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            id: 'mock-msg',
            role: 'assistant',
            content: (baseSnapshot().chapter_contents as Record<string, string>).ch1,
            timestamp: new Date().toISOString(),
          }),
        });
      }

      // Sandbox execution (SSE)
      if (method === 'POST' && pathname === '/api/sandbox/execute') {
        const sseBody = [
          { event: 'status', data: { state: 'running', message: 'Executing python code...' } },
          { event: 'stdout', data: { content: 'Hello, World!\n' } },
          { event: 'stdout', data: { content: 'Count: 0\n' } },
          { event: 'stdout', data: { content: 'Count: 1\n' } },
          { event: 'stdout', data: { content: 'Count: 2\n' } },
          { event: 'done', data: { result: { exit_code: 0, duration_ms: 150 } } },
        ].map(e => `event: ${e.event}\ndata: ${JSON.stringify(e.data)}\n\n`).join('');

        return route.fulfill({
          status: 200,
          contentType: 'text/event-stream',
          body: sseBody,
        });
      }

      // Sandbox health
      if (method === 'GET' && pathname === '/api/sandbox/health') {
        return route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            healthy: true,
            images: [
              { name: 'sandbox-python', version: 'mock' },
              { name: 'sandbox-node', version: 'mock' },
            ],
          }),
        });
      }

      // Chapter export
      if (pathname.includes('/export/')) {
        return route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ filename: 'test.typ', typst_source: '# test', checksum: 'abc' }),
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

test.describe('Sandbox Runner (mocked API)', () => {
  test.beforeEach(async ({ context }) => {
    await context.clearCookies();
    await context.addInitScript(() => localStorage.clear());
  });

  async function setupSessionWithChapter(page: Page) {
    await page.addInitScript(() => {
      const planId = 'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee';
      localStorage.setItem('blup_plans', JSON.stringify([{
        id: planId, title: 'Python Course', domain: 'programming',
        state: 'CHAPTER_LEARNING', createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(),
      }]));
      localStorage.setItem('blup_active_plan_id', planId);
      localStorage.setItem('blup_current_chapter_id', 'ch1');
    });
  }

  test('Run button appears on Python code blocks', async ({ page }) => {
    await installSandboxMocks(page);
    await setupSessionWithChapter(page);
    await page.goto('/');

    // The Run button should appear next to the Python code block
    const runBtn = page.getByLabel('Run python code');
    await expect(runBtn).toBeVisible({ timeout: 15000 });
  });

  test('bash code blocks do NOT show a Run button', async ({ page }) => {
    await installSandboxMocks(page);
    await setupSessionWithChapter(page);
    await page.goto('/');

    // The bash code block should NOT have a Run button
    const bashRunBtn = page.getByLabel('Run bash code');
    await expect(bashRunBtn).not.toBeVisible({ timeout: 5000 });
  });

  test('clicking Run shows output terminal with stdout', async ({ page }) => {
    await installSandboxMocks(page);
    await setupSessionWithChapter(page);
    await page.goto('/');

    const runBtn = page.getByLabel('Run python code');
    await expect(runBtn).toBeVisible({ timeout: 15000 });

    // Click Run
    await runBtn.click();

    // Output terminal should appear with stdout
    await expect(page.locator('[data-testid="sandbox-output"]')).toBeVisible({ timeout: 5000 });
    await expect(page.locator('[data-testid="sandbox-stdout"]')).toContainText('Hello, World!', { timeout: 5000 });
  });

  test('Run button is disabled while execution is in progress', async ({ page }) => {
    await installSandboxMocks(page);
    await setupSessionWithChapter(page);
    await page.goto('/');

    const runBtn = page.getByLabel('Run python code');
    await expect(runBtn).toBeVisible({ timeout: 15000 });

    // Click Run - button should become disabled while running
    await runBtn.click();

    // Button should now be disabled (showing "Running...")
    await expect(runBtn).toBeDisabled({ timeout: 2000 });
  });

  test('output shows exit code and duration after completion', async ({ page }) => {
    await installSandboxMocks(page);
    await setupSessionWithChapter(page);
    await page.goto('/');

    const runBtn = page.getByLabel('Run python code');
    await expect(runBtn).toBeVisible({ timeout: 15000 });
    await runBtn.click();

    // After completion, should show exit code and duration
    await expect(page.locator('[data-testid="sandbox-output"]')).toContainText('Exit code: 0', { timeout: 10000 });
    await expect(page.locator('[data-testid="sandbox-output"]')).toContainText('Duration: 150ms', { timeout: 5000 });
  });
});
