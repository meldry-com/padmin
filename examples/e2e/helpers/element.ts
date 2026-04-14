import { expect, Page } from "@playwright/test";
import { allowPasionConsentIfPresent } from "./pasion";

/**
 * Log into Element via SSO. Assumes the user is already signed in on pasion
 * (registerPasionUser/loginPasionUser ran in the same browser context).
 */
export async function loginElement(page: Page): Promise<void> {
  await page.goto("http://localhost:8080/#/login");
  // "Continue" on the login page kicks off the SSO round-trip.
  await page.getByRole("button", { name: "Continue" }).click();
  await allowPasionConsentIfPresent(page);
  // Element home page is reached once the room list sidebar loads.
  await page.waitForURL("**/#/home", { timeout: 30_000 });
  await expect(page.getByRole("button", { name: "User menu" })).toBeVisible({ timeout: 30_000 });
}

/**
 * Create an unencrypted-ish group chat room (Element defaults to E2EE).
 * Returns the room alias/ID extracted from the URL.
 */
export async function createRoom(page: Page, name: string, topic = ""): Promise<string> {
  // The "Create a Group Chat" card in the Home tile is the most stable entry.
  await page.goto("http://localhost:8080/#/home");
  await page.locator('div[role="button"]', { hasText: "Create a Group Chat" }).click();

  const dialog = page.getByRole("dialog");
  await dialog.getByPlaceholder("Name").fill(name);
  if (topic) await dialog.getByPlaceholder("Topic (optional)").fill(topic);
  await dialog.getByRole("button", { name: "Create room" }).click();

  // URL changes to #/room/!<id>:<server>
  await page.waitForURL(/#\/room\/!/, { timeout: 30_000 });
  const hash = new URL(page.url()).hash;
  const match = hash.match(/#\/room\/(![^?]+)/);
  if (!match) throw new Error(`Could not parse room id from ${page.url()}`);
  return match[1];
}

/** Send a plain text message in the currently-open room. */
export async function sendMessage(page: Page, text: string): Promise<void> {
  const composer = page.locator('div[role="textbox"][contenteditable="true"]').first();
  await composer.waitFor({ state: "visible" });
  await composer.click();
  await composer.fill(text);
  await page.keyboard.press("Enter");
  // Wait for the message tile to appear.
  await expect(page.locator(".mx_EventTile_body", { hasText: text }).first()).toBeVisible({ timeout: 15_000 });
}

/** Open an existing room by id (e.g. "!abcd:localhost"). */
export async function openRoom(page: Page, roomId: string): Promise<void> {
  await page.goto(`http://localhost:8080/#/room/${encodeURIComponent(roomId)}`);
  await page.waitForURL("**/#/room/**");
}

/**
 * Invite a user by Matrix ID into the currently-open room.
 * Uses the keyboard shortcut to open the People panel rather than chasing
 * the ever-changing right-panel DOM.
 */
export async function inviteUser(page: Page, mxid: string): Promise<void> {
  // Open the "Invite to this room" control from the room header/body.
  const inviteButton = page.getByRole("button", { name: /Invite to this room/i }).first();
  if (await inviteButton.isVisible({ timeout: 3_000 }).catch(() => false)) {
    await inviteButton.click();
  } else {
    // Fallback: use the menu item in the room info panel. Element exposes a
    // text button labeled "Invite" inside the member list.
    await page.getByRole("button", { name: /^Invite$/ }).first().click();
  }
  // Invite dialog: type the mxid, pick the suggestion, click Invite.
  const dialog = page.getByRole("dialog");
  const searchBox = dialog.getByRole("textbox").first();
  await searchBox.fill(mxid);
  // Wait for the suggestion row to become clickable, then click it.
  const suggestion = dialog.locator(`text=${mxid}`).first();
  await suggestion.waitFor({ state: "visible", timeout: 10_000 });
  await suggestion.click();
  await dialog.getByRole("button", { name: /^Invite$/ }).click();
  await expect(dialog).toBeHidden({ timeout: 10_000 });
}

/** Accept a pending invite for the currently-previewed room. */
export async function acceptInvite(page: Page): Promise<void> {
  const accept = page.getByRole("button", { name: /Accept/i }).first();
  await accept.waitFor({ state: "visible", timeout: 15_000 });
  await accept.click();
}

/**
 * Kick a user from the currently-open room via the member profile panel.
 */
export async function kickUser(page: Page, mxid: string): Promise<void> {
  // Open People tab (right panel).
  await page.getByRole("button", { name: /People/i }).first().click().catch(() => {});
  // Click the member row.
  await page.locator(`text=${mxid}`).first().click();
  // Click the Remove/Kick button in the member profile.
  const removeBtn = page.getByRole("button", { name: /^(Remove from room|Remove from chat|Kick)$/ }).first();
  await removeBtn.waitFor({ state: "visible", timeout: 10_000 });
  await removeBtn.click();
  // Confirmation dialog.
  const confirm = page.getByRole("dialog").getByRole("button", { name: /^(Remove|Kick)$/ }).first();
  await confirm.click();
}
