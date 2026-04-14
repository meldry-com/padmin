import { test, expect } from "@playwright/test";
import { PALPO_URL, PASION_URL } from "./helpers/services";

// Login tests run without stored auth state (fresh browser)
test.use({ storageState: { cookies: [], origins: [] } });

test.describe("Login Page", () => {
  test("shows login page with sign-in button", async ({ page }) => {
    await page.goto("/login");
    await expect(
      page.getByRole("heading", { name: "Palpo Admin" })
    ).toBeVisible({ timeout: 20_000 });
    await expect(page.getByRole("button", { name: "Sign In" })).toBeVisible();
  });

  test("sign-in button redirects to Pasion authorize", async ({ page }) => {
    await page.goto("/login");
    await expect(
      page.getByRole("heading", { name: "Palpo Admin" })
    ).toBeVisible({ timeout: 20_000 });

    // Click sign in and verify redirect to Pasion
    await page.getByRole("button", { name: "Sign In" }).click();

    // Should redirect to Pasion's login page (via /authorize)
    await expect(page).toHaveURL(/login/, { timeout: 20_000 });
    // Pasion login form should appear
    await expect(
      page
        .locator('input[placeholder="Username or email"]')
        .or(page.locator('input[type="text"]').first())
    ).toBeVisible({ timeout: 20_000 });
  });

  test("delegated-auth homeserver advertises SSO-only login", async ({
    request,
  }) => {
    const resp = await request.get(`${PALPO_URL}/_matrix/client/v3/login`);
    expect(resp.ok()).toBe(true);
    const body = await resp.json();
    expect(body.flows).toEqual([{ type: "m.login.sso" }]);
  });

  test("redirects unauthenticated users to login", async ({ page }) => {
    await page.goto("/users");
    await expect(
      page.getByRole("heading", { name: "Palpo Admin" })
    ).toBeVisible({ timeout: 20_000 });
    await expect(page).toHaveURL(/\/login/);
  });
});
