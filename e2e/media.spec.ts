import { test, expect } from "@playwright/test";

test.describe("Media", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/media");
    await expect(page.locator("aside")).toBeVisible({ timeout: 20_000 });
  });

  test("shows media management page", async ({ page }) => {
    await expect(page.getByRole("heading", { name: "Media" })).toBeVisible({
      timeout: 10_000,
    });
  });

  test("shows media table or empty state", async ({ page }) => {
    await expect(page.locator("table")).toBeVisible({ timeout: 20_000 });
  });
});
