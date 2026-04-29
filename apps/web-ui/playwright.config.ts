import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './tests/e2e',
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: 1,
  reporter: 'html',
  use: {
    baseURL: 'http://localhost:5173',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
  webServer: [
    {
      command: 'cd ../../ && cargo run -p agent-core',
      port: 3000,
      reuseExistingServer: true,
      timeout: 30000,
    },
    {
      command: 'npm run dev',
      port: 5173,
      reuseExistingServer: true,
      timeout: 10000,
    },
  ],
});
