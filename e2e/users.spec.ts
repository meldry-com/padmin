import { test, expect } from "@playwright/test";
import { PALPO_URL } from "./helpers/services";

test.describe("Users", () => {
  test.describe("User List", () => {
    test.beforeEach(async ({ page }) => {
      await page.goto("/users");
      await expect(page.locator("aside")).toBeVisible({ timeout: 20_000 });
    });

    test("shows user management page", async ({ page }) => {
      await expect(page.locator("text=Manage Matrix users")).toBeVisible();
      await expect(page.locator("a", { hasText: "Create User" })).toBeVisible();
    });

    test("loads and displays users in table", async ({ page }) => {
      // Wait for the user table to load with real data
      await expect(page.locator("table")).toBeVisible({ timeout: 20_000 });
      // Should have at least the admin user
      await expect(page.locator("table tbody tr").first()).toBeVisible();
    });

    test("search filters users", async ({ page }) => {
      await expect(page.locator("table")).toBeVisible({ timeout: 20_000 });
      const searchInput = page.locator('input[placeholder="Search users..."]');
      await searchInput.fill("admin");
      // Table should still show results (admin user exists)
      await expect(page.locator("table tbody tr").first()).toBeVisible({ timeout: 10_000 });
    });

    test("can navigate to Create User page", async ({ page }) => {
      await page.locator("a", { hasText: "Create User" }).click();
      await expect(page).toHaveURL(/\/users\/create/);
      await expect(page.locator("text=Create New User")).toBeVisible();
    });
  });

  test.describe("Create User", () => {
    test.beforeEach(async ({ page }) => {
      await page.goto("/users/create");
      await expect(page.locator("aside")).toBeVisible({ timeout: 20_000 });
    });

    test("shows create user form with all fields", async ({ page }) => {
      await expect(page.locator("text=Create New User")).toBeVisible();
      await expect(page.locator('input[placeholder="username"]')).toBeVisible();
      await expect(page.locator('input[placeholder="Display Name"]')).toBeVisible();
      await expect(page.locator('input[placeholder="Password"]')).toBeVisible();
      await expect(page.locator("text=Admin privileges")).toBeVisible();
    });

    test("shows validation error for empty fields", async ({ page }) => {
      await page.getByRole("button", { name: "Create User" }).click();
      await expect(
        page.locator("text=Username and password are required")
      ).toBeVisible({ timeout: 5_000 });
    });

    test("can create a new user and see them in the list", async ({ page }) => {
      const testUser = `e2euser_${Date.now()}`;

      await page.locator('input[placeholder="username"]').fill(testUser);
      await page.locator('input[placeholder="Display Name"]').fill("E2E Test User");
      await page.locator('input[placeholder="Password"]').fill("TestPass123!");
      await page.getByRole("button", { name: "Create User" }).click();

      // Should redirect to users list or show success
      await expect(page).toHaveURL(/\/users/, { timeout: 15_000 });

      // The new user should appear in the list
      await expect(page.locator("table")).toBeVisible({ timeout: 20_000 });
      // Search for the user
      await page.locator('input[placeholder="Search users..."]').fill(testUser);
      await expect(
        page.locator("table tbody tr").filter({ hasText: testUser }).first()
      ).toBeVisible({ timeout: 10_000 });
    });

    test("Cancel navigates back to user list", async ({ page }) => {
      await page.getByRole("button", { name: "Cancel" }).click();
      await expect(page).toHaveURL(/\/users$/);
    });
  });

  test.describe("User Detail", () => {
    test("can click on a user to view details", async ({ page }) => {
      await page.goto("/users");
      await expect(page.locator("table")).toBeVisible({ timeout: 20_000 });

      const firstRow = page.locator("table tbody tr").first();
      const userLink = firstRow.getByRole("link").first();
      const href = await userLink.getAttribute("href");
      await firstRow.getByRole("button", { name: "View" }).click();

      expect(href).toBeTruthy();
      await expect(page).toHaveURL(new RegExp(href!.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")));
    });
  });
});
