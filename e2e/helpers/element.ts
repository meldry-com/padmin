import { expect, Page } from "@playwright/test";
import { allowPasionConsentIfPresent } from "./pasion-browser";
import { ELEMENT_URL } from "./services";

export async function loginElement(page: Page): Promise<void> {
  await page.goto(`${ELEMENT_URL}/#/login`);
  await page.getByRole("button", { name: "Continue" }).click();
  const deadline = Date.now() + 45_000;
  const userMenu = page.getByRole("button", { name: "User menu" });

  while (Date.now() < deadline) {
    await allowPasionConsentIfPresent(page);

    const skipVerification = page.getByLabel("Skip verification for now");
    if (await skipVerification.isVisible().catch(() => false)) {
      await skipVerification.click();
    }

    const verifyLater = page.getByRole("button", { name: /I'll verify later/i });
    if (await verifyLater.isVisible().catch(() => false)) {
      await verifyLater.click();
    }

    if (await userMenu.isVisible().catch(() => false)) {
      return;
    }

    await page.waitForTimeout(500);
  }

  await expect(userMenu).toBeVisible({ timeout: 5_000 });
}

export async function createRoom(
  page: Page,
  name: string,
  topic: string = ""
): Promise<string> {
  await page.goto(`${ELEMENT_URL}/#/home`);
  await page.locator('div[role="button"]', { hasText: "Create a Group Chat" }).click();

  const dialog = page.getByRole("dialog");
  await dialog.getByPlaceholder("Name").fill(name);
  if (topic) {
    await dialog.getByPlaceholder("Topic (optional)").fill(topic);
  }
  await dialog.getByRole("button", { name: "Create room" }).click();

  await page.waitForURL(/#\/room\/!/, { timeout: 30_000 });
  const hash = new URL(page.url()).hash;
  const match = hash.match(/#\/room\/(![^?]+)/);
  if (!match) {
    throw new Error(`Could not parse room id from ${page.url()}`);
  }
  return match[1];
}

export async function sendMessage(page: Page, text: string): Promise<void> {
  const composer = page.locator('div[role="textbox"][contenteditable="true"]').first();
  await composer.waitFor({ state: "visible" });
  await composer.click();
  await composer.fill(text);
  await page.keyboard.press("Enter");
  await expect(page.locator(".mx_EventTile_body", { hasText: text }).first()).toBeVisible({
    timeout: 15_000,
  });
}
