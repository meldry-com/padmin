import { test, expect } from "@playwright/test";

test.describe("404 Not Found", () => {
  test("shows 404 page for unknown routes", async ({ page }) => {
    await page.goto("/some/unknown/route");
    await expect(page.locator("text=404")).toBeVisible({ timeout: 20_000 });
    await expect(page.locator("text=Page not found")).toBeVisible();
    await expect(page.locator("text=Go to Dashboard")).toBeVisible();
  });

  test("can navigate back to dashboard from 404", async ({ page }) => {
    await page.goto("/nonexistent");
    await expect(page.locator("text=404")).toBeVisible({ timeout: 20_000 });
    await page.locator("text=Go to Dashboard").click();
    await expect(page.locator("h1")).toContainText("Dashboard");
    await expect(page).toHaveURL("/");
  });
});
