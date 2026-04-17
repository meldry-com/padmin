import { test, expect } from "@playwright/test";
import { readAdminMeta } from "./helpers/admin-meta";
import {
  acquireOidcAccessToken,
  buildMatrixScopes,
  newDeviceId,
} from "./helpers/oidc-device";
import {
  matrixCreateRoom,
  matrixReportEvent,
  matrixSendMessage,
} from "./helpers/matrix-admin";

test.describe("Reports", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/reports");
    await expect(page.locator("aside")).toBeVisible({ timeout: 20_000 });
  });

  test("shows reports page", async ({ page }) => {
    await expect(page.getByRole("heading", { name: "Reports" })).toBeVisible({
      timeout: 10_000,
    });
  });

  test("shows reports table or empty state", async ({ page }) => {
    await expect(page.locator("table")).toBeVisible({ timeout: 20_000 });
  });

  test("status persists after refresh", async ({ page, request }) => {
    const admin = await readAdminMeta();
    const token = await acquireOidcAccessToken(request, {
      username: admin.username,
      password: admin.password,
      scopes: buildMatrixScopes(newDeviceId("PWRPT"), [
        "urn:palpo:admin:*",
        "urn:pasion:admin",
      ]),
      clientName: "Playwright Reports",
      timeoutMs: 120_000,
    });
    const runId = Date.now().toString(36);
    const reportReason = `Playwright report ${runId}`;
    const room = await matrixCreateRoom(request, token.accessToken, {
      name: `Report Room ${runId}`,
      topic: "Playwright reports persistence check",
    });
    const message = await matrixSendMessage(
      request,
      token.accessToken,
      room.room_id,
      `Reported message ${runId}`
    );

    await matrixReportEvent(request, token.accessToken, room.room_id, message.event_id, {
      reason: reportReason,
      score: -80,
    });

    await page.goto("/reports");
    await expect(page.locator("table")).toBeVisible({ timeout: 20_000 });

    const reportRow = page.locator("table tbody tr").filter({ hasText: reportReason }).first();
    await expect(reportRow).toBeVisible({ timeout: 20_000 });
    await reportRow.getByRole("link").first().click();

    await expect(page).toHaveURL(/\/reports\/\d+$/, { timeout: 15_000 });

    const statusSelect = page.getByRole("combobox").first();
    await expect(statusSelect).toHaveValue("new", { timeout: 15_000 });

    await statusSelect.selectOption("resolved");
    await expect(page.locator("text=Status updated")).toBeVisible({ timeout: 10_000 });
    await expect(statusSelect).toHaveValue("resolved", { timeout: 15_000 });

    await page.reload();
    await expect(page.getByRole("combobox").first()).toHaveValue("resolved", {
      timeout: 20_000,
    });

    await page.goto("/reports");
    const resolvedRow = page.locator("table tbody tr").filter({ hasText: reportReason }).first();
    await expect(resolvedRow).toContainText("Resolved", { timeout: 20_000 });
  });
});
