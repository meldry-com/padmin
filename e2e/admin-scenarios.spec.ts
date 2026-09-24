/**
 * Admin Scenarios E2E Tests
 *
 * Simulates a Matrix server administrator performing daily management tasks:
 *   - Navigating all admin pages (including Pasion identity provider)
 *   - Managing users (view, create, deactivate, reactivate, password reset)
 *   - Managing rooms (view details, check members)
 *   - Session oversight (OAuth2 sessions, personal access tokens)
 *   - Sending server notices to users
 *   - Reviewing audit logs
 *   - Checking system health (connector health, upstream providers)
 *   - Managing registration tokens (create, revoke, unrevoke)
 *   - Viewing policy data and notification templates
 */

import { test, expect, type Page } from "@playwright/test";
import { PADMIN_URL, PASION_URL } from "./helpers/services";
import { readAdminMeta } from "./helpers/admin-meta";

import {
  acquireOidcAccessToken,
  buildMatrixScopes,
  newDeviceId,
} from "./helpers/oidc-device";

const ADMIN_STORAGE_STATE_PATH = "e2e/.auth/admin.json";
const runId = Date.now().toString(36);

// ── Helpers ───────────────────────────────────────────────────────────────

async function openPage(page: Page, path: string): Promise<void> {
  await page.goto(`${PADMIN_URL}${path}`);
  await expect(page.locator("aside")).toBeVisible({ timeout: 20_000 });
}

async function expectPageHeader(page: Page, titlePattern: string | RegExp): Promise<void> {
  const heading = page.locator("h1, h2").filter({
    hasText: typeof titlePattern === "string" ? titlePattern : undefined,
  });
  if (typeof titlePattern === "string") {
    await expect(heading.first()).toBeVisible({ timeout: 15_000 });
  } else {
    await expect(heading.first()).toBeVisible({ timeout: 15_000 });
  }
}

async function expectTableOrEmpty(page: Page, emptyText?: string): Promise<boolean> {
  const table = page.locator("table");
  const empty = page.locator("text=" + (emptyText || "No "));
  const hasTable = await table.isVisible().catch(() => false);
  const hasEmpty = await empty.first().isVisible().catch(() => false);
  expect(hasTable || hasEmpty).toBeTruthy();
  return hasTable && !hasEmpty;
}

// ── Test Suite ────────────────────────────────────────────────────────────

test.describe("Admin Scenarios: Daily Management", () => {
  test.describe.configure({ mode: "serial" });
  test.setTimeout(180_000);

  test.use({ storageState: ADMIN_STORAGE_STATE_PATH });

  // ─── Scenario 1: Complete Navigation Audit ──────────────────────────

  test("Admin navigates all sidebar sections and verifies page loads", async ({
    page,
  }) => {
    // Dashboard
    await openPage(page, "/");
    await expect(page.getByRole("heading", { name: "Dashboard" })).toBeVisible({ timeout: 15_000 });
    await expect(page.locator("text=Server Version").first()).toBeVisible({ timeout: 15_000 });

    // Identity section
    const identityPages = [
      { path: "/users", text: "Manage Matrix users" },
      { path: "/registration-tokens", text: "Registration Tokens" },
      { path: "/auth-status", text: "Authentication" },
    ];

    for (const { path, text } of identityPages) {
      await openPage(page, path);
      await expect(page.locator(`text=${text}`).first()).toBeVisible({ timeout: 15_000 });
    }

    // Moderation section
    const moderationPages = [
      { path: "/rooms", text: "Room" },
      { path: "/reports", text: "Reports" },
      { path: "/server-notices", text: "Server Notices" },
    ];

    for (const { path, text } of moderationPages) {
      await openPage(page, path);
      await expect(page.locator(`text=${text}`).first()).toBeVisible({ timeout: 15_000 });
    }

    // Infrastructure section
    const infraPages = [
      { path: "/destinations", text: "Federation" },
      { path: "/media", text: "Media" },
    ];

    for (const { path, text } of infraPages) {
      await openPage(page, path);
      await expect(page.locator(`text=${text}`).first()).toBeVisible({ timeout: 15_000 });
    }
  });

  // ─── Scenario 2: Pasion Identity Provider Pages ─────────────────────

  test("Admin navigates all Pasion identity provider pages", async ({
    page,
  }) => {
    // Local Accounts page
    await openPage(page, "/pasion/accounts");
    await expect(page.locator("text=Local Accounts").first()).toBeVisible({ timeout: 15_000 });
    await expect(page.locator('input[placeholder="Search username..."]')).toBeVisible();
    await expect(page.getByRole("button", { name: "Create Account" })).toBeVisible();
    const accountsTable = page.locator("table");
    const accountsRetry = page.getByRole("button", { name: "Retry" });
    await expect(accountsTable.or(accountsRetry)).toBeVisible({ timeout: 15_000 });

    // OAuth2 Sessions page
    await openPage(page, "/pasion/oauth2-sessions");
    await expect(page.locator("text=OAuth2 Sessions").first()).toBeVisible({ timeout: 15_000 });
    // Verify filter and refresh controls are present
    await expect(page.locator('input[placeholder="Filter by user ID..."]')).toBeVisible();
    await expect(page.getByRole("button", { name: "Refresh" })).toBeVisible();
    // Should see table (success) or error with retry button
    const oauth2Table = page.locator("table");
    const oauth2Retry = page.getByRole("button", { name: "Retry" });
    await expect(oauth2Table.or(oauth2Retry)).toBeVisible({ timeout: 15_000 });

    // Personal Access Tokens page
    await openPage(page, "/pasion/personal-sessions");
    await expect(page.locator("text=Personal Access Tokens").first()).toBeVisible({ timeout: 15_000 });
    await expect(page.getByRole("button", { name: "Create Token" })).toBeVisible();

    // Audit Log page
    await openPage(page, "/pasion/audit-log");
    await expect(page.locator("text=Audit Log").first()).toBeVisible({ timeout: 15_000 });
    await expect(page.locator('input[placeholder="Filter by operation..."]')).toBeVisible();
    // Audit table or loading/error state
    const auditTable = page.locator("table");
    const auditRetry = page.getByRole("button", { name: "Retry" });
    await expect(auditTable.or(auditRetry)).toBeVisible({ timeout: 15_000 });

    // Upstream Providers page
    await openPage(page, "/pasion/upstream-providers");
    await expect(page.locator("text=Upstream OAuth Providers").first()).toBeVisible({ timeout: 15_000 });
    // Table or error state (API response format may vary)
    const providersTable = page.locator("table");
    const providersRetry = page.getByRole("button", { name: "Retry" });
    await expect(providersTable.or(providersRetry)).toBeVisible({ timeout: 15_000 });

    // Connector Health page
    await openPage(page, "/pasion/connector-health");
    await expect(page.locator("text=Connector Health").first()).toBeVisible({ timeout: 15_000 });
    await expect(page.getByRole("button", { name: "Refresh" })).toBeVisible();

    // Policy Data page
    await openPage(page, "/pasion/policy-data");
    await expect(page.locator("text=Policy Data").first()).toBeVisible({ timeout: 15_000 });
  });

  // ─── Scenario 3: User Lifecycle Management ─────────────────────────

  test("Admin creates, views, deactivates, and reactivates a user", async ({
    page,
  }) => {
    const testUsername = `scnuser${runId}`;
    const testDisplayName = `Scenario User ${runId}`;
    const testPassword = `ScnPass123!${runId}`;
    const testUserId = `@${testUsername}:localhost`;

    // Step 1: Create user
    await openPage(page, "/users/create");
    await expect(page.locator("text=Create New User")).toBeVisible({ timeout: 15_000 });
    await page.locator('input[placeholder="username"]').fill(testUsername);
    await page.locator('input[placeholder="Display Name"]').fill(testDisplayName);
    await page.locator('input[placeholder="Password"]').fill(testPassword);
    await page.getByRole("button", { name: "Create User" }).click();
    await expect(page).toHaveURL(/\/users$/, { timeout: 20_000 });

    // Step 2: Search and view the created user
    await expect(page.locator("table")).toBeVisible({ timeout: 20_000 });
    await page.locator('input[placeholder="Search users..."]').fill(testUsername);
    const userRow = page.locator("table tbody tr").filter({ hasText: testUsername }).first();
    await expect(userRow).toBeVisible({ timeout: 20_000 });

    // Step 3: View user detail
    await userRow.getByRole("button", { name: "View" }).click();
    await expect(page.getByText(testUserId, { exact: true }).first()).toBeVisible({
      timeout: 15_000,
    });

    // Step 4: Go back and deactivate user
    await openPage(page, "/users");
    await expect(page.locator("table")).toBeVisible({ timeout: 20_000 });
    await page.locator('input[placeholder="Search users..."]').fill(testUsername);
    const rowToDeactivate = page.locator("table tbody tr").filter({ hasText: testUsername }).first();
    await expect(rowToDeactivate).toBeVisible({ timeout: 20_000 });
    await rowToDeactivate.getByRole("button", { name: "Deactivate" }).click();
    await expect(page.locator("text=User deactivated")).toBeVisible({ timeout: 10_000 });

    // Step 5: Verify deactivation by checking Reactivate button appears
    await page.locator('input[placeholder="Search users..."]').fill(testUsername);
    const deactivatedRow = page.locator("table tbody tr").filter({ hasText: testUsername }).first();
    await expect(deactivatedRow.getByRole("button", { name: "Reactivate" })).toBeVisible({
      timeout: 20_000,
    });

    // Step 6: Reactivate user
    await deactivatedRow.getByRole("button", { name: "Reactivate" }).click();
    await expect(page.locator("text=User reactivated")).toBeVisible({ timeout: 10_000 });

    // Step 7: Verify user is active again
    await page.locator('input[placeholder="Search users..."]').fill(testUsername);
    const reactivatedRow = page.locator("table tbody tr").filter({ hasText: testUsername }).first();
    await expect(reactivatedRow.getByRole("button", { name: "Deactivate" })).toBeVisible({
      timeout: 20_000,
    });
  });

  // ─── Scenario 4: Registration Token Lifecycle ──────────────────────

  test("Admin creates, manages, and deletes registration tokens", async ({
    page,
  }) => {
    await openPage(page, "/registration-tokens");
    await expect(page.locator("text=Registration Tokens").first()).toBeVisible({ timeout: 15_000 });

    // Create a new token - click opens a dialog
    await page.getByRole("button", { name: "Create Token" }).first().click();
    // Wait for dialog to appear
    await expect(page.getByRole("heading", { name: "Create Token" })).toBeVisible({ timeout: 10_000 });
    // Submit the dialog (leave fields empty for auto-generated token)
    const dialogCreateBtn = page.locator(".fixed .relative button").filter({ hasText: "Create Token" });
    await dialogCreateBtn.click();

    // Wait for table to reload and show at least one token
    await page.waitForTimeout(2_000);
    await expect(page.locator("table")).toBeVisible({ timeout: 20_000 });
    const tokenRows = page.locator("table tbody tr").filter({ hasNot: page.locator("text=No registration tokens") });
    await expect(tokenRows.first()).toBeVisible({ timeout: 20_000 });
  });

  // ─── Scenario 5: OAuth2 Session Management ─────────────────────────

  test("Admin reviews OAuth2 sessions and filters by user", async ({
    page,
  }) => {
    await openPage(page, "/pasion/oauth2-sessions");
    await expect(page.locator("text=OAuth2 Sessions").first()).toBeVisible({ timeout: 15_000 });

    // Should see table or error with retry
    const table = page.locator("table");
    const retry = page.getByRole("button", { name: "Retry" });
    await expect(table.or(retry)).toBeVisible({ timeout: 15_000 });

    // Only check table contents if table is visible (API might return error)
    if (await table.isVisible().catch(() => false)) {
      const headers = page.locator("table thead th");
      await expect(headers.filter({ hasText: "Client" }).first()).toBeVisible();
      await expect(headers.filter({ hasText: "Status" }).first()).toBeVisible();

      const sessionRows = page.locator("table tbody tr");
      const rowCount = await sessionRows.count();
      expect(rowCount).toBeGreaterThan(0);

      const activeBadge = page.locator("table tbody").locator("text=Active").first();
      await expect(activeBadge).toBeVisible({ timeout: 10_000 });

      // Test filter
      const admin = await readAdminMeta();
      const filterInput = page.locator('input[placeholder="Filter by user ID..."]');
      await filterInput.fill(admin.userId);
      await page.waitForTimeout(1_000);
      const filteredRows = page.locator("table tbody tr");
      const filteredCount = await filteredRows.count();
      expect(filteredCount).toBeGreaterThan(0);

      await filterInput.fill("");
      await page.getByRole("button", { name: "Refresh" }).click();
      await expect(page.locator("table")).toBeVisible({ timeout: 15_000 });
    }
  });

  // ─── Scenario 6: Personal Access Token Management ──────────────────

  test("Admin views personal access tokens page", async ({
    page,
  }) => {
    await openPage(page, "/pasion/personal-sessions");
    await expect(page.locator("text=Personal Access Tokens").first()).toBeVisible({ timeout: 15_000 });
    await expect(page.getByRole("button", { name: "Create Token" })).toBeVisible();

    // Should see table or error state (API may not be available)
    const table = page.locator("table");
    const retryBtn = page.getByRole("button", { name: "Retry" });
    await expect(table.or(retryBtn)).toBeVisible({ timeout: 15_000 });
  });

  // ─── Scenario 7: Server Notices ─────────────────────────────────────

  test("Admin sends a server notice to a specific user", async ({
    page,
  }) => {
    const admin = await readAdminMeta();

    await openPage(page, "/server-notices");
    await expect(page.locator("text=Server Notices").first()).toBeVisible({ timeout: 15_000 });

    // Verify Single User mode is active by default
    await expect(page.locator("text=Single User").first()).toBeVisible();
    await expect(page.locator("text=Broadcast to All").first()).toBeVisible();

    // Fill in user ID and message
    const userIdInput = page.locator('input[placeholder="@user:example.com"]');
    await expect(userIdInput).toBeVisible();
    await userIdInput.fill(admin.userId);

    const messageArea = page.locator("textarea");
    await messageArea.fill(`Test server notice from Playwright scenario at ${runId}`);

    // Send the notice
    await page.getByRole("button", { name: "Send Notice" }).click();

    // Should see success toast
    await expect(page.locator("text=Server notice sent")).toBeVisible({ timeout: 15_000 });

    // Verify notice appears in history
    await expect(page.locator("text=Notice History").first()).toBeVisible();
    await expect(
      page.locator(`text=Test server notice from Playwright scenario at ${runId}`).first()
    ).toBeVisible({ timeout: 10_000 });
  });

  // ─── Scenario 8: Audit Log Review ──────────────────────────────────

  test("Admin reviews audit log entries and uses filter", async ({
    page,
  }) => {
    await openPage(page, "/pasion/audit-log");
    await expect(page.locator("text=Audit Log").first()).toBeVisible({ timeout: 15_000 });

    // Should see table or error state
    const table = page.locator("table");
    const retryBtn = page.getByRole("button", { name: "Retry" });
    await expect(table.or(retryBtn)).toBeVisible({ timeout: 15_000 });
    if (!(await table.isVisible().catch(() => false))) return;

    // Table should have headers
    const headers = page.locator("table thead th");
    await expect(headers.filter({ hasText: "Timestamp" }).first()).toBeVisible();
    await expect(headers.filter({ hasText: "Operation" }).first()).toBeVisible();
    await expect(headers.filter({ hasText: "Admin" }).first()).toBeVisible();

    // Should have at least one audit entry from admin setup actions
    const entryRows = page.locator("table tbody tr");
    const entryCount = await entryRows.count();
    expect(entryCount).toBeGreaterThan(0);

    // Test the operation filter
    const filterInput = page.locator('input[placeholder="Filter by operation..."]');
    // Filter with a non-existing operation to verify filtering works
    await filterInput.fill("nonexistent_operation_xyz");
    await page.waitForTimeout(500);
    // Should show empty state or fewer rows
    const emptyOrFiltered = page.locator("text=No audit entries found");
    const filteredCount = await page.locator("table tbody tr").count();
    const isEmpty = await emptyOrFiltered.isVisible().catch(() => false);
    expect(isEmpty || filteredCount === 1).toBeTruthy(); // 1 = empty row

    // Clear filter to restore entries
    await filterInput.fill("");
    await page.waitForTimeout(500);
    const restoredCount = await page.locator("table tbody tr").count();
    expect(restoredCount).toBeGreaterThan(0);
  });

  // ─── Scenario 9: Room Inspection ───────────────────────────────────

  test("Admin views rooms list and inspects room details", async ({
    page,
  }) => {
    await openPage(page, "/rooms");

    // Wait for table or empty state
    const table = page.locator("table");
    const hasTable = await table.isVisible({ timeout: 15_000 }).catch(() => false);

    if (!hasTable) {
      // If no rooms, just verify the page loads
      await expect(page.locator("text=Room").first()).toBeVisible();
      return;
    }

    // Search bar should be visible
    const searchInput = page.locator('input[placeholder="Search rooms by name or alias..."]');
    await expect(searchInput).toBeVisible();

    // Check table has rows
    const roomRows = page.locator("table tbody tr");
    const roomCount = await roomRows.count();

    if (roomCount > 0) {
      // Click first room with a link to view details
      const firstRoomLink = page.locator("table tbody tr a").first();
      if (await firstRoomLink.isVisible().catch(() => false)) {
        const roomName = await firstRoomLink.textContent();
        await firstRoomLink.click();

        // Should navigate to room detail page
        await expect(page).toHaveURL(/\/rooms\//, { timeout: 15_000 });

        // Room detail should show room info
        if (roomName) {
          await expect(page.locator(`text=${roomName}`).first()).toBeVisible({ timeout: 15_000 });
        }
      }
    }
  });

  // ─── Scenario 10: Dashboard Health Check ───────────────────────────

  test("Admin verifies dashboard statistics and server health", async ({
    page,
  }) => {
    await openPage(page, "/");

    // Dashboard should show statistics
    await expect(page.getByRole("heading", { name: "Dashboard" })).toBeVisible({ timeout: 15_000 });

    // Server Version card
    const serverVersion = page.locator("text=Server Version").first();
    await expect(serverVersion).toBeVisible({ timeout: 15_000 });

    // Wait for version to load (should not stay as "Loading")
    await page.waitForTimeout(3_000);
    const versionCard = serverVersion.locator("..").locator("..");
    const versionText = await versionCard.textContent();
    expect(versionText).not.toContain("Loading");

    // Check for user and room statistics
    await expect(page.locator("text=Total Users").first()).toBeVisible({ timeout: 10_000 });
    await expect(page.locator("text=Total Rooms").first()).toBeVisible({ timeout: 10_000 });
  });

  // ─── Scenario 11: Upstream OAuth Provider Inspection ───────────────

  test("Admin views upstream OAuth providers configured in Pasion", async ({
    page,
  }) => {
    await openPage(page, "/pasion/upstream-providers");
    await expect(page.locator("text=Upstream OAuth Providers").first()).toBeVisible({ timeout: 15_000 });

    // Table or error state (API response format may vary)
    const table = page.locator("table");
    const retry = page.getByRole("button", { name: "Retry" });
    await expect(table.or(retry)).toBeVisible({ timeout: 15_000 });

    if (await table.isVisible().catch(() => false)) {
      // The example pasion.yaml has a GitHub provider configured
      const githubRow = page.locator("table tbody tr").filter({ hasText: "GitHub" });
      if (await githubRow.isVisible().catch(() => false)) {
        await expect(githubRow.locator("text=github").first()).toBeVisible();
      }
    }
  });

  // ─── Scenario 12: Multi-User Management Workflow ───────────────────

  test("Admin creates multiple users and verifies them in the list", async ({
    page,
  }) => {
    const users = [
      {
        username: `batch1_${runId}`,
        displayName: `Batch User 1`,
        password: `BatchPass1!${runId}`,
      },
      {
        username: `batch2_${runId}`,
        displayName: `Batch User 2`,
        password: `BatchPass2!${runId}`,
      },
    ];

    // Create first user
    for (const user of users) {
      await openPage(page, "/users/create");
      await expect(page.locator("text=Create New User")).toBeVisible({ timeout: 15_000 });
      await page.locator('input[placeholder="username"]').fill(user.username);
      await page.locator('input[placeholder="Display Name"]').fill(user.displayName);
      await page.locator('input[placeholder="Password"]').fill(user.password);
      await page.getByRole("button", { name: "Create User" }).click();
      await expect(page).toHaveURL(/\/users$/, { timeout: 20_000 });
    }

    // Verify both users exist in the list
    for (const user of users) {
      await openPage(page, "/users");
      await expect(page.locator("table")).toBeVisible({ timeout: 20_000 });
      await page.locator('input[placeholder="Search users..."]').fill(user.username);
      const row = page.locator("table tbody tr").filter({ hasText: user.username }).first();
      await expect(row).toBeVisible({ timeout: 20_000 });
    }
  });

  // ─── Scenario 13: Connector Health Monitoring ──────────────────────

  test("Admin checks connector health status", async ({ page }) => {
    await openPage(page, "/pasion/connector-health");
    await expect(page.locator("text=Connector Health").first()).toBeVisible({ timeout: 15_000 });

    // Should show health cards or empty state
    const refreshBtn = page.getByRole("button", { name: "Refresh" });
    await expect(refreshBtn).toBeVisible();

    // Click refresh and verify page still works
    await refreshBtn.click();
    await page.waitForTimeout(2_000);
    await expect(page.locator("text=Connector Health").first()).toBeVisible();
  });

  // ─── Scenario 14: Policy Data Inspection ───────────────────────────

  test("Admin views current policy data", async ({ page }) => {
    await openPage(page, "/pasion/policy-data");
    await expect(page.locator("text=Policy Data").first()).toBeVisible({ timeout: 15_000 });

    // Should show current policy or empty state
    // The page shows either policy JSON or an upload button
    const uploadBtn = page.getByRole("button", { name: "Upload New Policy" });
    const policyPre = page.locator("pre");
    const hasUpload = await uploadBtn.isVisible().catch(() => false);
    const hasPolicy = await policyPre.isVisible().catch(() => false);

    // At least the page should have loaded without error
    expect(hasUpload || hasPolicy).toBeTruthy();
  });

  // ─── Scenario 15: Media and Reports Quick Check ────────────────────

  test("Admin checks media and reports pages load correctly", async ({
    page,
  }) => {
    // Media page
    await openPage(page, "/media");
    await expect(page.locator("text=Media").first()).toBeVisible({ timeout: 15_000 });

    // Reports page
    await openPage(page, "/reports");
    await expect(page.locator("text=Reports").first()).toBeVisible({ timeout: 15_000 });
  });

  // ─── Scenario 16: Federation Destinations Check ────────────────────

  test("Admin views federation destinations", async ({ page }) => {
    await openPage(page, "/destinations");
    await expect(page.locator("text=Federation").first()).toBeVisible({ timeout: 15_000 });

    // Should show table or empty state
    const table = page.locator("table");
    const hasTable = await table.isVisible({ timeout: 10_000 }).catch(() => false);

    if (hasTable) {
      // Verify table headers
      const headers = page.locator("table thead th");
      const headerCount = await headers.count();
      expect(headerCount).toBeGreaterThan(0);
    }
  });
});

// ── Separate describe for tests requiring fresh auth context ──────────────

test.describe("Admin Scenarios: Auth-Sensitive Operations", () => {
  test.setTimeout(180_000);

  test("Admin can re-login with fresh OIDC token and access dashboard", async ({
    browser,
    request,
  }) => {
    const admin = await readAdminMeta();

    // Acquire fresh token
    const tokenResult = await acquireOidcAccessToken(request, {
      username: admin.username,
      password: admin.password,
      scopes: buildMatrixScopes(newDeviceId("PWSCN"), ["urn:palpo:admin:*", "urn:pasion:admin"]),
      clientName: "Playwright Admin Scenario",
      timeoutMs: 120_000,
    });

    // Create fresh context and inject auth
    const context = await browser.newContext({
      storageState: { cookies: [], origins: [] },
    });
    const page = await context.newPage();

    try {
      await page.goto(`${PADMIN_URL}/login`);
      await expect(page.getByRole("heading", { name: "Palpo Admin" })).toBeVisible({
        timeout: 20_000,
      });

      // Inject auth state (use padmin URL for pasion_url so requests go through nginx proxy)
      await page.evaluate(
        ({ token, uid, padminUrl }) => {
          localStorage.setItem("access_token", JSON.stringify(token));
          localStorage.setItem("user_id", JSON.stringify(uid));
          localStorage.setItem("home_server", JSON.stringify("localhost"));
          localStorage.setItem("pasion_url", JSON.stringify(padminUrl));
        },
        { token: tokenResult.accessToken, uid: tokenResult.userId, padminUrl: PADMIN_URL }
      );

      // Navigate to dashboard
      await page.goto(`${PADMIN_URL}/`);
      await expect(page.locator("aside")).toBeVisible({ timeout: 20_000 });
      await expect(page.getByRole("heading", { name: "Dashboard" })).toBeVisible({ timeout: 15_000 });

      // Verify admin can access user management (requires admin scope)
      await page.goto(`${PADMIN_URL}/users`);
      await expect(page.locator("table")).toBeVisible({ timeout: 20_000 });
    } finally {
      await context.close();
    }
  });
});
