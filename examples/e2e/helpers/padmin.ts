import { expect, Page } from "@playwright/test";
import { allowPasionConsentIfPresent } from "./pasion";

/**
 * Log into padmin via pasion OAuth. Assumes the user is already signed in
 * on pasion (i.e. registerPasionUser or loginPasionUser was just called in
 * the same browser context).
 */
export async function loginPadmin(page: Page): Promise<void> {
  await page.goto("http://localhost:9090/");
  // Padmin's Dioxus/WASM bundle needs a moment to boot before rendering
  // the login button.
  const signIn = page.getByRole("button", { name: "Sign In" });
  await signIn.waitFor({ state: "visible", timeout: 15_000 });
  await signIn.click();

  // Lands on pasion consent (first time) then bounces back to /oauth/callback.
  await allowPasionConsentIfPresent(page);

  // Wait for the dashboard to render. "Dashboard" heading is our signal.
  await expect(page.locator("main")).toContainText("Dashboard", { timeout: 30_000 });
  await expect(page).toHaveURL(/localhost:9090\//);
}

/**
 * Navigate padmin to an arbitrary path using *client-side* routing: click
 * the matching anchor in the sidebar/layout instead of page.goto(). A
 * hard-reload goto can race with the refresh-token exchange and bounce you
 * to /login; staying in the WASM app avoids that entirely.
 *
 * Assumes loginPadmin() already ran so we're on / with the sidebar loaded.
 * Falls back to page.goto() if no matching link is found (e.g. a path that
 * lives outside the sidebar nav).
 */
export async function gotoPadminPath(page: Page, path: string): Promise<void> {
  // Same path already? Nothing to do.
  if (new URL(page.url()).pathname === path) return;

  const link = page.locator(`a[href="${path}"]`).first();
  if (await link.isVisible({ timeout: 3_000 }).catch(() => false)) {
    await link.click();
    // Dioxus router updates the URL synchronously after client-side nav.
    await page.waitForURL(`http://localhost:9090${path}`, { timeout: 15_000 });
    await page.waitForLoadState("networkidle").catch(() => {});
    return;
  }

  // Fallback: hard goto. This can land on /login if the refresh dance
  // hasn't happened yet; the caller may need to re-run loginPadmin.
  await page.goto(`http://localhost:9090${path}`);
  await page.waitForLoadState("networkidle").catch(() => {});
}

/** Install a "Custom Appservice" via the Register modal on the Appservices page. */
export async function createAppservice(
  page: Page,
  details: { id: string; sender: string; url: string }
): Promise<void> {
  await gotoPadminPath(page, "/appservices");
  await page.getByRole("button", { name: "Register" }).click();

  // The template gallery is a modal; click the "Install →" chip on the
  // "Custom Appservice" card.
  const customCard = page.locator("text=Custom Appservice").locator("..");
  await customCard.getByText("Install").click();

  await page.getByPlaceholder("wechat-bridge").fill(details.id);
  await page.getByPlaceholder("bot").fill(details.sender);
  await page.getByPlaceholder("http://localhost:29335").fill(details.url);

  // AS / HS token fields: use the "Generate random" helpers.
  for (const btn of await page.getByRole("button", { name: "Generate random" }).all()) {
    await btn.click();
  }
  await page.getByRole("button", { name: "Install", exact: true }).click();

  // Appears in the list.
  await expect(page.locator("main")).toContainText(details.id);
}
