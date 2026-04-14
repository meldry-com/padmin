import { test, expect } from "@playwright/test";

test.describe("Registration Tokens", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/registration-tokens");
    await expect(page.locator("aside")).toBeVisible({ timeout: 20_000 });
  });

  test("shows registration tokens page", async ({ page }) => {
    await expect(page.getByRole("heading", { name: "Registration Tokens" })).toBeVisible({
      timeout: 10_000,
    });
  });

  test("shows tokens table or empty state", async ({ page }) => {
    await expect(page.locator("table")).toBeVisible({ timeout: 20_000 });
  });
});
