import { expect, Page } from "@playwright/test";
import { getLatestEmailCode } from "./db";

export interface UserCreds {
  username: string;
  email: string;
  password: string;
  displayName: string;
}

/**
 * Pasion's queue worker crashes intermittently (see
 * examples/e2e/README.md). Retry a navigation a couple of times so short
 * restarts don't fail an entire test. Only retries ERR_EMPTY_RESPONSE /
 * ERR_CONNECTION_REFUSED — any other error is a real failure.
 */
async function gotoWithRetry(page: Page, url: string, attempts = 5): Promise<void> {
  for (let i = 0; i < attempts; i++) {
    try {
      await page.goto(url, { timeout: 20_000 });
      return;
    } catch (e) {
      const msg = String(e);
      const transient =
        /ERR_EMPTY_RESPONSE|ERR_CONNECTION_REFUSED|ERR_CONNECTION_RESET|net::ERR_/.test(msg);
      if (!transient || i === attempts - 1) throw e;
      await new Promise((r) => setTimeout(r, 2_000));
    }
  }
}

/**
 * Full pasion password registration flow: username/email/passwords ->
 * email verification (code pulled from postgres) -> display name -> home.
 * Leaves the page signed-in on pasion's overview.
 *
 * Uses named role selectors and `waitForURL` after each step so the helper
 * is robust against Pasion's multi-step SPA navigation.
 */
export async function registerPasionUser(page: Page, user: UserCreds): Promise<void> {
  await gotoWithRetry(page, "http://localhost:8090/register");
  await page.getByPlaceholder("Choose a username").fill(user.username);
  await page.getByPlaceholder("your@email.com").fill(user.email);
  const passwords = page.locator('input[type="password"]');
  await passwords.nth(0).fill(user.password);
  await passwords.nth(1).fill(user.password);
  await Promise.all([
    page.waitForURL(/\/register\/steps\/.+\/verify-email/, { timeout: 20_000 }),
    page.getByRole("button", { name: "Create account" }).click(),
  ]);

  // Verify email: code is stored in user_email_authentication_codes.
  const codeInput = page.getByPlaceholder("6-digit code");
  await codeInput.waitFor({ state: "visible", timeout: 15_000 });
  const code = await getLatestEmailCode(user.email);
  await codeInput.fill(code);
  // Submit via Enter so we dispatch the form's submit event directly; a
  // bare button click has occasionally raced the React state update.
  await Promise.all([
    page.waitForURL((u) => !/verify-email/.test(u.pathname), { timeout: 30_000 }),
    codeInput.press("Enter"),
  ]);

  // Display name step: shown when the registration flow isn't configured to
  // skip it. Dismiss with the "Continue" button.
  if (/\/display-name/.test(new URL(page.url()).pathname)) {
    await page.getByPlaceholder("Your name").fill(user.displayName);
    await Promise.all([
      page.waitForURL("http://localhost:8090/", { timeout: 20_000 }),
      page.getByRole("button", { name: "Continue" }).click(),
    ]);
  }

  await expect(page).toHaveURL("http://localhost:8090/");
}

/** Password login for an already-registered pasion user. */
export async function loginPasionUser(page: Page, user: Pick<UserCreds, "username" | "password">): Promise<void> {
  await gotoWithRetry(page, "http://localhost:8090/login");
  await page.getByPlaceholder("Username or email").fill(user.username);
  await page.getByPlaceholder("Password").fill(user.password);
  await page.locator('button[type="submit"]').click();
  await expect(page).toHaveURL("http://localhost:8090/");
}

/**
 * Assert the pasion overview shows the expected profile info. Call while
 * already on http://localhost:8090/.
 */
export async function assertPasionProfile(page: Page, user: UserCreds): Promise<void> {
  const body = page.locator("body");
  await expect(body).toContainText(user.displayName);
  await expect(body).toContainText(`@${user.username}:localhost`);
  await expect(body).toContainText("Password is set");
}

/** Click "Allow" on a pasion OAuth consent screen if it is currently shown. */
export async function allowPasionConsentIfPresent(page: Page): Promise<void> {
  const allow = page.getByRole("button", { name: "Allow" });
  if (await allow.isVisible({ timeout: 5_000 }).catch(() => false)) {
    await allow.click();
  }
}
