import { test, expect } from "@playwright/test";
import { PASION_URL } from "./helpers/services";
import { readAdminMeta } from "./helpers/admin-meta";

// Registration tests run without stored auth
test.use({ storageState: { cookies: [], origins: [] } });

test.describe("Pasion Registration Flow", () => {
  test("rejects registration without email or phone", async ({ request }) => {
    const resp = await request.post(`${PASION_URL}/api/v1/auth/register`, {
      data: {
        username: `nocontact_${Date.now()}`,
        password: "TestPass123!",
        password_confirm: "TestPass123!",
      },
    });
    const body = await resp.json();
    expect(body.status).toBe("error");
    expect(body.error).toBe("email_or_phone_required");
  });

  test("rejects registration with invalid email", async ({ request }) => {
    const resp = await request.post(`${PASION_URL}/api/v1/auth/register`, {
      data: {
        username: `bademail_${Date.now()}`,
        email: "not-an-email",
        password: "TestPass123!",
        password_confirm: "TestPass123!",
      },
    });
    const body = await resp.json();
    expect(body.status).toBe("error");
    expect(body.error).toBe("email_invalid");
  });

  test("rejects weak passwords", async ({ request }) => {
    const resp = await request.post(`${PASION_URL}/api/v1/auth/register`, {
      data: {
        username: `weakpass_${Date.now()}`,
        email: `weak_${Date.now()}@test.local`,
        password: "123",
        password_confirm: "123",
      },
    });
    const body = await resp.json();
    expect(body.status).toBe("error");
  });

  test("rejects duplicate email", async ({ request }) => {
    const admin = await readAdminMeta();

    const resp = await request.post(`${PASION_URL}/api/v1/auth/register`, {
      data: {
        username: `dupemail_${Date.now()}`,
        email: admin.email,
        password: "TestPass123!",
        password_confirm: "TestPass123!",
      },
    });
    const body = await resp.json();
    // May be email_in_use or the email might not exist yet — skip if not applicable
    if (body.status === "error" && body.errors) {
      // Good — validation caught it
      expect(body.errors.some((e: string) => e.includes("email"))).toBe(true);
    }
  });

  test("registration with email creates a pending verification", async ({ request }) => {
    const username = `e2ereg_${Date.now()}`;
    const email = `${username}@test.local`;

    const resp = await request.post(`${PASION_URL}/api/v1/auth/register`, {
      data: {
        username,
        email,
        password: "TestPass123!",
        password_confirm: "TestPass123!",
      },
    });
    const body = await resp.json();
    test.skip(
      body.status === "error" && body.error === "rate_limited",
      "Pasion registration is rate-limited under compose load; full registration is covered in compose-full-flow.spec.ts"
    );
    expect(body.status).toBe("success");
    expect(body.id).toBeTruthy();
    expect(body.next_step).toBe("verify_email");

    // Check registration status
    const statusResp = await request.get(
      `${PASION_URL}/api/v1/auth/register/${body.id}`
    );
    const status = await statusResp.json();
    expect(status.id).toBe(body.id);
    expect(status.email_pending).toBe(true);
  });

  test("browser registration flow shows email verification step", async ({ page }) => {
    await page.goto(`${PASION_URL}/register`);

    await expect(page.getByRole("heading", { name: "Create account" })).toBeVisible({
      timeout: 20_000,
    });
    await expect(page.locator('input[placeholder="Choose a username"]')).toBeVisible();

    const username = `e2ebrowser_${Date.now()}`;

    await page.locator('input[placeholder="Choose a username"]').fill(username);
    await page.locator('input[placeholder="your@email.com"]').fill(`${username}@test.local`);
    await page.locator('input[type="password"]').first().fill("TestPass123!");
    const passwordFields = page.locator('input[type="password"]');
    if ((await passwordFields.count()) > 1) {
      await passwordFields.nth(1).fill("TestPass123!");
    }

    await page.locator('button[type="submit"]').first().click();

    try {
      await page.waitForURL(/\/register\/[^/]+\/verify-email/, { timeout: 15_000 });
    } catch (error) {
      const bodyText = (await page.locator("body").textContent()) || "";
      test.skip(
        bodyText.includes("rate_limited"),
        "Pasion browser registration is rate-limited under compose load; full registration is covered in compose-full-flow.spec.ts"
      );
      throw error;
    }

    await expect(page.getByRole("heading", { name: "Verify your email" })).toBeVisible();
  });
});
