import { test, expect } from "@playwright/test";

test.describe("Federation / Destinations", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/destinations");
    await expect(page.locator("aside")).toBeVisible({ timeout: 20_000 });
  });

  test("shows destinations page", async ({ page }) => {
    await expect(page.getByRole("heading", { name: "Federation" })).toBeVisible({
      timeout: 10_000,
    });
  });

  test("shows destinations table or empty state", async ({ page }) => {
    await expect(page.locator("table")).toBeVisible({ timeout: 20_000 });
  });
});
