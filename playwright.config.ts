import { defineConfig, devices } from "@playwright/test";
import { PADMIN_URL } from "./e2e/helpers/services";

export default defineConfig({
  testDir: "./e2e",
  testIgnore: ["**/example-stack/**"],
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: 1,
  reporter: [["html", { open: "never", outputFolder: "D:/BuildCache/padmin-glory-branch/playwright-report" }], ["list"]],
  outputDir: "D:/BuildCache/padmin-glory-branch/test-results",
  timeout: 60_000,
  expect: { timeout: 15_000 },

  use: {
    baseURL: PADMIN_URL,
    trace: "on-first-retry",
    screenshot: "only-on-failure",
    video: "on-first-retry",
  },

  projects: [
    // Setup: verify services are up, create admin user, authenticate
    {
      name: "setup",
      testMatch: /global-setup\.ts/,
    },
    {
      name: "chromium",
      use: {
        ...devices["Desktop Chrome"],
        storageState: "e2e/.auth/admin.json",
      },
      dependencies: ["setup"],
    },
  ],
});
