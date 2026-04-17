import { expect, Page } from "@playwright/test";
import { allowPasionConsentIfPresent } from "./pasion-browser";
import { PADMIN_URL } from "./services";

/**
 * Log into padmin via the browser SSO flow. Assumes the user is already
 * signed into Pasion within the same browser context.
 */
export async function loginPadmin(page: Page): Promise<void> {
  await page.goto(`${PADMIN_URL}/`);
  const signIn = page.getByRole("button", { name: "Sign In" });
  await signIn.waitFor({ state: "visible", timeout: 15_000 });
  await signIn.click();
  await allowPasionConsentIfPresent(page);
  await expect(page.locator("main")).toContainText("Dashboard", { timeout: 30_000 });
  await expect(page).toHaveURL(new RegExp(`${PADMIN_URL.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}/?`));
}

/**
 * Prefer client-side navigation so the app keeps its in-memory auth state.
 */
export async function gotoPadminPath(page: Page, path: string): Promise<void> {
  if (new URL(page.url()).pathname === path) {
    return;
  }

  const link = page.locator(`a[href="${path}"]`).first();
  if (await link.isVisible({ timeout: 3_000 }).catch(() => false)) {
    await link.click();
    await page.waitForURL(`${PADMIN_URL}${path}`, { timeout: 15_000 });
    await page.waitForLoadState("networkidle").catch(() => {});
    return;
  }

  await page.goto(`${PADMIN_URL}${path}`);
  await page.waitForLoadState("networkidle").catch(() => {});
}

export async function createAppservice(
  page: Page,
  details: { id: string; sender: string; url: string }
): Promise<void> {
  await gotoPadminPath(page, "/appservices");
  await page.getByRole("button", { name: "Register" }).click();

  const customCard = page.locator("text=Custom Appservice").locator("..");
  await customCard.getByText("Install").click();

  await page.getByPlaceholder("wechat-bridge").fill(details.id);
  await page.getByPlaceholder("bot").fill(details.sender);
  await page.getByPlaceholder("http://localhost:29335").fill(details.url);

  for (const button of await page.getByRole("button", { name: "Generate random" }).all()) {
    await button.click();
  }

  await page.getByRole("button", { name: "Install", exact: true }).click();
  await expect(page.locator("main")).toContainText(details.id);
}
