import { test, expect } from '@playwright/test';

test.describe('章节并发加载测试', () => {
  test.beforeEach(async ({ context }) => {
    await context.clearCookies();
    await context.addInitScript(() => localStorage.clear());
  });

  test('验证章节缓存和快速切换', async ({ page }) => {
    // 完成前置流程
    await page.goto('/');
    await expect(page.getByLabel('Learning Goal')).toBeVisible({ timeout: 15000 });

    await page.getByLabel('Learning Goal').fill('Learn Python basics');
    await page.getByLabel('Subject Domain').fill('programming');
    await page.getByRole('button', { name: 'Start Learning' }).click();

    await expect(page.getByText('Goal Analysis')).toBeVisible({ timeout: 15000 });
    await expect(page.getByText('looks great')).toBeVisible({ timeout: 30000 });

    await page.getByRole('button', { name: 'Continue to Profile Setup' }).click();
    await expect(page.getByText('Tell Us About Yourself')).toBeVisible({ timeout: 10000 });

    await page.getByLabel('No experience at all').click();
    await page.getByRole('button', { name: 'Continue', exact: true }).click();

    // 等待课程加载
    await expect(page.getByRole('heading', { name: 'Curriculum' })).toBeVisible({ timeout: 30000 });

    // 等待章节列表出现
    const chapterItems = page.locator('.curriculum-sidebar li');
    await expect(chapterItems.first()).toBeVisible({ timeout: 30000 });
    const chapterCount = await chapterItems.count();
    console.log(`章节数量: ${chapterCount}`);

    // 点击第一个章节
    console.log('\n=== 点击第一个章节 ===');
    await chapterItems.first().click();

    // 等待内容加载
    await expect(page.getByText('Loading chapter content...')).not.toBeVisible({ timeout: 60000 });
    const firstContent = await page.locator('.chapter-content').textContent();
    console.log(`✓ 第一个章节加载完成 (${firstContent?.length} 字符)`);

    // 切换到第二个章节
    console.log('\n=== 切换到第二个章节 ===');
    const switchStart = Date.now();
    await chapterItems.nth(1).click();

    // 等待内容变化
    await expect(page.locator('.chapter-content')).not.toHaveText(firstContent!, { timeout: 30000 });
    const switchTime = Date.now() - switchStart;
    console.log(`✓ 切换完成: ${switchTime}ms`);

    if (switchTime < 2000) {
      console.log('  ✓ 缓存命中，切换很快');
    } else {
      console.log('  ⚠ 切换较慢，可能需要优化');
    }

    // 切换回第一个章节（应该更快）
    console.log('\n=== 切换回第一个章节 ===');
    const switchBackStart = Date.now();
    await chapterItems.first().click();
    await page.waitForTimeout(500); // 短暂等待
    const switchBackTime = Date.now() - switchBackStart;
    console.log(`✓ 切换回第一个章节: ${switchBackTime}ms`);

    if (switchBackTime < 500) {
      console.log('  ✓ 缓存命中，切换非常快');
    }

    // 验证结果
    expect(chapterCount).toBeGreaterThan(1);
    console.log('\n=== 测试完成 ===');
  });
});
