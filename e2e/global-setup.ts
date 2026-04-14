import { test as setup, expect } from "@playwright/test";
import { waitForServices, PALPO_URL, PADMIN_URL } from "./helpers/services";
import { whoami } from "./helpers/matrix-admin";
import {
  createPasionEmailCode,
  findLatestPasionUsername,
  setPalpoUserAdmin,
  setPasionUserCanRequestAdmin,
  waitForPalpoUser,
  waitForPostgres,
} from "./helpers/postgres";
import {
  pasionFinishRegistration,
  pasionRegister,
  pasionSetDisplayName,
  pasionVerifyEmail,
} from "./helpers/pasion-api";
import { acquireOidcAccessToken, buildMatrixScopes, newDeviceId } from "./helpers/oidc-device";
import { type AdminMeta, writeAdminMeta } from "./helpers/admin-meta";

const ADMIN_DISPLAY_NAME = "Playwright Admin";
setup.setTimeout(240_000);

function buildAdminMeta(username?: string, password?: string): AdminMeta {
  const suffix = Date.now().toString(36);
  const resolvedUsername = username || `pwadmin${suffix}`;
  const resolvedPassword = password || `Admin123!${suffix}`;

  return {
    username: resolvedUsername,
    password: resolvedPassword,
    userId: `@${resolvedUsername}:localhost`,
    email: `${resolvedUsername}@test.local`,
    displayName: ADMIN_DISPLAY_NAME,
  };
}

async function findExistingGeneratedAdminMeta(): Promise<AdminMeta | null> {
  const username = await findLatestPasionUsername("pwadmin");
  if (!username) {
    return null;
  }

  const suffix = username.slice("pwadmin".length);
  if (!suffix) {
    return null;
  }

  return buildAdminMeta(username, `Admin123!${suffix}`);
}

async function bootstrapAdminUser(
  request: Parameters<typeof whoami>[0],
  meta: AdminMeta
): Promise<void> {
  const existingPalpoUser = await waitForPalpoUser(meta.userId, 3_000).catch(() => null);
  if (existingPalpoUser) {
    return;
  }

  try {
    const registration = await pasionRegister(
      request,
      meta.username,
      meta.password,
      meta.email
    );
    const code = await createPasionEmailCode(meta.email);

    const verifyResult = await pasionVerifyEmail(request, registration.id, code);
    if (verifyResult.next_step === "display_name" || verifyResult.next_step === "finish") {
      await pasionSetDisplayName(request, registration.id, meta.displayName);
    }

    await pasionFinishRegistration(request, registration.id);
  } catch (error) {
    const alreadyProvisioned = await waitForPalpoUser(meta.userId, 30_000).catch(() => null);
    if (!alreadyProvisioned) {
      throw error;
    }
  }
  await waitForPalpoUser(meta.userId, 90_000);
}

async function ensureAdminUser(
  request: Parameters<typeof whoami>[0]
): Promise<AdminMeta & { accessToken: string; userId: string }> {
  const hasEnvAdmin =
    Boolean(process.env.TEST_ADMIN_USERNAME) || Boolean(process.env.TEST_ADMIN_PASSWORD);
  const existingGenerated = await findExistingGeneratedAdminMeta();
  const preferred = buildAdminMeta(
    process.env.TEST_ADMIN_USERNAME,
    process.env.TEST_ADMIN_PASSWORD
  );
  const generatedFallback = buildAdminMeta();
  const orderedCandidates = hasEnvAdmin
    ? [preferred, existingGenerated, generatedFallback]
    : [existingGenerated, generatedFallback];
  const candidates = orderedCandidates.filter(
    (meta, index, all): meta is AdminMeta =>
      Boolean(meta) &&
      all.findIndex((candidate) => candidate?.username === meta.username) === index
  );
  let lastError: unknown;

  for (const meta of candidates) {
    try {
      console.log(`[global-setup] trying admin candidate ${meta.username}`);
      await bootstrapAdminUser(request, meta);
      await setPasionUserCanRequestAdmin(meta.username, true);
      await waitForPalpoUser(meta.userId, 90_000);
      await setPalpoUserAdmin(meta.userId, true);

      const adminToken = await acquireOidcAccessToken(request, {
        username: meta.username,
        password: meta.password,
        scopes: buildMatrixScopes(newDeviceId("PWADMIN"), ["urn:palpo:admin:*", "urn:pasion:admin"]),
        clientName: "Playwright Admin Setup",
        timeoutMs: 120_000,
      });

      console.log(`[global-setup] authenticated admin candidate ${meta.username}`);
      return {
        ...meta,
        accessToken: adminToken.accessToken,
        userId: adminToken.userId,
      };
    } catch (error) {
      console.error(
        `[global-setup] admin candidate failed ${meta.username}: ${String(error)}`
      );
      lastError = error;
    }
  }

  throw new Error(`Unable to provision admin user: ${String(lastError)}`);
}

setup("verify services and authenticate", async ({ page, request }) => {
  await waitForServices(request);
  await waitForPostgres("pasion");
  await waitForPostgres("palpo");

  const admin = await ensureAdminUser(request);
  await writeAdminMeta({
    username: admin.username,
    password: admin.password,
    userId: admin.userId,
    email: admin.email,
    displayName: admin.displayName,
  });

  const me = await whoami(request, admin.accessToken);
  expect(me, "Access token should be valid").not.toBeNull();
  expect(me!.user_id).toBe(admin.userId);

  await page.goto(`${PADMIN_URL}/login`);
  await expect(
    page.getByRole("heading", { name: "Palpo Admin" })
  ).toBeVisible({ timeout: 20_000 });

  await page.evaluate(
    ({ baseUrl, token, userId, padminUrl }) => {
      localStorage.setItem("base_url", JSON.stringify(baseUrl));
      localStorage.setItem("access_token", JSON.stringify(token));
      localStorage.setItem("user_id", JSON.stringify(userId));
      localStorage.setItem("home_server", JSON.stringify("localhost"));
      localStorage.setItem("login_type", JSON.stringify("accessToken"));
      // Use padmin URL as pasion_url so requests go through nginx proxy
      localStorage.setItem("pasion_url", JSON.stringify(padminUrl));
    },
    { baseUrl: PALPO_URL, token: admin.accessToken, userId: admin.userId, padminUrl: PADMIN_URL }
  );

  await page.goto(`${PADMIN_URL}/`);
  await expect(page.locator("aside")).toBeVisible({ timeout: 20_000 });
  await expect(page.locator("h1")).toContainText("Dashboard");

  await page.context().storageState({ path: "e2e/.auth/admin.json" });
});
