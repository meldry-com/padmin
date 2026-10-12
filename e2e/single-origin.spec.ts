import { test, expect } from "@playwright/test";

test.skip(!process.env.PADMIN_MOUNT_TEST_URL, "Requires an isolated /admin + /auth stack");

test.beforeEach(async ({ page }) => {
  // Mount coverage should not depend on the availability of external fonts.
  await page.route(/^https:\/\/fonts\.(googleapis|gstatic)\.com\//, route => route.abort());
});

test("mounted dashboard loads JS/WASM and refreshes deep links", async ({ page, request }) => {
  const failed: string[] = [];
  const errors: string[] = [];
  page.on("response", response => {
    if (new URL(response.url()).pathname.startsWith("/admin/") && response.status() >= 400) {
      failed.push(`${response.status()} ${response.url()}`);
    }
  });
  page.on("pageerror", error => errors.push(error.message));
  await page.goto("/admin/login", { waitUntil: "domcontentloaded" });
  await expect(page.getByRole("heading", { name: "Palpo Admin" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Sign In", exact: true })).toBeEnabled();
  const resources = await page.evaluate(() => performance.getEntriesByType("resource").map(e => e.name));
  expect(resources.some(url => url.includes("/admin/assets/") && url.endsWith(".wasm"))).toBeTruthy();
  expect(resources.some(url => new URL(url).pathname.startsWith("/assets/"))).toBeFalsy();
  expect(resources.some(url => url.endsWith("/admin/config.json"))).toBeTruthy();
  await page.goto("/admin/users", { waitUntil: "domcontentloaded" });
  await expect(page).toHaveURL(/\/admin\/login$/);
  await page.reload({ waitUntil: "domcontentloaded" });
  await expect(page.getByRole("heading", { name: "Palpo Admin" })).toBeVisible();
  expect(failed).toEqual([]);
  expect(errors).toEqual([]);
  expect((await request.get("/admin/assets/missing.wasm")).status()).toBe(404);
  const redirect = await request.get("/admin?test=1", { maxRedirects: 0 });
  expect(redirect.status()).toBe(308);
  expect(redirect.headers().location).toContain("/admin/?test=1");
});

test("OAuth uses the auth mount and admin callback", async ({ page }) => {
  await page.goto("/admin/login", { waitUntil: "domcontentloaded" });
  const authorization = page.waitForRequest(request => new URL(request.url()).pathname === "/auth/authorize");
  await page.getByRole("button", { name: "Sign In", exact: true }).click();
  const url = new URL((await authorization).url());
  expect(url.searchParams.get("redirect_uri")).toBe(`${url.origin}/admin/oauth/callback`);
  expect(url.searchParams.get("code_challenge_method")).toBe("S256");
});

test("Pasion assets and token exchange retain their mount", async ({ page }) => {
  test.skip(!process.env.PADMIN_MOUNT_USERNAME || !process.env.PADMIN_MOUNT_PASSWORD, "Requires a fixture admin account");
  const assetFailures: string[] = [];
  page.on("response", response => {
    if (new URL(response.url()).pathname.startsWith("/auth/assets/") && response.status() >= 400) {
      assetFailures.push(response.url());
    }
  });
  await page.goto("/admin/login", { waitUntil: "domcontentloaded" });
  await page.getByRole("button", { name: "Sign In", exact: true }).click();
  await page.getByPlaceholder("Username or email").fill(process.env.PADMIN_MOUNT_USERNAME!);
  await page.getByPlaceholder("Password", { exact: true }).fill(process.env.PADMIN_MOUNT_PASSWORD!);
  const tokenResponse = page.waitForResponse(response => new URL(response.url()).pathname.endsWith("/oauth2/token"));
  const adminProbe = page.waitForResponse(response => new URL(response.url()).pathname === "/auth/api/admin/v1/version");
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await page.waitForURL(url => url.pathname.startsWith("/auth/consent/") || url.pathname.startsWith("/admin/"));
  if (new URL(page.url()).pathname.startsWith("/auth/consent/")) {
    await page.getByRole("button", { name: "Allow", exact: true }).click();
  }
  const token = await tokenResponse;
  expect(new URL(token.url()).pathname).toBe("/auth/oauth2/token");
  expect(token.status()).toBe(200);
  await expect(page).toHaveURL(/\/admin\/$/);
  expect((await adminProbe).status()).toBe(200);
  await expect(page.getByText("Welcome to Palpo Admin", { exact: true })).toBeVisible();
  expect(assetFailures).toEqual([]);
});
