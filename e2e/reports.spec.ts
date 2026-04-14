import { test, expect } from "@playwright/test";

test.describe("Reports", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/reports");
    await expect(page.locator("aside")).toBeVisible({ timeout: 20_000 });
  });

  test("shows reports page", async ({ page }) => {
    await expect(page.getByRole("heading", { name: "Reports" })).toBeVisible({
      timeout: 10_000,
    });
  });

  test("shows reports table or empty state", async ({ page }) => {
    await expect(page.locator("table")).toBeVisible({ timeout: 20_000 });
  });
});
