import { test, expect } from "@playwright/test";

test.describe("Dashboard", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await expect(page.locator("aside")).toBeVisible({ timeout: 20_000 });
  });

  test("shows dashboard heading and welcome message", async ({ page }) => {
    await expect(page.locator("h1")).toContainText("Dashboard");
    await expect(page.locator("text=Welcome to Palpo Admin")).toBeVisible();
  });

  test("shows server version stat card with real data", async ({ page }) => {
    const versionCard = page.getByRole("heading", { name: "Server Version" });
    await expect(versionCard).toBeVisible({ timeout: 20_000 });
    // The stat card should contain an actual version string
    const card = versionCard.locator("..");
    await expect(card).not.toContainText("Loading");
  });

  test("shows server features section", async ({ page }) => {
    await expect(
      page.getByRole("heading", { name: "Server Features" })
    ).toBeVisible({ timeout: 20_000 });
  });

  test("shows user and room statistics", async ({ page }) => {
    await expect(page.getByRole("heading", { name: "Users" })).toBeVisible({
      timeout: 20_000,
    });
    await expect(page.getByRole("heading", { name: "Rooms" })).toBeVisible({
      timeout: 20_000,
    });
  });
});
