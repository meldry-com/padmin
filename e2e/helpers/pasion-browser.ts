import { expect, Page } from "@playwright/test";
import { createPasionEmailCode } from "./postgres";
import { PASION_URL } from "./services";

export interface UserCreds {
  username: string;
  email: string;
  password: string;
  displayName: string;
}

/**
 * Pasion's queue worker can restart under load in the example stack. Retry
 * navigations when the browser sees a short-lived connection failure.
 */
async function gotoWithRetry(page: Page, url: string, attempts: number = 5): Promise<void> {
  for (let attempt = 0; attempt < attempts; attempt++) {
    try {
      await page.goto(url, { timeout: 20_000 });
      return;
    } catch (error) {
      const message = String(error);
      const transient =
        /ERR_EMPTY_RESPONSE|ERR_CONNECTION_REFUSED|ERR_CONNECTION_RESET|net::ERR_/i.test(
          message
        );

      if (!transient || attempt === attempts - 1) {
        throw error;
      }

      await page.waitForTimeout(2_000);
    }
  }
}

export async function registerPasionUser(page: Page, user: UserCreds): Promise<void> {
  for (let attempt = 0; attempt < 3; attempt++) {
    await gotoWithRetry(page, `${PASION_URL}/register`);
    await page.getByPlaceholder("Choose a username").fill(user.username);
    await page.getByPlaceholder("your@email.com").fill(user.email);
    const passwords = page.locator('input[type="password"]');
    await passwords.nth(0).fill(user.password);
    await passwords.nth(1).fill(user.password);
    await page.getByRole("button", { name: "Create account" }).click();

    try {
      await page.waitForURL(/\/register\/steps\/.+\/verify-email/, { timeout: 20_000 });
      break;
    } catch (error) {
      const bodyText = (await page.locator("body").textContent()) || "";

      // Playwright retries rerun the same test without resetting the stack.
      // If the user was already created by an earlier attempt, continue by
      // signing into the existing account instead of failing on duplicates.
      if (bodyText.includes("username_exists") || bodyText.includes("email_in_use")) {
        await loginPasionUser(page, user);
        return;
      }

      const transient = bodyText.includes("rate_limited");
      if (!transient || attempt === 2) {
        throw error;
      }

      await page.waitForTimeout(2_000 * (attempt + 1));
    }
  }

  const codeInput = page.getByPlaceholder("6-digit code");
  await codeInput.waitFor({ state: "visible", timeout: 15_000 });
  const code = await createPasionEmailCode(user.email);
  await codeInput.fill(code);
  await Promise.all([
    page.waitForURL((url) => !/verify-email/.test(url.pathname), { timeout: 30_000 }),
    codeInput.press("Enter"),
  ]);

  if (/\/display-name/.test(new URL(page.url()).pathname)) {
    await page.getByPlaceholder("Your name").fill(user.displayName);
    await Promise.all([
      page.waitForURL(`${PASION_URL}/`, { timeout: 20_000 }),
      page.getByRole("button", { name: "Continue" }).click(),
    ]);
  }

  await expect(page).toHaveURL(`${PASION_URL}/`);
}

export async function loginPasionUser(
  page: Page,
  user: Pick<UserCreds, "username" | "password">
): Promise<void> {
  await gotoWithRetry(page, `${PASION_URL}/login`);
  await page.getByPlaceholder("Username or email").fill(user.username);
  await page.getByPlaceholder("Password").fill(user.password);
  await page.locator('button[type="submit"]').click();
  await expect(page).toHaveURL(`${PASION_URL}/`);
}

export async function assertPasionProfile(page: Page, user: UserCreds): Promise<void> {
  const body = page.locator("body");
  await expect(body).toContainText(user.displayName);
  await expect(body).toContainText(`@${user.username}:localhost`);
  await expect(body).toContainText("Password is set");
}

export async function allowPasionConsentIfPresent(page: Page): Promise<void> {
  const allow = page.getByRole("button", { name: "Allow" });
  if (await allow.isVisible({ timeout: 5_000 }).catch(() => false)) {
    await allow.click();
  }
}
