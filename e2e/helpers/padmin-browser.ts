import { expect, Page } from "@playwright/test";
import { allowPasionConsentIfPresent } from "./pasion-browser";
import { PADMIN_URL } from "./services";

const SIDEBAR_BUTTON_LABELS: Record<string, string> = {
  "/": "Dashboard",
  "/appservices": "Appservices",
  "/users": "Users",
};

/**
 * Log into padmin via the browser SSO flow. Assumes the user is already
 * signed into Pasion within the same browser context.
 */
export async function loginPadmin(page: Page): Promise<void> {
  await page.goto(`${PADMIN_URL}/`);
  const signIn = page.getByRole("button", { name: "Sign In" });
  await signIn.waitFor({ state: "visible", timeout: 15_000 });
  await signIn.click();

  const escapedBaseUrl = PADMIN_URL.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const deadline = Date.now() + 45_000;

  while (Date.now() < deadline) {
    await allowPasionConsentIfPresent(page);

    const main = page.locator("main");
    if (await main.isVisible({ timeout: 1_000 }).catch(() => false)) {
      const text = (await main.textContent()) || "";
      if (text.includes("Dashboard")) {
        await expect(page).toHaveURL(new RegExp(`${escapedBaseUrl}/?`));
        return;
      }
    }

    await page.waitForTimeout(500);
  }

  await expect(page.locator("main")).toContainText("Dashboard", { timeout: 5_000 });
}

/**
 * Prefer client-side navigation so the app keeps its in-memory auth state.
 */
export async function gotoPadminPath(page: Page, path: string): Promise<void> {
  if (new URL(page.url()).pathname === path) {
    return;
  }

  const sidebarLabel = SIDEBAR_BUTTON_LABELS[path];
  if (sidebarLabel) {
    const navButton = page.locator("aside").getByRole("button", { name: sidebarLabel, exact: true });
    if (await navButton.isVisible({ timeout: 3_000 }).catch(() => false)) {
      await navButton.click();
      await page.waitForURL(`${PADMIN_URL}${path}`, { timeout: 15_000 });
      await page.waitForLoadState("networkidle").catch(() => {});
      return;
    }
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
  await expect(page.locator("main")).toContainText("Install a new appservice", {
    timeout: 15_000,
  });
  await page.getByRole("button", { name: /Custom Appservice/i }).click();
  await expect(page.getByRole("heading", { name: /Install Custom Appservice/i })).toBeVisible({
    timeout: 15_000,
  });

  await page.getByPlaceholder("wechat-bridge").fill(details.id);
  await page.getByPlaceholder("bot").fill(details.sender);
  await page.getByPlaceholder("http://localhost:29335").fill(details.url);

  for (const button of await page.getByRole("button", { name: "Generate random" }).all()) {
    await button.click();
  }

  const install = page.getByRole("button", { name: "Install", exact: true }).last();
  await install.scrollIntoViewIfNeeded().catch(() => {});
  await install.click();
  await expect(page.locator("main")).toContainText(details.id);
}
