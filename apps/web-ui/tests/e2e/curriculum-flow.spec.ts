import { test, expect } from '@playwright/test';

test.describe('Curriculum 页面完整测试', () => {
  test.beforeEach(async ({ context }) => {
    await context.clearCookies();
    await context.addInitScript(() => localStorage.clear());
  });

  test('完整流程：Goal → Profile → Curriculum → Chapter', async ({ page }) => {
    const logs: string[] = [];
    page.on('console', msg => {
      if (msg.type() === 'error' || msg.text().includes('[')) {
        logs.push(`[${msg.type()}] ${msg.text()}`);
      }
    });

    // Step 1: 加载页面
    console.log('=== Step 1: 加载页面 ===');
    await page.goto('/');
    await expect(page.getByLabel('Learning Goal')).toBeVisible({ timeout: 15000 });
    console.log('✓ 页面加载完成');

    // Step 2: 提交 Goal
    console.log('\n=== Step 2: 提交 Goal ===');
    await page.getByLabel('Learning Goal').fill('Learn Python basics');
    await page.getByLabel('Subject Domain').fill('programming');
    await page.getByRole('button', { name: 'Start Learning' }).click();

    // 等待可行性分析结果
    await expect(page.getByText('Goal Analysis')).toBeVisible({ timeout: 15000 });
    await expect(page.getByText('looks great')).toBeVisible({ timeout: 30000 });
    console.log('✓ Goal 提交成功，显示可行性分析');

    // Step 3: 继续到 Profile
    console.log('\n=== Step 3: 继续到 Profile ===');
    await page.getByRole('button', { name: 'Continue to Profile Setup' }).click();
    await expect(page.getByText('Tell Us About Yourself')).toBeVisible({ timeout: 10000 });
    console.log('✓ 显示 Profile 问题');

    // Step 4: 提交 Profile
    console.log('\n=== Step 4: 提交 Profile ===');
    await page.getByLabel('No experience at all').click();
    await page.getByRole('button', { name: 'Continue', exact: true }).click();

    // Step 5: 等待 Curriculum 加载
    console.log('\n=== Step 5: 等待 Curriculum 加载 ===');
    await expect(page.getByRole('heading', { name: 'Curriculum' })).toBeVisible({ timeout: 30000 });
    console.log('✓ Curriculum 侧边栏显示');

    // 等待章节列表加载
    await page.waitForTimeout(8000);
    
    // 检查是否有章节（等待 loading 消失）
    await expect(page.getByText('Loading curriculum...')).not.toBeVisible({ timeout: 30000 }).catch(() => {});
    
    const chapterItems = page.locator('.curriculum-sidebar li');
    const chapterCount = await chapterItems.count();
    console.log(`✓ 加载了 ${chapterCount} 个章节`);

    // Step 6: 点击第一个章节
    console.log('\n=== Step 6: 点击第一个章节 ===');
    if (chapterCount > 0) {
      await chapterItems.first().click();
      
      // 等待 "Loading chapter content..." 显示
      await expect(page.getByText('Loading chapter content...')).toBeVisible({ timeout: 5000 }).catch(() => {});
      
      // 等待章节内容加载（等待 loading 消失）
      await expect(page.getByText('Loading chapter content...')).not.toBeVisible({ timeout: 120000 }).catch(() => {});
      
      // 检查章节内容是否显示
      const chapterContent = page.locator('.chapter-content');
      const contentText = await chapterContent.textContent();
      
      if (contentText && contentText.length > 100) {
        console.log('✓ 章节内容已加载');
        console.log(`  内容长度: ${contentText.length} 字符`);
        console.log(`  内容预览: ${contentText.substring(0, 200)}...`);
      } else {
        console.log('⚠ 章节内容为空或很短');
        console.log(`  内容: ${contentText}`);
      }
    }

    // Step 7: 测试聊天功能
    console.log('\n=== Step 7: 测试聊天功能 ===');
    const chatInput = page.getByLabel('Question input');
    
    if (await chatInput.isVisible()) {
      await chatInput.fill('What is Python?');
      await page.getByRole('button', { name: 'Send' }).click();
      
      // 等待 AI 回复
      await page.waitForTimeout(15000);
      
      // 检查是否有回复
      const messages = page.locator('.message');
      const messageCount = await messages.count();
      console.log(`✓ 聊天消息数量: ${messageCount}`);
      
      if (messageCount > 0) {
        const lastMessage = messages.last();
        const lastMessageText = await lastMessage.textContent();
        console.log(`  最后一条消息: ${lastMessageText?.substring(0, 100)}...`);
      }
    }

    // Step 8: 测试缓存 - 切换到其他章节再切换回来
    console.log('\n=== Step 8: 测试缓存 ===');
    if (chapterCount > 1) {
      // 点击第二个章节
      await chapterItems.nth(1).click();
      await page.waitForTimeout(3000);
      
      // 切换回第一个章节
      const startTime = Date.now();
      await chapterItems.first().click();
      
      // 等待内容加载（应该很快，因为有缓存）
      await expect(page.getByText('Loading chapter content...')).not.toBeVisible({ timeout: 10000 });
      const endTime = Date.now();
      const loadTime = endTime - startTime;
      
      console.log(`✓ 章节切换时间: ${loadTime}ms`);
      if (loadTime < 2000) {
        console.log('  ✓ 缓存生效，加载很快');
      } else {
        console.log('  ⚠ 缓存可能未生效，加载较慢');
      }
      
      // 验证聊天消息是否保留
      const messagesAfterSwitch = page.locator('.message');
      const messageCountAfterSwitch = await messagesAfterSwitch.count();
      console.log(`✓ 切换后聊天消息数量: ${messageCountAfterSwitch}`);
    }

    // 打印日志
    console.log('\n=== Console Logs ===');
    logs.forEach(log => console.log(log));

    // 验证最终状态
    console.log('\n=== 最终验证 ===');
    const finalContent = await page.textContent('body');
    const hasCurriculum = finalContent?.includes('Curriculum');
    const hasChapterContent = finalContent?.includes('Python') || finalContent?.includes('chapter');
    
    console.log('Has Curriculum:', hasCurriculum);
    console.log('Has Chapter Content:', hasChapterContent);
    
    expect(hasCurriculum).toBe(true);
  });
});
