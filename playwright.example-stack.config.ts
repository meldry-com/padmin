import { defineConfig } from "@playwright/test";
import { PADMIN_URL } from "./e2e/helpers/services";

export default defineConfig({
  testDir: "./e2e/example-stack",
  fullyParallel: false,
  workers: 1,
  retries: 2,
  reporter: [
    ["list"],
    ["html", { open: "never", outputFolder: "playwright-report/example-stack" }],
  ],
  outputDir: "test-results/example-stack",
  timeout: 120_000,
  expect: { timeout: 15_000 },
  use: {
    baseURL: PADMIN_URL,
    actionTimeout: 15_000,
    navigationTimeout: 30_000,
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    video: "retain-on-failure",
  },
});
