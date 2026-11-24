import { existsSync } from 'node:fs';
import { defineConfig, devices } from '@playwright/test';

const browserPath = process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE
  || (process.env.CI ? undefined : existsSync('/usr/bin/chromium') ? '/usr/bin/chromium' : undefined);

export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  reporter: 'list',
  use: {
    baseURL: 'http://127.0.0.1:4173',
    ...devices['Desktop Chrome'],
    launchOptions: { ...(browserPath ? { executablePath: browserPath } : {}), args: ['--no-sandbox'] },
  },
  webServer: {
    command: 'npm run dev -- --host 127.0.0.1 --port 4173',
    url: 'http://127.0.0.1:4173',
    reuseExistingServer: !process.env.CI,
    timeout: 30_000,
  },
});
