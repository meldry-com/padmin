import { test, expect } from "@playwright/test";
import { readAdminMeta } from "./helpers/admin-meta";
import {
  acquireOidcAccessToken,
  buildMatrixScopes,
  newDeviceId,
} from "./helpers/oidc-device";
import { PALPO_URL, PADMIN_URL } from "./helpers/services";

test.describe("Sidebar Navigation", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await expect(page.locator("aside")).toBeVisible({ timeout: 20_000 });
  });

  test("shows all navigation items with branding", async ({ page }) => {
    const sidebar = page.locator("aside");
    await expect(sidebar.locator("text=Palpo Admin")).toBeVisible();
    await expect(sidebar.locator("text=Management")).toBeVisible();

    const navItems = ["Dashboard", "Users", "Rooms", "Media", "Reports", "Federation", "Registration Tokens"];
    for (const item of navItems) {
      await expect(sidebar.locator("button", { hasText: item })).toBeVisible();
    }

    await expect(sidebar.locator("button", { hasText: "Logout" })).toBeVisible();
  });

  test("navigates to each section and highlights active item", async ({ page }) => {
    const routes: Array<{ name: string; url: RegExp }> = [
      { name: "Users", url: /\/users/ },
      { name: "Rooms", url: /\/rooms/ },
      { name: "Media", url: /\/media/ },
      { name: "Reports", url: /\/reports/ },
      { name: "Federation", url: /\/destinations/ },
      { name: "Registration Tokens", url: /\/registration-tokens/ },
      { name: "Dashboard", url: /\/$/ },
    ];

    for (const route of routes) {
      const btn = page.locator("aside").locator("button", { hasText: route.name });
      await btn.click();
      await expect(page).toHaveURL(route.url, { timeout: 10_000 });
      // Active item should have highlight class
      await expect(btn).toHaveClass(/bg-sidebar-accent/);
    }
  });

  test("logout clears auth and redirects to login", async ({ browser, request }) => {
    const admin = await readAdminMeta();
    const freshToken = await acquireOidcAccessToken(request, {
      username: admin.username,
      password: admin.password,
      scopes: buildMatrixScopes(newDeviceId("PWSIDE"), ["urn:palpo:admin:*"]),
      clientName: "Playwright Sidebar Logout",
      timeoutMs: 120_000,
    });
    const context = await browser.newContext({
      storageState: { cookies: [], origins: [] },
    });
    const page = await context.newPage();

    try {
      // Inject auth state directly since the login page uses OAuth redirect
      await page.goto(`${PADMIN_URL}/login`);
      await expect(page.getByRole("heading", { name: "Palpo Admin" })).toBeVisible({ timeout: 20_000 });
      await page.evaluate(
        ({ token, userId }) => {
          localStorage.setItem("access_token", JSON.stringify(token));
          localStorage.setItem("user_id", JSON.stringify(userId));
          localStorage.setItem("home_server", JSON.stringify("localhost"));
        },
        { token: freshToken.accessToken, userId: freshToken.userId }
      );
      await page.goto(`${PADMIN_URL}/`);
      await expect(page.locator("aside")).toBeVisible({ timeout: 20_000 });

      await page.locator("aside").locator("button", { hasText: "Logout" }).click();
      await expect(
        page.getByRole("heading", { name: "Palpo Admin" })
      ).toBeVisible({ timeout: 20_000 });
      await expect(page).toHaveURL(/\/login/);

      const hasToken = await page.evaluate(() => localStorage.getItem("access_token"));
      expect(hasToken).toBeNull();
    } finally {
      await context.close();
    }
  });
});

test.describe("Sidebar Navigation on Mobile", () => {
  test.use({ viewport: { width: 390, height: 844 } });

  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await expect(page.locator("h1")).toContainText("Dashboard", { timeout: 20_000 });
  });

  test("opens as a drawer and closes after navigation", async ({ page }) => {
    const toggle = page.locator("header > button").first();
    const drawer = page.locator("aside");
    const backdrop = page.locator(".sidebar-backdrop");

    await expect(drawer).not.toHaveClass(/sidebar-open/);
    await expect(backdrop).not.toHaveClass(/sidebar-backdrop-open/);

    await toggle.click();

    await expect(drawer).toHaveClass(/sidebar-open/);
    await expect(backdrop).toHaveClass(/sidebar-backdrop-open/);

    await drawer.getByRole("button", { name: "Users" }).click();

    await expect(page).toHaveURL(/\/users/, { timeout: 10_000 });
    await expect(drawer).not.toHaveClass(/sidebar-open/);
    await expect(backdrop).not.toHaveClass(/sidebar-backdrop-open/);
  });

  test("backdrop dismisses the drawer", async ({ page }) => {
    const toggle = page.locator("header > button").first();
    const drawer = page.locator("aside");
    const backdrop = page.locator(".sidebar-backdrop");

    await toggle.click();
    await expect(drawer).toHaveClass(/sidebar-open/);

    await backdrop.click();

    await expect(drawer).not.toHaveClass(/sidebar-open/);
    await expect(backdrop).not.toHaveClass(/sidebar-backdrop-open/);
  });
});
