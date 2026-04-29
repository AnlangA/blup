import { test, expect } from '@playwright/test';

const SESSION_ID = 'test-session';

const LONG_CONTENT = '# Chapter 1\n\n' + Array.from({ length: 50 }, (_, i) =>
  `## Section ${i + 1}\n\nLorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.\n\nDuis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur.\n`
).join('\n');

const MANY_CHAPTERS = Array.from({ length: 30 }, (_, i) => ({
  id: `chapter-${i + 1}`,
  title: `Chapter ${i + 1}: Learning Topic ${i + 1}`,
  order: i + 1,
  objectives: [`Understand topic ${i + 1}`],
}));

function setupApiMocks(page: import('@playwright/test').Page) {
  return page.route(/\/api\//, async (route) => {
    const url = route.request().url();
    const method = route.request().method();

    // POST /api/session
    if (method === 'POST' && url.endsWith('/api/session')) {
      return route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ session_id: SESSION_ID, state: 'created' }),
      });
    }

    // POST /api/session/:id/goal
    if (method === 'POST' && url.includes('/goal')) {
      return route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          feasibility: {
            feasible: true,
            reason: 'This is a great learning goal!',
            suggestions: [],
            estimated_duration: '2 weeks',
            prerequisites: [],
          },
        }),
      });
    }

    // POST /api/session/:id/profile/answer
    if (method === 'POST' && url.includes('/profile/answer')) {
      return route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ is_complete: true }),
      });
    }

    // GET /api/session/:id/curriculum
    if (method === 'GET' && url.includes('/curriculum')) {
      return route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ chapters: MANY_CHAPTERS }),
      });
    }

    // GET /api/session/:id/chapter/:chapterId
    if (method === 'GET' && url.includes('/chapter/')) {
      return route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ content: LONG_CONTENT }),
      });
    }

    // POST /api/session/:id/chapter/:chapterId/ask
    if (method === 'POST' && url.includes('/ask')) {
      return route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          answer: 'Here is a detailed answer to your question. '.repeat(20),
        }),
      });
    }

    // Fallback
    return route.continue();
  });
}

async function navigateToChapterPage(page: import('@playwright/test').Page) {
  // Step 1: Page loads, createSession is called automatically
  await page.goto('/');
  await page.waitForLoadState('networkidle');
  await expect(page.getByLabel('Learning Goal')).toBeVisible({ timeout: 5000 });

  // Step 2: Submit goal
  await page.getByLabel('Learning Goal').fill('Learn Python basics');
  await page.getByLabel('Subject Domain').fill('programming');
  await page.getByRole('button', { name: 'Start Learning' }).click();

  // Step 3: Wait for feasibility result
  await expect(page.getByText('Goal Analysis')).toBeVisible({ timeout: 5000 });
  await expect(page.getByText('looks great')).toBeVisible({ timeout: 5000 });

  // Step 4: Continue to profile
  await page.getByRole('button', { name: 'Continue to Profile Setup' }).click();
  await expect(page.getByText('Tell Us About Yourself')).toBeVisible({ timeout: 5000 });

  // Step 5: Submit profile
  await page.getByLabel('No experience at all').click();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();

  // Step 6: Wait for curriculum and chapter loading
  await expect(page.getByRole('heading', { name: 'Curriculum' })).toBeVisible({ timeout: 5000 });
  await page.waitForTimeout(500); // Allow concurrent chapter fetches to complete

  // Step 7: Click first chapter
  const firstChapter = page.locator('.curriculum-sidebar li').first();
  await firstChapter.click();

  // Step 8: Wait for chapter content to appear (should be instant from cache)
  await expect(page.locator('.chapter-content')).toBeVisible({ timeout: 5000 });
}

test.describe('LearningLayout — Three Independent Scroll Panels', () => {
  test.beforeEach(async ({ context }) => {
    await context.clearCookies();
    await context.addInitScript(() => localStorage.clear());
  });

  test('layout container is constrained to viewport height', async ({ page }) => {
    await setupApiMocks(page);
    await navigateToChapterPage(page);

    const layout = await page.evaluate(() => {
      const el = document.querySelector('.learning-layout');
      if (!el) return null;
      const style = window.getComputedStyle(el);
      return {
        height: parseInt(style.height),
        overflow: style.overflow,
        viewportHeight: window.innerHeight,
      };
    });

    expect(layout).not.toBeNull();
    expect(layout!.height).toBe(layout!.viewportHeight);
    expect(layout!.overflow).toBe('hidden');
  });

  test('three panels each have overflow-y: auto', async ({ page }) => {
    await setupApiMocks(page);
    await navigateToChapterPage(page);

    const overflows = await page.evaluate(() => {
      const sidebar = document.querySelector('.curriculum-sidebar');
      const content = document.querySelector('.chapter-content');
      const messages = document.querySelector('.messages-container');

      return {
        sidebar: sidebar ? window.getComputedStyle(sidebar).overflowY : 'not-found',
        content: content ? window.getComputedStyle(content).overflowY : 'not-found',
        messages: messages ? window.getComputedStyle(messages).overflowY : 'not-found',
      };
    });

    expect(overflows.sidebar).toBe('auto');
    expect(overflows.content).toBe('auto');
    expect(overflows.messages).toBe('auto');
  });

  test('scrolling center panel does not affect left sidebar scroll position', async ({ page }) => {
    await setupApiMocks(page);
    await navigateToChapterPage(page);

    // Verify content has scrollable overflow
    const hasOverflow = await page.evaluate(() => {
      const el = document.querySelector('.chapter-content');
      return el ? el.scrollHeight > el.clientHeight : false;
    });
    expect(hasOverflow).toBe(true);

    // Record initial positions
    const initial = await page.evaluate(() => ({
      sidebar: document.querySelector('.curriculum-sidebar')?.scrollTop ?? 0,
      content: document.querySelector('.chapter-content')?.scrollTop ?? 0,
    }));

    // Scroll center panel
    await page.evaluate(() => {
      const el = document.querySelector('.chapter-content');
      if (el) el.scrollTop = 500;
    });
    await page.waitForTimeout(50);

    const after = await page.evaluate(() => ({
      sidebar: document.querySelector('.curriculum-sidebar')?.scrollTop ?? 0,
      content: document.querySelector('.chapter-content')?.scrollTop ?? 0,
    }));

    expect(after.content).toBeGreaterThan(initial.content);
    expect(after.sidebar).toBe(initial.sidebar);
  });

  test('scrolling center panel does not affect right chat panel scroll position', async ({ page }) => {
    await setupApiMocks(page);
    await navigateToChapterPage(page);

    // Send chat messages to make chat scrollable
    const chatInput = page.getByLabel('Question input');
    for (let i = 0; i < 8; i++) {
      await chatInput.fill(`Question number ${i + 1}?`);
      await page.getByRole('button', { name: 'Send' }).click();
      await page.waitForTimeout(200);
    }

    // Scroll messages to mid position
    await page.evaluate(() => {
      const el = document.querySelector('.messages-container');
      if (el) el.scrollTop = 300;
    });

    const initial = await page.evaluate(() => ({
      messages: document.querySelector('.messages-container')?.scrollTop ?? 0,
      content: document.querySelector('.chapter-content')?.scrollTop ?? 0,
    }));

    // Scroll center panel
    await page.evaluate(() => {
      const el = document.querySelector('.chapter-content');
      if (el) el.scrollTop = 600;
    });

    const after = await page.evaluate(() => ({
      messages: document.querySelector('.messages-container')?.scrollTop ?? 0,
      content: document.querySelector('.chapter-content')?.scrollTop ?? 0,
    }));

    expect(after.content).toBeGreaterThan(initial.content);
    expect(after.messages).toBe(initial.messages);
  });

  test('scrolling left sidebar does not affect center panel scroll position', async ({ page }) => {
    await setupApiMocks(page);
    await navigateToChapterPage(page);

    const hasOverflow = await page.evaluate(() => {
      const el = document.querySelector('.curriculum-sidebar');
      return el ? el.scrollHeight > el.clientHeight : false;
    });
    expect(hasOverflow).toBe(true);

    const initial = await page.evaluate(() => ({
      sidebar: document.querySelector('.curriculum-sidebar')?.scrollTop ?? 0,
      content: document.querySelector('.chapter-content')?.scrollTop ?? 0,
    }));

    // Scroll sidebar
    await page.evaluate(() => {
      const el = document.querySelector('.curriculum-sidebar');
      if (el) el.scrollTop = 400;
    });

    const after = await page.evaluate(() => ({
      sidebar: document.querySelector('.curriculum-sidebar')?.scrollTop ?? 0,
      content: document.querySelector('.chapter-content')?.scrollTop ?? 0,
    }));

    expect(after.sidebar).toBeGreaterThan(initial.sidebar);
    expect(after.content).toBe(initial.content);
  });

  test('responsive: tablet viewport (1024px) hides chat window', async ({ page }) => {
    await setupApiMocks(page);
    await navigateToChapterPage(page);
    await page.setViewportSize({ width: 1024, height: 768 });
    await page.waitForTimeout(100);

    const chatDisplay = await page.evaluate(() => {
      const el = document.querySelector('.chat-window') as HTMLElement;
      return el ? window.getComputedStyle(el).display : 'not-found';
    });

    expect(chatDisplay).toBe('none');
  });

  test('responsive: mobile viewport (768px) hides sidebar and chat', async ({ page }) => {
    await setupApiMocks(page);
    await navigateToChapterPage(page);
    await page.setViewportSize({ width: 375, height: 812 });
    await page.waitForTimeout(100);

    const display = await page.evaluate(() => {
      const sidebar = document.querySelector('.curriculum-sidebar') as HTMLElement;
      const chat = document.querySelector('.chat-window') as HTMLElement;
      return {
        sidebar: sidebar ? window.getComputedStyle(sidebar).display : 'not-found',
        chat: chat ? window.getComputedStyle(chat).display : 'not-found',
      };
    });

    expect(display.sidebar).toBe('none');
    expect(display.chat).toBe('none');
  });

  test('responsive: desktop viewport shows all three panels', async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await setupApiMocks(page);
    await navigateToChapterPage(page);

    const inDom = await page.evaluate(() => {
      return {
        sidebar: document.querySelector('.curriculum-sidebar') !== null,
        content: document.querySelector('.chapter-content') !== null,
        chat: document.querySelector('.chat-window') !== null,
      };
    });

    expect(inDom.sidebar).toBe(true);
    expect(inDom.content).toBe(true);
    expect(inDom.chat).toBe(true);
  });
});
