import { test, expect } from "@playwright/test";
import { PALPO_URL, PASION_URL } from "./helpers/services";
import { getAuthMetadata } from "./helpers/matrix-admin";

// OIDC tests run without stored auth
test.use({ storageState: { cookies: [], origins: [] } });

test.describe("OIDC / Pasion Integration", () => {
  test("Palpo exposes valid OIDC auth metadata", async ({ request }) => {
    const metadata = await getAuthMetadata(request);
    expect(metadata).not.toBeNull();
    expect(metadata!.issuer).toBe(`${PASION_URL}/`);
    expect(metadata!.authorization_endpoint).toContain(PASION_URL);
    expect(metadata!.token_endpoint).toContain(PASION_URL);
    expect(metadata!.jwks_uri).toContain(PASION_URL);
  });

  test("Pasion JWKS endpoint returns signing keys", async ({ request }) => {
    const resp = await request.get(`${PASION_URL}/oauth2/keys.json`);
    expect(resp.ok()).toBe(true);
    const body = await resp.json();
    expect(body.keys).toBeDefined();
    expect(body.keys.length).toBeGreaterThan(0);
  });

  test("Pasion OIDC discovery is accessible", async ({ request }) => {
    const resp = await request.get(`${PASION_URL}/.well-known/openid-configuration`);
    expect(resp.ok()).toBe(true);
    const body = await resp.json();
    expect(body.issuer).toBe(`${PASION_URL}/`);
    expect(body.authorization_endpoint).toBeTruthy();
    expect(body.token_endpoint).toBeTruthy();
    expect(body.registration_endpoint).toBeTruthy();
    expect(body.id_token_signing_alg_values_supported.length).toBeGreaterThan(0);
  });

  test("Pasion login page renders correctly", async ({ page }) => {
    await page.goto(`${PASION_URL}/login`);
    // Wait for WASM frontend to load
    await expect(
      page.locator('input[placeholder="Username or email"]')
        .or(page.locator('input[type="text"]').first())
    ).toBeVisible({ timeout: 20_000 });
    await expect(
      page.locator('input[type="password"]')
    ).toBeVisible();
    await expect(
      page.locator('button[type="submit"]')
        .or(page.getByRole("button", { name: "Sign in" }))
    ).toBeVisible();
  });

  test("Pasion registration page renders correctly", async ({ page }) => {
    await page.goto(`${PASION_URL}/register`);
    await expect(page.getByRole("heading", { name: "Create account" })).toBeVisible({
      timeout: 20_000,
    });
    await expect(page.locator('input[placeholder="Choose a username"]')).toBeVisible();
  });

  test("Pasion dynamic client registration works for HTTP", async ({ request }) => {
    const resp = await request.post(`${PASION_URL}/oauth2/registration`, {
      data: {
        client_name: "Playwright Test",
        client_uri: "http://localhost:9999/",
        redirect_uris: ["http://localhost:9999/callback"],
        response_types: ["code"],
        grant_types: ["authorization_code", "refresh_token"],
        token_endpoint_auth_method: "none",
        application_type: "web",
      },
    });
    expect(resp.ok()).toBe(true);
    const body = await resp.json();
    expect(body.client_id).toBeTruthy();
    expect(body.redirect_uris).toContain("http://localhost:9999/callback");
  });

  test("Pasion site config reports correct features", async ({ request }) => {
    const resp = await request.get(`${PASION_URL}/api/v1/site-config`);
    expect(resp.ok()).toBe(true);
    const body = await resp.json();
    expect(body.passwordLoginEnabled).toBe(true);
    expect(body.passwordRegistrationEnabled).toBe(true);
  });
});
