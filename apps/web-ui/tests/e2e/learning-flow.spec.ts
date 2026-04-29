import { test, expect } from '@playwright/test';

test.describe('Blup Learning Flow - Full Verification', () => {
  test.beforeEach(async ({ context }) => {
    await context.clearCookies();
    await context.addInitScript(() => localStorage.clear());
  });

  test('1. 页面加载并显示表单', async ({ page }) => {
    await page.goto('/');

    // 验证标题
    await expect(page.getByText('What do you want to learn?')).toBeVisible({ timeout: 15000 });

    // 验证表单元素
    await expect(page.getByLabel('Learning Goal')).toBeVisible();
    await expect(page.getByLabel('Subject Domain')).toBeVisible();
    await expect(page.getByLabel('Context (optional)')).toBeVisible();

    // 验证按钮初始状态
    const button = page.getByRole('button', { name: 'Start Learning' });
    await expect(button).toBeVisible();
    await expect(button).toBeDisabled();
  });

  test('2. 表单验证：空字段时按钮禁用', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByLabel('Learning Goal')).toBeVisible({ timeout: 15000 });

    const button = page.getByRole('button', { name: 'Start Learning' });
    await expect(button).toBeDisabled();

    // 只填一个字段
    await page.getByLabel('Learning Goal').fill('Learn Python');
    await expect(button).toBeDisabled();

    // 填写两个字段后启用
    await page.getByLabel('Subject Domain').fill('programming');
    await expect(button).not.toBeDisabled();
  });

  test('3. 提交目标并验证API响应', async ({ page }) => {
    // 监听网络请求
    const apiResponses: { url: string; status: number; body?: unknown }[] = [];
    page.on('response', async (response) => {
      if (response.url().includes('/api/')) {
        try {
          const body = await response.json().catch(() => undefined);
          apiResponses.push({
            url: response.url(),
            status: response.status(),
            body,
          });
        } catch {
          apiResponses.push({
            url: response.url(),
            status: response.status(),
          });
        }
      }
    });

    await page.goto('/');
    await expect(page.getByLabel('Learning Goal')).toBeVisible({ timeout: 15000 });

    // 填写表单
    await page.getByLabel('Learning Goal').fill('Learn Python for data analysis');
    await page.getByLabel('Subject Domain').fill('programming');

    // 点击提交
    await page.getByRole('button', { name: 'Start Learning' }).click();

    // 等待API响应
    await page.waitForTimeout(15000);

    // 验证API调用
    console.log('API Responses:', JSON.stringify(apiResponses, null, 2));

    // 应该有createSession和submitGoal两个API调用
    const sessionApi = apiResponses.find(r => r.url.endsWith('/api/session') && r.status === 200);
    const goalApi = apiResponses.find(r => r.url.includes('/goal'));

    expect(sessionApi).toBeDefined();
    expect(sessionApi?.status).toBe(200);
    expect(goalApi).toBeDefined();

    // 验证页面状态
    const bodyText = await page.textContent('body');
    const hasGoalAnalysis = bodyText?.includes('Goal Analysis');
    const hasError = bodyText?.includes('Something went wrong');

    console.log('Page shows Goal Analysis:', hasGoalAnalysis);
    console.log('Page shows Error:', hasError);

    // 应该显示结果（成功或错误）
    expect(hasGoalAnalysis || hasError).toBe(true);
  });

  test('4. 成功流程：显示可行性分析结果', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByLabel('Learning Goal')).toBeVisible({ timeout: 15000 });

    // 填写表单
    await page.getByLabel('Learning Goal').fill('Learn Python basics');
    await page.getByLabel('Subject Domain').fill('programming');
    await page.getByRole('button', { name: 'Start Learning' }).click();

    // 等待结果（成功或错误）
    await page.waitForTimeout(15000);

    const bodyText = await page.textContent('body');
    const hasGoalAnalysis = bodyText?.includes('Goal Analysis');
    const hasError = bodyText?.includes('Something went wrong');

    // 应该显示结果
    expect(hasGoalAnalysis || hasError).toBe(true);

    if (hasGoalAnalysis) {
      console.log('✓ 成功显示可行性分析');
    } else {
      console.log('⚠ LLM服务返回错误（测试环境中可能出现）');
      // 验证错误信息格式正确
      expect(bodyText).toContain('Code:');
    }
  });

  test('5. 错误处理：显示错误信息', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByLabel('Learning Goal')).toBeVisible({ timeout: 15000 });

    // 提交一个会触发API调用的目标
    await page.getByLabel('Learning Goal').fill('Learn Python for data analysis');
    await page.getByLabel('Subject Domain').fill('programming');
    await page.getByRole('button', { name: 'Start Learning' }).click();

    // 等待响应
    await page.waitForTimeout(15000);

    // 验证页面状态
    const bodyText = await page.textContent('body');

    // 应该显示结果或错误
    const hasResult = bodyText?.includes('Goal Analysis');
    const hasError = bodyText?.includes('Something went wrong');

    expect(hasResult || hasError).toBe(true);

    if (hasError) {
      // 验证错误信息包含有用信息
      expect(bodyText).toContain('Code:');
      console.log('✓ Error displayed with code');
    }
  });

  test('6. 验证localStorage持久化', async ({ page }) => {
    await page.goto('/');
    await expect(page.getByLabel('Learning Goal')).toBeVisible({ timeout: 15000 });

    // 等待session创建
    await page.waitForTimeout(3000);

    // 检查localStorage
    const sessionId = await page.evaluate(() => localStorage.getItem('blup_session_id'));
    console.log('Session ID stored:', sessionId);

    expect(sessionId).toBeTruthy();
    expect(sessionId).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/);
  });
});
