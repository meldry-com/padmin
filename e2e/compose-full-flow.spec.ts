import { test, expect, type APIRequestContext, type Page } from "@playwright/test";
import { PADMIN_URL, PALPO_URL, PASION_URL } from "./helpers/services";
import { readAdminMeta } from "./helpers/admin-meta";
import {
  createPasionEmailCode,
  findLatestPasionUsernames,
  waitForPalpoUser,
} from "./helpers/postgres";
import {
  matrixCreateRoom,
  matrixJoinRoom,
  matrixSendMessage,
  syncUntilMessage,
} from "./helpers/matrix-admin";
import {
  pasionFinishRegistration,
  pasionRegister,
  pasionSetDisplayName,
  pasionVerifyEmail,
} from "./helpers/pasion-api";
import {
  acquireOidcAccessToken,
  buildMatrixScopes,
  newDeviceId,
} from "./helpers/oidc-device";

const ADMIN_STORAGE_STATE_PATH = "e2e/.auth/admin.json";

type TestUser = {
  username: string;
  password: string;
  email: string;
  displayName: string;
  userId: string;
};

const runId = Date.now().toString(36);
const USERNAME_PREFIX = "pwuser";
const PASSWORD_PREFIX = "UserPass123!";
const DISPLAY_PREFIX = "Playwright User";

function buildGeneratedUser(suffix: string): TestUser {
  const username = `${USERNAME_PREFIX}${suffix}`;
  return {
    username,
    password: `${PASSWORD_PREFIX}${suffix}`,
    email: `${username}@test.local`,
    displayName: `${DISPLAY_PREFIX} ${suffix}`,
    userId: `@${username}:localhost`,
  };
}

function buildGeneratedUserFromUsername(username: string): TestUser {
  const suffix = username.slice(USERNAME_PREFIX.length);
  return buildGeneratedUser(suffix);
}

async function findReusableGeneratedUsers(limit: number): Promise<TestUser[]> {
  const usernames = await findLatestPasionUsernames(USERNAME_PREFIX, limit);
  return usernames.map(buildGeneratedUserFromUsername);
}

const freshPrimaryUser = buildGeneratedUser(runId);
const freshSecondaryUser = buildGeneratedUser(`${runId}b`);
let activePrimaryUser = freshPrimaryUser;
let activeSecondaryUser = freshSecondaryUser;

const managedUser = `pwmanaged${runId}`;
const managedPassword = `ManagedPass123!${runId}`;
const managedDisplayName = `Managed User ${runId}`;
const managedUserId = `@${managedUser}:localhost`;

const roomName = `playwright-room-${runId}`;

async function ensurePasionUser(
  request: APIRequestContext,
  user: TestUser
): Promise<void> {
  const existingPalpoUser = await waitForPalpoUser(user.userId, 3_000).catch(() => null);
  if (existingPalpoUser) {
    return;
  }

  try {
    const registration = await pasionRegister(
      request,
      user.username,
      user.password,
      user.email
    );
    const code = await createPasionEmailCode(user.email);
    const verifyResult = await pasionVerifyEmail(request, registration.id, code);

    if (verifyResult.next_step === "display_name" || verifyResult.next_step === "finish") {
      await pasionSetDisplayName(request, registration.id, user.displayName);
    }

    await pasionFinishRegistration(request, registration.id);
  } catch (error) {
    const alreadyProvisioned = await waitForPalpoUser(user.userId, 30_000).catch(() => null);
    if (!alreadyProvisioned) {
      throw error;
    }
  }
  await waitForPalpoUser(user.userId, 90_000);
}

async function acquireUserToken(
  request: APIRequestContext,
  user: TestUser
): Promise<string> {
  const token = await acquireOidcAccessToken(request, {
    username: user.username,
    password: user.password,
    scopes: buildMatrixScopes(newDeviceId("PWCHAT")),
    clientName: `Playwright ${user.username}`,
    timeoutMs: 120_000,
  });

  return token.accessToken;
}

async function searchUserRow(page: Page, username: string) {
  await page.goto(`${PADMIN_URL}/users`);
  await expect(page.locator("table")).toBeVisible({ timeout: 20_000 });
  await page.locator('input[placeholder="Search users..."]').fill(username);
  return page.locator("table tbody tr").filter({ hasText: username }).first();
}

async function openAuthenticatedPadmin(page: Page, path: string = "/"): Promise<void> {
  await page.goto(`${PADMIN_URL}${path}`);
  await expect(page.locator("aside")).toBeVisible({ timeout: 20_000 });
}

async function loginToPadminWithAccessToken(page: Page, accessToken: string, userId?: string): Promise<void> {
  await page.goto(`${PADMIN_URL}/login`);
  await expect(page.getByRole("heading", { name: "Palpo Admin" })).toBeVisible({
    timeout: 20_000,
  });

  // Inject auth state directly since the login page uses OAuth redirect
  await page.evaluate(
    ({ token, uid }) => {
      localStorage.setItem("access_token", JSON.stringify(token));
      if (uid) localStorage.setItem("user_id", JSON.stringify(uid));
      localStorage.setItem("home_server", JSON.stringify("localhost"));
    },
    { token: accessToken, uid: userId }
  );

  await page.goto(`${PADMIN_URL}/`);
  await expect(page.locator("aside")).toBeVisible({ timeout: 20_000 });
}

async function tryRegisterUserInBrowser(page: Page, user: TestUser): Promise<boolean> {
  await page.goto(`${PASION_URL}/register`);
  await expect(page.getByRole("heading", { name: "Create account" })).toBeVisible({
    timeout: 20_000,
  });

  await page.locator('input[placeholder="Choose a username"]').fill(user.username);
  await page.locator('input[placeholder="your@email.com"]').fill(user.email);
  const passwordInputs = page.locator('input[autocomplete="new-password"]');
  await passwordInputs.first().fill(user.password);
  await passwordInputs.nth(1).fill(user.password);
  await page.getByRole("button", { name: "Create account" }).click();

  try {
    await page.waitForURL(/\/register\/[^/]+\/verify-email/, { timeout: 15_000 });
  } catch (error) {
    const bodyText = (await page.locator("body").textContent()) || "";
    if (bodyText.includes("rate_limited")) {
      return false;
    }

    throw error;
  }

  const verificationCode = await createPasionEmailCode(user.email);
  await page.locator('input[placeholder="6-digit code"]').fill(verificationCode);
  await page.getByRole("button", { name: "Verify" }).click();

  await page.waitForURL(/\/register\/[^/]+\/display-name/, { timeout: 30_000 });
  await page.locator('input[placeholder="Your name"]').fill(user.displayName);
  await page.getByRole("button", { name: "Continue" }).click();

  await waitForPalpoUser(user.userId, 90_000);
  return true;
}

async function loginToPasionInBrowser(
  page: Page,
  user: TestUser,
  timeoutMs: number = 60_000
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  let retryDelayMs = 2_000;

  while (true) {
    await expect(page.getByRole("heading", { name: "Sign in" })).toBeVisible({
      timeout: 30_000,
    });
    await page.locator('input[placeholder="Username or email"]').fill(user.username);
    await page.locator('input[placeholder="Password"]').fill(user.password);
    await page.getByRole("button", { name: "Sign in" }).click();

    try {
      await expect(page.getByRole("heading", { name: "Account" })).toBeVisible({
        timeout: 5_000,
      });
      return;
    } catch (error) {
      const bodyText = (await page.locator("body").textContent()) || "";
      const shouldRetry =
        bodyText.includes("rate_limited") && Date.now() + retryDelayMs < deadline;

      if (shouldRetry) {
        await page.waitForTimeout(retryDelayMs);
        retryDelayMs = Math.min(retryDelayMs * 2, 10_000);
        continue;
      }

      throw error;
    }
  }
}

test.describe("Compose Stack Full Flow", () => {
  test.describe.configure({ mode: "serial" });
  test.setTimeout(240_000);

  test("Pasion browser session flow completes end to end", async ({
    browser,
  }) => {
    const context = await browser.newContext({
      storageState: { cookies: [], origins: [] },
    });
    const page = await context.newPage();

    try {
      const browserRegistrationSucceeded = await tryRegisterUserInBrowser(page, freshPrimaryUser);
      const reusableUsers = await findReusableGeneratedUsers(3);

      if (browserRegistrationSucceeded) {
        activePrimaryUser = freshPrimaryUser;
      } else {
        if (!reusableUsers[0]) {
          throw new Error("Pasion registration was rate-limited and no reusable browser user exists");
        }
        activePrimaryUser = reusableUsers[0];
        await page.goto(`${PASION_URL}/login`);
      }

      const distinctReusableSecondary = reusableUsers.find(
        (user) => user.userId !== activePrimaryUser.userId
      );
      if (distinctReusableSecondary) {
        activeSecondaryUser = distinctReusableSecondary;
      }

      if (!browserRegistrationSucceeded) {
        await loginToPasionInBrowser(page, activePrimaryUser);
      }

      await expect(page.getByRole("heading", { name: "Account" })).toBeVisible({
        timeout: 30_000,
      });
      await expect(page.getByText(activePrimaryUser.userId, { exact: true }).first()).toBeVisible();
      const logoutResponse = await page.evaluate(async () => {
        const response = await fetch("/api/v1/auth/logout", {
          method: "POST",
          credentials: "include",
        });

        return {
          status: response.status,
          body: await response.json().catch(() => null),
        };
      });
      expect(logoutResponse.status).toBe(200);
      expect(logoutResponse.body).toEqual({ status: "success" });

      await page.goto(`${PASION_URL}/login`);

      await loginToPasionInBrowser(page, activePrimaryUser);
      await expect(page.getByText(activePrimaryUser.userId, { exact: true }).first()).toBeVisible();
    } finally {
      await context.close();
    }
  });

  test("Padmin admin flow manages users, activation state, and registration tokens", async ({
    browser,
  }) => {
    const context = await browser.newContext({
      storageState: ADMIN_STORAGE_STATE_PATH,
    });
    const page = await context.newPage();

    try {
      await openAuthenticatedPadmin(page, "/users");

      const primaryRow = await searchUserRow(page, activePrimaryUser.username);
      await expect(primaryRow).toBeVisible({ timeout: 20_000 });
      await primaryRow.getByRole("button", { name: "View" }).click();
      await expect(page).toHaveURL(
        new RegExp(`/users/${encodeURIComponent(activePrimaryUser.userId)}`)
      );
      await expect(page.getByText(activePrimaryUser.userId, { exact: true }).first()).toBeVisible();

      await page.goto(`${PADMIN_URL}/users/create`);
      await expect(page.locator("text=Create New User")).toBeVisible();
      await page.locator('input[placeholder="username"]').fill(managedUser);
      await page.locator('input[placeholder="Display Name"]').fill(managedDisplayName);
      await page.locator('input[placeholder="Password"]').fill(managedPassword);
      await page.getByRole("button", { name: "Create User" }).click();

      await expect(page).toHaveURL(/\/users$/);
      const managedRow = await searchUserRow(page, managedUser);
      await expect(managedRow).toBeVisible({ timeout: 20_000 });
      await managedRow.getByRole("button", { name: "View" }).click();
      await expect(page.getByText(managedUserId, { exact: true }).first()).toBeVisible();

      const rowToDeactivate = await searchUserRow(page, activePrimaryUser.username);
      await rowToDeactivate.getByRole("button", { name: "Deactivate" }).click();
      await expect(page.locator("text=User deactivated")).toBeVisible({ timeout: 10_000 });

      const rowToReactivate = await searchUserRow(page, activePrimaryUser.username);
      await expect(
        rowToReactivate.getByRole("button", { name: "Reactivate" })
      ).toBeVisible({ timeout: 20_000 });
      await rowToReactivate.getByRole("button", { name: "Reactivate" }).click();
      await expect(page.locator("text=User reactivated")).toBeVisible({ timeout: 10_000 });

      const reloadedPrimaryRow = await searchUserRow(page, activePrimaryUser.username);
      await expect(
        reloadedPrimaryRow.getByRole("button", { name: "Deactivate" })
      ).toBeVisible({ timeout: 20_000 });

      await page.goto(`${PADMIN_URL}/registration-tokens`);
      await page.getByRole("button", { name: "Create Token" }).click();
      const tokenCode = page.locator("code").first();
      await expect(tokenCode).toBeVisible({ timeout: 20_000 });
      const createdToken = (await tokenCode.textContent())?.trim() || "";
      expect(createdToken).not.toBe("");

      const tokenRow = page.locator("table tbody tr").filter({ hasText: createdToken }).first();
      await tokenRow.getByRole("button", { name: "Delete" }).click();
      await page.getByRole("button", { name: "Delete" }).last().click();
      await expect(page.locator("code").filter({ hasText: createdToken })).toHaveCount(0, {
        timeout: 20_000,
      });

    } finally {
      await context.close();
    }
  });

  test("Registered users can exchange messages and the room appears in padmin", async ({
    browser,
    request,
  }) => {
    const reusableUsers = await findReusableGeneratedUsers(4);
    const distinctReusableSecondary = reusableUsers.find(
      (user) => user.userId !== activePrimaryUser.userId
    );
    if (distinctReusableSecondary) {
      activeSecondaryUser = distinctReusableSecondary;
    }

    await waitForPalpoUser(activePrimaryUser.userId, 90_000);
    await ensurePasionUser(request, activeSecondaryUser);

    const firstMessage = `hello from ${activePrimaryUser.username} at ${runId}`;
    const secondMessage = `reply from ${activeSecondaryUser.username} at ${runId}`;
    const senderToken = await acquireUserToken(request, activePrimaryUser);
    const receiverToken = await acquireUserToken(request, activeSecondaryUser);

    const room = await matrixCreateRoom(request, senderToken, {
      name: roomName,
      topic: "Playwright compose stack verification",
      invite: [activeSecondaryUser.userId],
      isDirect: true,
    });

    await matrixJoinRoom(request, receiverToken, room.room_id);
    await matrixSendMessage(request, senderToken, room.room_id, firstMessage);
    await syncUntilMessage(request, receiverToken, room.room_id, firstMessage, 30_000);

    await matrixSendMessage(request, receiverToken, room.room_id, secondMessage);
    await syncUntilMessage(request, senderToken, room.room_id, secondMessage, 30_000);

    const context = await browser.newContext({
      storageState: ADMIN_STORAGE_STATE_PATH,
    });
    const page = await context.newPage();

    try {
      await openAuthenticatedPadmin(page, "/rooms");
      await expect(page.locator("table")).toBeVisible({ timeout: 20_000 });
      await page.locator('input[placeholder="Search rooms by name or alias..."]').fill(roomName);

      const roomRow = page.locator("table tbody tr").filter({ hasText: roomName }).first();
      await expect(roomRow).toBeVisible({ timeout: 30_000 });
      await roomRow.getByRole("link", { name: roomName }).click();

      await expect(page.locator(`text=${roomName}`)).toBeVisible({ timeout: 20_000 });
      await expect(page.getByText(room.room_id, { exact: true }).first()).toBeVisible();
      await expect(page.locator("text=Playwright compose stack verification")).toBeVisible({
        timeout: 20_000,
      });
      await expect(page.locator("text=Members").locator("..")).toContainText("2");
    } finally {
      await context.close();
    }
  });

  test("Padmin logout redirects back to login", async ({ browser, request }) => {
    const admin = await readAdminMeta();
    const freshAdminToken = await acquireOidcAccessToken(request, {
      username: admin.username,
      password: admin.password,
      scopes: buildMatrixScopes(newDeviceId("PWLGOUT"), ["urn:palpo:admin:*", "urn:pasion:admin"]),
      clientName: "Playwright Padmin Logout",
      timeoutMs: 120_000,
    });
    const context = await browser.newContext({
      storageState: { cookies: [], origins: [] },
    });
    const page = await context.newPage();

    try {
      await loginToPadminWithAccessToken(page, freshAdminToken.accessToken, freshAdminToken.userId);
      await page.locator("header").getByRole("button", { name: "Logout" }).click();
      await expect(page.getByRole("heading", { name: "Palpo Admin" })).toBeVisible({
        timeout: 20_000,
      });
      await expect(page).toHaveURL(/\/login/);
    } finally {
      await context.close();
    }
  });
});
