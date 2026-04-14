import { defineConfig, devices } from "@playwright/test";

// Service URLs matching examples/compose.yml
export const PADMIN_URL = process.env.PADMIN_URL || "http://localhost:9090";
export const PALPO_URL = process.env.PALPO_URL || "http://localhost:8008";
export const PASION_URL = process.env.PASION_URL || "http://localhost:8090";
export const ELEMENT_URL = process.env.ELEMENT_URL || "http://localhost:8080";

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: 1,
  reporter: [["html"], ["list"]],
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
