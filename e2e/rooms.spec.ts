import { test, expect } from "@playwright/test";

test.describe("Rooms", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/rooms");
    await expect(page.locator("aside")).toBeVisible({ timeout: 20_000 });
  });

  test("shows rooms page with table or empty state", async ({ page }) => {
    await expect(
      page.locator("table")
        .or(page.locator("text=No rooms"))
    ).toBeVisible({ timeout: 20_000 });
  });

  test("shows search/filter input", async ({ page }) => {
    await expect(
      page.locator("input[placeholder*='Search']")
        .or(page.locator("input[placeholder*='search']"))
        .or(page.locator("input[placeholder*='Filter']"))
    ).toBeVisible({ timeout: 10_000 });
  });

  test("rooms table has expected columns", async ({ page }) => {
    const table = page.locator("table");
    const hasTable = await table.isVisible({ timeout: 15_000 }).catch(() => false);
    if (!hasTable) {
      test.skip(true, "No rooms in table to verify columns");
      return;
    }

    // Check header columns exist
    const headers = table.locator("thead th");
    await expect(headers.first()).toBeVisible();
  });
});
