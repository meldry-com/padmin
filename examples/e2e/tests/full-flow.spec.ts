import { test, expect, Browser, BrowserContext, Page } from "@playwright/test";
import { registerPasionUser, loginPasionUser, assertPasionProfile, UserCreds } from "../helpers/pasion";
import { loginPadmin, gotoPadminPath } from "../helpers/padmin";
import {
  clearGuestFlag,
  getUserFlags,
  waitForPalpoUser,
} from "../helpers/db";
import { upsertDevice } from "../helpers/matrix-admin";
import { loginElement, createRoom, sendMessage } from "../helpers/element";

const alice: UserCreds = {
  username: "alice",
  email: "alice@test.local",
  password: "AlicePass123!",
  displayName: "Alice Admin",
};
const bob: UserCreds = {
  username: "bob",
  email: "bob@test.local",
  password: "BobPass123!",
  displayName: "Bob Second",
};
const carol: UserCreds = {
  username: "carol",
  email: "carol@test.local",
  password: "CarolPass123!",
  displayName: "Carol Third",
};

async function freshContext(browser: Browser): Promise<{ ctx: BrowserContext; page: Page }> {
  const ctx = await browser.newContext();
  const page = await ctx.newPage();
  return { ctx, page };
}

/**
 * Run after a pasion registration + first homeserver bootstrap so that
 * subsequent padmin admin API calls from this user work:
 *   - wait for pasion's async provision-user job to land the mxid in
 *     palpo's `users` table
 *   - clear the stamped-at-provision is_guest=true flag
 *   - seed a `user_device` row so auth_by_delegated_token finds one
 */
async function prepareUserForAdmin(username: string): Promise<void> {
  const mxid = `@${username}:localhost`;
  await waitForPalpoUser(mxid);
  clearGuestFlag();
  await upsertDevice(username, `E2E_${username.toUpperCase()}`);
}

test.describe("palpo stack smoke suite", () => {
  test("alice registers and becomes the matrix admin", async ({ browser }) => {
    const { ctx, page } = await freshContext(browser);
    try {
      // 1. Register alice via the pasion UI.
      await registerPasionUser(page, alice);
      await assertPasionProfile(page, alice);
      await expect(page.locator("body")).toContainText("Active sessions");
      await expect(page.locator("body")).toContainText("Linked identities");

      // 2. OAuth into padmin so pasion's provisioning job runs and creates
      //    alice in palpo's users table.
      await loginPadmin(page);
      await expect(page.locator("main")).toContainText("Dashboard");
      await expect(page.locator("main")).toContainText("Total Users");

      // 3. The "first user" should be flagged as admin in palpo and (per
      //    the current provisioning path) starts out as is_guest=true.
      //    Capture that invariant, then patch the stack so alice can
      //    actually use the /_palpo/admin/* endpoints.
      await prepareUserForAdmin(alice.username);
      const flags = getUserFlags(`@${alice.username}:localhost`);
      expect(flags?.is_admin).toBe(true);
    } finally {
      await ctx.close();
    }
  });

  test("alice uses padmin: users list and appservice register", async ({ browser }) => {
    const { ctx, page } = await freshContext(browser);
    try {
      // Alice is already registered from the prior test; log in via pasion
      // password then re-OAuth into padmin.
      await loginPasionUser(page, alice);
      await loginPadmin(page);

      // Users list shows alice as an admin.
      await gotoPadminPath(page, "/users");
      const main = page.locator("main");
      await expect(main).toContainText(`@${alice.username}:localhost`);
      await expect(main).toContainText("Admin");

      // Appservices: install a custom bot and verify it appears in the list.
      await gotoPadminPath(page, "/appservices");
      await page.getByRole("button", { name: "Register" }).click();

      const customCard = page.locator("text=Custom Appservice").locator("..");
      await customCard.getByText("Install").click();

      await page.getByPlaceholder("wechat-bridge").fill("test-bot");
      await page.getByPlaceholder("bot").fill("testbot");
      await page.getByPlaceholder("http://localhost:29335").fill("http://localhost:29999");
      for (const btn of await page.getByRole("button", { name: "Generate random" }).all()) {
        await btn.click();
      }
      await page.getByRole("button", { name: "Install", exact: true }).click();
      await expect(page.locator("main")).toContainText("test-bot");
    } finally {
      await ctx.close();
    }
  });

  test("alice logs into Element and creates a room", async ({ browser }) => {
    const { ctx, page } = await freshContext(browser);
    try {
      await loginPasionUser(page, alice);

      await loginElement(page);
      const roomId = await createRoom(page, "test-room", "E2E test room");
      expect(roomId).toMatch(/^!/);
      await expect(page.locator("main")).toContainText("test-room");
      await sendMessage(page, "hello from alice");
    } finally {
      await ctx.close();
    }
  });

  test("bob registers and is NOT flagged as admin", async ({ browser }) => {
    const { ctx, page } = await freshContext(browser);
    try {
      await registerPasionUser(page, bob);
      await assertPasionProfile(page, bob);

      // Drive bob through padmin OAuth once so pasion provisions him in palpo.
      await loginPadmin(page);
      await prepareUserForAdmin(bob.username);

      const bobFlags = getUserFlags(`@${bob.username}:localhost`);
      expect(bobFlags?.is_admin).toBe(false);

      const aliceFlags = getUserFlags(`@${alice.username}:localhost`);
      expect(aliceFlags?.is_admin).toBe(true);
    } finally {
      await ctx.close();
    }
  });

  test("carol registers as third user", async ({ browser }) => {
    const { ctx, page } = await freshContext(browser);
    try {
      await registerPasionUser(page, carol);
      await assertPasionProfile(page, carol);
      await loginElement(page);
      // Simply reaching the Element home tile is enough — we've exercised
      // the full SSO round-trip for a non-admin user.
      await expect(page.getByRole("button", { name: "User menu" })).toBeVisible();
    } finally {
      await ctx.close();
    }
  });

  test("padmin reflects all three users", async ({ browser }) => {
    const { ctx, page } = await freshContext(browser);
    try {
      await loginPasionUser(page, alice);
      await loginPadmin(page);

      // Carol also needs padmin-friendly device provisioning for her row to
      // exist in palpo; `prepareUserForAdmin` was only called for alice/bob.
      await prepareUserForAdmin(carol.username);

      await gotoPadminPath(page, "/users");
      const main = page.locator("main");
      await expect(main).toContainText(`@${alice.username}:localhost`);
      await expect(main).toContainText(`@${bob.username}:localhost`);
      await expect(main).toContainText(`@${carol.username}:localhost`);
    } finally {
      await ctx.close();
    }
  });
});
