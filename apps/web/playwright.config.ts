import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: './tests/e2e',
  fullyParallel: false,
  workers: 1,
  reporter: 'list',
  timeout: 60_000,
  use: {
    baseURL: 'http://127.0.0.1:4179',
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
    viewport: { width: 1600, height: 1000 },
  },
  projects: [
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        browserName: 'chromium',
        viewport: { width: 1600, height: 1000 },
      },
    },
    {
      name: 'mobile-chromium',
      testMatch: /responsive\.spec\.ts/,
      use: {
        ...devices['Pixel 5'],
        browserName: 'chromium',
        viewport: { width: 390, height: 844 },
      },
    },
  ],
  webServer: {
    command: 'tsx scripts/e2e-web-server.ts --port 4179',
    env: {
      ASTERWEFT_E2E: '1',
      ASTERWEFT_LIBRARY_PATH: './tests/fixtures/library',
    },
    gracefulShutdown: {
      signal: 'SIGTERM',
      timeout: 5_000,
    },
    reuseExistingServer: false,
    timeout: 120_000,
    url: 'http://127.0.0.1:4179',
  },
});
