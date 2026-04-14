import { defineConfig } from "@playwright/test";

// The stack is driven externally via `npm run stack:up` (or manually with
// docker compose). These tests hit the compose services at their published
// ports on localhost — see examples/compose.yml.
export default defineConfig({
  testDir: "./tests",
  // The full-flow spec runs users sequentially inside one test; parallel
  // contexts within the test are safe, but we never run multiple spec files
  // against the same stack at the same time.
  fullyParallel: false,
  workers: 1,
  // Pasion's debug build has several race conditions that crash its
  // queue worker under load. The local reset script patches the known
  // FK issues, but heartbeat/mark_as_completed races still bite. Retry
  // each test up to twice so transient pasion restarts don't fail CI.
  retries: 2,
  reporter: [["list"], ["html", { open: "never" }]],
  timeout: 120_000,
  expect: { timeout: 15_000 },
  use: {
    baseURL: "http://localhost:9090",
    actionTimeout: 15_000,
    navigationTimeout: 30_000,
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    video: "retain-on-failure",
  },
});
