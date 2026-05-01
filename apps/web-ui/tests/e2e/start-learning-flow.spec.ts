/**
 * E2E test for the "Start Learning" button flow.
 *
 * Uses Playwright page.route() to mock all backend API responses,
 * so the test is fully deterministic and does not require a running
 * backend or LLM gateway.
 */
import { test, expect, type Page, type Route } from '@playwright/test';

// ---------------------------------------------------------------------------
// Mock data
// ---------------------------------------------------------------------------

const MOCK_SESSION_ID = 'aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee';

const MOCK_FEASIBILITY = {
  feasible: true,
  reason: 'This is a well-scoped learning goal with clear milestones.',
  suggestions: ['Start with fundamentals', 'Practice regularly'],
  estimated_duration: '4 weeks',
  prerequisites: ['Basic computer literacy'],
};

const MOCK_PROFILE = {
  experience_level: { domain_knowledge: 'beginner' },
  learning_style: { preferred_format: ['text'], pace: 'moderate' },
  available_time: { hours_per_week: 10, session_duration_minutes: 60 },
};

const MOCK_CURRICULUM = {
  title: 'Python for Data Analysis',
  description: 'A structured curriculum',
  chapters: [
    { id: 'ch1', title: 'Introduction', order: 1, objectives: ['Understand basics'], estimated_minutes: 60, prerequisites: [] },
    { id: 'ch2', title: 'Core Concepts', order: 2, objectives: ['Master fundamentals'], estimated_minutes: 120, prerequisites: ['ch1'] },
    { id: 'ch3', title: 'Practice', order: 3, objectives: ['Apply knowledge'], estimated_minutes: 90, prerequisites: ['ch2'] },
  ],
  estimated_duration: '4-6 weeks',
};

// ---------------------------------------------------------------------------
// SSE helper
// ---------------------------------------------------------------------------

/** Build a text/event-stream response body from typed SSE events. */
function buildSseBody(events: Array<{ event: string; data: unknown }>): string {
  return events.map((e) =>
    `event: ${e.event}\ndata: ${JSON.stringify(e.data)}\n\n`,
  ).join('');
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Base session snapshot shape. */
function baseSnapshot(): Record<string, unknown> {
  return {
    session_id: MOCK_SESSION_ID,
    state: 'IDLE',
    goal: null,
    feasibility_result: null,
    profile: null,
    profile_rounds: 0,
    curriculum: null,
    current_chapter_id: null,
    chapter_contents: {},
    messages: [],
    messages_total: 0,
  };
}

/**
 * Parse an API URL to extract the action/path segments after /api/.
 */
function parseApiPath(pathname: string): {
  action: string;
  extraPath: string;
} {
  const withoutApi = pathname.replace(/^\/api\//, '');
  const parts = withoutApi.split('/');

  if (parts[0] === 'sessions') {
    return { action: 'list_sessions', extraPath: '' };
  }

  if (parts[0] === 'session' && parts.length >= 2) {
    const rest = parts.slice(2).join('/');
    return { action: rest || 'get_session', extraPath: rest };
  }

  if (parts[0] === 'session') {
    return { action: 'create_session', extraPath: '' };
  }

  return { action: 'unknown', extraPath: '' };
}

/** Install a single comprehensive route handler for all /api/* requests. */
async function installApiMocks(page: Page) {
  const stateRef: { value: Record<string, unknown> } = { value: baseSnapshot() };

  await page.route(
    (url) => {
      const pathname = new URL(url).pathname;
      return pathname === '/api/session' || pathname === '/api/sessions' || pathname.startsWith('/api/session/');
    },
    async (route: Route) => {
      const request = route.request();
      const url = new URL(request.url());
      const method = request.method();
      const { action, extraPath } = parseApiPath(url.pathname);

      // ---- POST /api/session — create session ----
      if (method === 'POST' && action === 'create_session') {
        stateRef.value = baseSnapshot();
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ session_id: MOCK_SESSION_ID, state: 'IDLE' }),
        });
        return;
      }

      // ---- GET /api/sessions — list sessions ----
      if (method === 'GET' && action === 'list_sessions') {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify([]),
        });
        return;
      }

      // ---- DELETE /api/session/:id ----
      if (method === 'DELETE' && action === 'get_session') {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ deleted: true }),
        });
        return;
      }

      // ---- GET /api/session/:id — session snapshot ----
      if (method === 'GET' && action === 'get_session') {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify(stateRef.value),
        });
        return;
      }

      // ---- POST /api/session/:id/goal/stream — SSE goal stream ----
      if (method === 'POST' && extraPath === 'goal/stream') {
        stateRef.value = {
          ...stateRef.value,
          state: 'PROFILE_COLLECTION',
          goal: {
            description: 'Learn Python for data analysis',
            domain: 'programming',
            context: null,
            current_level: null,
          },
          feasibility_result: MOCK_FEASIBILITY,
        };

        const sseBody = buildSseBody([
          { event: 'status', data: { state: 'FEASIBILITY_CHECK', message: 'Checking goal feasibility...' } },
          { event: 'done', data: { feasibility: MOCK_FEASIBILITY, state: 'PROFILE_COLLECTION' } },
        ]);

        await route.fulfill({
          status: 200,
          contentType: 'text/event-stream',
          body: sseBody,
        });
        return;
      }

      // ---- POST /api/session/:id/goal — non-streaming goal submit (fallback) ----
      if (method === 'POST' && extraPath === 'goal') {
        stateRef.value = {
          ...stateRef.value,
          state: 'PROFILE_COLLECTION',
          goal: {
            description: 'Learn Python for data analysis',
            domain: 'programming',
            context: null,
            current_level: null,
          },
          feasibility_result: MOCK_FEASIBILITY,
        };

        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            feasibility: MOCK_FEASIBILITY,
            state: 'PROFILE_COLLECTION',
          }),
        });
        return;
      }

      // ---- POST /api/session/:id/profile/answer ----
      if (method === 'POST' && extraPath === 'profile/answer') {
        const body = await request.postDataJSON();
        const round = parseInt(body.question_id?.replace('q', '') || '0', 10);

        if (round >= 2) {
          stateRef.value = {
            ...stateRef.value,
            state: 'CURRICULUM_PLANNING',
            profile: MOCK_PROFILE,
            profile_rounds: 3,
          };

          await route.fulfill({
            status: 200,
            contentType: 'application/json',
            body: JSON.stringify({
              is_complete: true,
              profile: MOCK_PROFILE,
              state: 'CURRICULUM_PLANNING',
            }),
          });
        } else {
          stateRef.value = {
            ...stateRef.value,
            state: 'PROFILE_COLLECTION',
            profile_rounds: round + 1,
          };

          await route.fulfill({
            status: 200,
            contentType: 'application/json',
            body: JSON.stringify({
              is_complete: false,
              round: round + 1,
              total_rounds: 3,
              next_question: round === 0
                ? 'How would you describe your preferred learning style?'
                : 'How much time can you dedicate each week?',
              state: 'PROFILE_COLLECTION',
            }),
          });
        }
        return;
      }

      // ---- GET /api/session/:id/curriculum ----
      if (method === 'GET' && extraPath === 'curriculum') {
        stateRef.value = {
          ...stateRef.value,
          state: 'CHAPTER_LEARNING',
          curriculum: MOCK_CURRICULUM,
          current_chapter_id: 'ch1',
        };

        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify(MOCK_CURRICULUM),
        });
        return;
      }

      // ---- GET /api/session/:id/chapter/:ch_id ----
      if (method === 'GET' && extraPath.startsWith('chapter/')) {
        const chId = extraPath.replace('chapter/', '').replace('/stream', '');
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            id: 'mock-msg-id',
            role: 'assistant',
            content: `# ${chId}\n\nThis is the chapter content for ${chId}.`,
            timestamp: new Date().toISOString(),
          }),
        });
        return;
      }

      // ---- POST /api/session/:id/chapter/:ch_id/ask ----
      if (method === 'POST' && extraPath.includes('/ask')) {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            id: 'mock-answer-id',
            role: 'assistant',
            content: 'This is a mock answer to your question.',
            timestamp: new Date().toISOString(),
          }),
        });
        return;
      }

      // ---- POST /api/session/:id/chapter/:ch_id/complete ----
      if (method === 'POST' && extraPath.includes('/complete')) {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            chapter_id: 'ch1',
            status: 'completed',
            completion: 100,
            last_accessed: new Date().toISOString(),
          }),
        });
        return;
      }

      // Fallback
      console.warn(`[mock] Unhandled API request: ${method} ${url.pathname}`);
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({}),
      });
    },
  );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

test.describe('Start Learning flow (mocked API)', () => {
  test.beforeEach(async ({ context }) => {
    await context.clearCookies();
    await context.addInitScript(() => localStorage.clear());
  });

  test('goal form is visible and submit button starts disabled', async ({ page }) => {
    await installApiMocks(page);
    await page.goto('/');

    await expect(page.getByText('What do you want to learn?')).toBeVisible({ timeout: 15000 });
    await expect(page.getByLabel('Learning Goal')).toBeVisible();
    await expect(page.getByLabel('Subject Domain')).toBeVisible();

    const button = page.getByRole('button', { name: 'Start Learning' });
    await expect(button).toBeVisible();
    await expect(button).toBeDisabled();
  });

  test('filling required fields enables the submit button', async ({ page }) => {
    await installApiMocks(page);
    await page.goto('/');

    await expect(page.getByLabel('Learning Goal')).toBeVisible({ timeout: 15000 });

    const button = page.getByRole('button', { name: 'Start Learning' });
    await expect(button).toBeDisabled();

    await page.getByLabel('Learning Goal').fill('Learn Python for data analysis');
    await expect(button).toBeDisabled();

    await page.getByLabel('Subject Domain').fill('programming');
    await expect(button).not.toBeDisabled();
  });

  test('clicking Start Learning shows feasibility result and profile question', async ({ page }) => {
    await installApiMocks(page);
    await page.goto('/');

    await expect(page.getByLabel('Learning Goal')).toBeVisible({ timeout: 15000 });

    // Fill and submit the form
    await page.getByLabel('Learning Goal').fill('Learn Python for data analysis');
    await page.getByLabel('Subject Domain').fill('programming');
    await page.getByRole('button', { name: 'Start Learning' }).click();

    // After successful submission, feasibility result should appear
    await expect(page.getByText('Goal Analysis')).toBeVisible({ timeout: 10000 });
    await expect(page.getByText('Your learning goal looks great!')).toBeVisible({ timeout: 10000 });

    // Profile question should also appear
    await expect(page.getByText('Tell Us About Yourself')).toBeVisible({ timeout: 10000 });
  });

  test('full flow: goal -> feasibility -> profile -> curriculum', async ({ page }) => {
    await installApiMocks(page);
    await page.goto('/');

    await expect(page.getByLabel('Learning Goal')).toBeVisible({ timeout: 15000 });

    // Step 1: Submit goal
    await page.getByLabel('Learning Goal').fill('Learn Python for data analysis');
    await page.getByLabel('Subject Domain').fill('programming');
    await page.getByRole('button', { name: 'Start Learning' }).click();

    // Step 2: Verify feasibility result is shown
    await expect(page.getByText('Your learning goal looks great!')).toBeVisible({ timeout: 10000 });

    // Step 3: Answer profile questions (3 rounds)
    // Round 1: Experience
    await expect(page.getByText('What experience do you have with this subject?')).toBeVisible({ timeout: 5000 });
    await page.getByLabel('No experience at all').check();
    await page.getByRole('button', { name: 'Continue' }).click();

    // Round 2: Learning style
    await expect(page.getByText('How do you prefer to learn new material?')).toBeVisible({ timeout: 5000 });
    await page.getByLabel('Reading text and documentation').check();
    await page.getByRole('button', { name: 'Continue' }).click();

    // Round 3: Time availability
    await expect(page.getByText('How much time can you dedicate each week?')).toBeVisible({ timeout: 5000 });
    await page.getByLabel('2-5 hours').check();
    await page.getByRole('button', { name: 'Complete Profile' }).click();

    // Step 4: Curriculum should load — verify the curriculum header appears
    await expect(page.locator('.curriculum-sidebar').getByRole('heading', { name: 'Curriculum' })).toBeVisible({ timeout: 10000 });
    // Verify the sidebar shows chapter titles from the curriculum
    await expect(page.locator('.curriculum-sidebar').getByText('Introduction')).toBeVisible({ timeout: 5000 });
    await expect(page.locator('.curriculum-sidebar').getByText('Core Concepts')).toBeVisible({ timeout: 5000 });
    await expect(page.locator('.curriculum-sidebar').getByText('Practice')).toBeVisible({ timeout: 5000 });
  });
});
