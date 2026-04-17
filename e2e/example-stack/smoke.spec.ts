import { test, expect, Browser, BrowserContext, Page } from "@playwright/test";
import { createRoom, loginElement, sendMessage } from "../helpers/element";
import { upsertDevice } from "../helpers/matrix-admin";
import {
  createAppservice,
  gotoPadminPath,
  loginPadmin,
} from "../helpers/padmin-browser";
import {
  assertPasionProfile,
  loginPasionUser,
  registerPasionUser,
  type UserCreds,
} from "../helpers/pasion-browser";
import {
  getPalpoUserFlags,
  setPalpoUserGuest,
  waitForPalpoUser,
} from "../helpers/postgres";

const runId = Date.now().toString(36);

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

async function freshContext(
  browser: Browser
): Promise<{ context: BrowserContext; page: Page }> {
  const context = await browser.newContext();
  const page = await context.newPage();
  return { context, page };
}

async function prepareDelegatedAuthUser(username: string): Promise<void> {
  const userId = `@${username}:localhost`;
  await waitForPalpoUser(userId, 90_000);
  await setPalpoUserGuest(userId, false);
  await upsertDevice(username, `E2E_${username.toUpperCase()}`);
}

test.describe("Example stack smoke suite", () => {
  test.describe.configure({ mode: "serial" });
  test.setTimeout(180_000);

  test("first registered user becomes admin and can install an appservice", async ({
    browser,
  }) => {
    const { context, page } = await freshContext(browser);

    try {
      await registerPasionUser(page, alice);
      await assertPasionProfile(page, alice);

      await loginPadmin(page);
      await prepareDelegatedAuthUser(alice.username);

      const flags = await getPalpoUserFlags(`@${alice.username}:localhost`);
      expect(flags?.is_admin).toBe(true);

      await createAppservice(page, {
        id: `example-bot-${runId}`,
        sender: `examplebot${runId}`,
        url: "http://localhost:29999",
      });
    } finally {
      await context.close();
    }
  });

  test("first user can complete Element SSO and create a room", async ({ browser }) => {
    const { context, page } = await freshContext(browser);

    try {
      await loginPasionUser(page, alice);
      await loginElement(page);

      const roomId = await createRoom(page, `example-room-${runId}`, "Example stack smoke room");
      expect(roomId).toMatch(/^!/);
      await expect(page.locator("main")).toContainText(`example-room-${runId}`);
      await sendMessage(page, `hello from alice ${runId}`);
    } finally {
      await context.close();
    }
  });

  test("later users stay non-admin and padmin lists all seeded users", async ({
    browser,
  }) => {
    const bobSession = await freshContext(browser);

    try {
      await registerPasionUser(bobSession.page, bob);
      await assertPasionProfile(bobSession.page, bob);
      await loginPadmin(bobSession.page);
      await prepareDelegatedAuthUser(bob.username);

      const bobFlags = await getPalpoUserFlags(`@${bob.username}:localhost`);
      expect(bobFlags?.is_admin).toBe(false);

      const aliceFlags = await getPalpoUserFlags(`@${alice.username}:localhost`);
      expect(aliceFlags?.is_admin).toBe(true);
    } finally {
      await bobSession.context.close();
    }

    const carolSession = await freshContext(browser);

    try {
      await registerPasionUser(carolSession.page, carol);
      await assertPasionProfile(carolSession.page, carol);
      await prepareDelegatedAuthUser(carol.username);
    } finally {
      await carolSession.context.close();
    }

    const adminSession = await freshContext(browser);

    try {
      await loginPasionUser(adminSession.page, alice);
      await loginPadmin(adminSession.page);
      await gotoPadminPath(adminSession.page, "/users");

      const main = adminSession.page.locator("main");
      await expect(main).toContainText(`@${alice.username}:localhost`);
      await expect(main).toContainText(`@${bob.username}:localhost`);
      await expect(main).toContainText(`@${carol.username}:localhost`);
    } finally {
      await adminSession.context.close();
    }
  });
});
