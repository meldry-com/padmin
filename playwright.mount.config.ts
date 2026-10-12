import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  testMatch: "single-origin.spec.ts",
  workers: 1,
  use: {
    ...devices["Desktop Chrome"],
    baseURL: process.env.PADMIN_MOUNT_TEST_URL || "http://127.0.0.1:17060",
  },
});
