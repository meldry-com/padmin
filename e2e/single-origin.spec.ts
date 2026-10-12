import { test, expect } from "@playwright/test";

const adminPath = (process.env.PADMIN_MOUNT_PATH ?? "/admin").replace(/\/$/, "");
const authPath = (process.env.PASION_MOUNT_PATH ?? "/auth").replace(/\/$/, "");

test.skip(!process.env.PADMIN_MOUNT_TEST_URL, "Requires an isolated /admin + /auth stack");

test.beforeEach(async ({ page }) => {
  // Mount coverage should not depend on the availability of external fonts.
  await page.route(/^https:\/\/fonts\.(googleapis|gstatic)\.com\//, route => route.abort());
});

test("mounted dashboard loads JS/WASM and refreshes deep links", async ({ page, request }) => {
  const failed: string[] = [];
  const errors: string[] = [];
  page.on("response", response => {
    if (new URL(response.url()).pathname.startsWith(`${adminPath}/assets/`) && response.status() >= 400) {
      failed.push(`${response.status()} ${response.url()}`);
    }
  });
  page.on("pageerror", error => errors.push(error.message));
  await page.goto(`${adminPath}/login`, { waitUntil: "domcontentloaded" });
  await expect(page.getByRole("heading", { name: "Palpo Admin" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Sign In", exact: true })).toBeEnabled();
  const resources = await page.evaluate(() => performance.getEntriesByType("resource").map(e => e.name));
  expect(resources.some(url => url.includes(`${adminPath}/assets/`) && url.endsWith(".wasm"))).toBeTruthy();
  if (adminPath) expect(resources.some(url => new URL(url).pathname.startsWith("/assets/"))).toBeFalsy();
  expect(resources.some(url => url.endsWith(`${adminPath}/config.json`))).toBeTruthy();
  await page.goto(`${adminPath}/users`, { waitUntil: "domcontentloaded" });
  await expect(page).toHaveURL(new RegExp(`${adminPath}/login$`));
  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.getByRole("heading", { name: "Palpo Admin" })).toBeVisible();
  expect(failed).toEqual([]);
  expect(errors).toEqual([]);
  expect((await request.get(`${adminPath}/assets/missing.wasm`)).status()).toBe(404);
  if (adminPath) {
    const redirect = await request.get(`${adminPath}?test=1`, { maxRedirects: 0 });
    expect(redirect.status()).toBe(308);
    expect(redirect.headers().location).toContain(`${adminPath}/?test=1`);
  }
});

test("OAuth uses the auth mount and admin callback", async ({ page }) => {
  await page.goto(`${adminPath}/login`, { waitUntil: "domcontentloaded" });
  const authorization = page.waitForRequest(request => new URL(request.url()).pathname === `${authPath}/authorize`);
  await page.getByRole("button", { name: "Sign In", exact: true }).click();
  const url = new URL((await authorization).url());
  expect(url.searchParams.get("redirect_uri")).toBe(`${new URL(process.env.PADMIN_MOUNT_TEST_URL!).origin}${adminPath}/oauth/callback`);
  expect(url.searchParams.get("code_challenge_method")).toBe("S256");
});

test("Pasion assets and token exchange retain their mount", async ({ page }) => {
  test.skip(!process.env.PADMIN_MOUNT_USERNAME || !process.env.PADMIN_MOUNT_PASSWORD, "Requires a fixture admin account");
  const assetFailures: string[] = [];
  page.on("response", response => {
    if (new URL(response.url()).pathname.startsWith(`${authPath}/assets/`) && response.status() >= 400) {
      assetFailures.push(response.url());
    }
  });
  await page.goto(`${adminPath}/login`, { waitUntil: "domcontentloaded" });
  await page.getByRole("button", { name: "Sign In", exact: true }).click();
  await page.getByPlaceholder("Username or email").fill(process.env.PADMIN_MOUNT_USERNAME!);
  await page.getByPlaceholder("Password", { exact: true }).fill(process.env.PADMIN_MOUNT_PASSWORD!);
  const tokenResponse = page.waitForResponse(response => new URL(response.url()).pathname.endsWith("/oauth2/token"));
  const adminProbe = page.waitForResponse(response => new URL(response.url()).pathname === `${authPath}/api/admin/v1/version`);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await page.waitForURL(url => url.pathname.startsWith(`${authPath}/consent/`) || url.pathname.startsWith(`${adminPath}/oauth/callback`) || url.pathname === `${adminPath}/`);
  if (new URL(page.url()).pathname.startsWith(`${authPath}/consent/`)) {
    await page.getByRole("button", { name: "Allow", exact: true }).click();
  }
  const token = await tokenResponse;
  expect(new URL(token.url()).pathname).toBe(`${authPath}/oauth2/token`);
  expect(token.status()).toBe(200);
  await expect(page).toHaveURL(`${new URL(token.url()).origin}${adminPath}/`);
  expect((await adminProbe).status()).toBe(200);
  await expect(page.getByText("Welcome to Palpo Admin", { exact: true })).toBeVisible();
  expect(assetFailures).toEqual([]);
});
