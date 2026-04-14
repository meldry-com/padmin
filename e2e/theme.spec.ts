import { test, expect } from "@playwright/test";

async function setStoredTheme(page: Parameters<typeof test>[0]["page"], theme: "light" | "dark") {
  await page.evaluate((value) => {
    localStorage.setItem("theme", JSON.stringify(value));
  }, theme);
}

test.describe("Theme Preferences on Login", () => {
  test.use({ storageState: { cookies: [], origins: [] } });

  test("applies stored dark and light themes on the login page", async ({ page }) => {
    await page.goto("/login");
    await expect(page.getByRole("heading", { name: "Palpo Admin" })).toBeVisible({ timeout: 20_000 });

    await setStoredTheme(page, "dark");
    await page.reload();
    await expect(page.locator("html")).toHaveClass(/dark/);

    await setStoredTheme(page, "light");
    await page.reload();

    await expect(page.locator("html")).not.toHaveClass(/dark/);
  });
});

test.describe("Theme Toggle", () => {
  test("switches between light and dark modes and persists after reload", async ({ page }) => {
    await page.goto("/");
    await expect(page.locator("aside")).toBeVisible({ timeout: 20_000 });

    await setStoredTheme(page, "light");
    await page.reload();
    await expect(page.locator("aside")).toBeVisible({ timeout: 20_000 });
    await expect(page.locator("html")).not.toHaveClass(/dark/);

    await page.locator('header button[title="Switch to dark mode"]').click();
    await expect(page.locator("html")).toHaveClass(/dark/);

    await page.reload();
    await expect(page.locator("html")).toHaveClass(/dark/);

    await page.locator('header button[title="Switch to light mode"]').click();
    await expect(page.locator("html")).not.toHaveClass(/dark/);
  });
});