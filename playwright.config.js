import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: './tests/browser',
  workers: 1,
  use: { headless: true, viewport: { width: 1440, height: 960 } },
  reporter: 'list',
  timeout: 30_000,
});
