import { test, expect, Page } from "@playwright/test";

// Exercises the appservice management flow end-to-end against the rebuilt
// stack: YAML import → install → inline detail → in-place edit (PUT) → delete.

const HEX = "0123456789abcdef";
function token(): string {
  let s = "";
  for (let i = 0; i < 48; i++) s += HEX[i % 16];
  return s;
}

async function openAppservices(page: Page): Promise<void> {
  await page.goto("/appservices");
  await expect(page.locator("aside")).toBeVisible({ timeout: 20_000 });
  await expect(page.getByRole("heading", { name: "Appservices", exact: true })).toBeVisible({
    timeout: 15_000,
  });
}

test.describe("Appservices", () => {
  test("import → install → expand → edit → delete", async ({ page }) => {
    await openAppservices(page);

    // Unique id so re-runs don't collide with leftover rows.
    const id = `e2e-import-${Date.now()}`;
    const asToken = token();
    const hsToken = token();
    const yaml = [
      `id: ${id}`,
      "url: http://localhost:29999",
      `as_token: ${asToken}`,
      `hs_token: ${hsToken}`,
      "sender_localpart: e2eimportbot",
      "rate_limited: false",
      "namespaces:",
      "  users:",
      "    - exclusive: true",
      `      regex: '@e2eimport_.*'`,
      "  aliases: []",
      "  rooms: []",
      "",
    ].join("\n");

    // ── Import ──────────────────────────────────────────────────────────────
    await page.getByRole("button", { name: /Import registration\.yaml/i }).click();
    await expect(page.getByRole("heading", { name: /Install Custom Appservice/i })).toBeVisible({
      timeout: 15_000,
    });

    await page
      .getByPlaceholder(/Paste the registration\.yaml/i)
      .fill(yaml);
    await page.getByRole("button", { name: "Apply paste" }).click();

    // The structured fields should now reflect the parsed registration.
    await expect(page.getByPlaceholder("wechat-bridge")).toHaveValue(id);
    await expect(page.getByPlaceholder("bot")).toHaveValue("e2eimportbot");
    await expect(page.getByPlaceholder("http://localhost:29335")).toHaveValue(
      "http://localhost:29999"
    );

    // ── Install ─────────────────────────────────────────────────────────────
    await page.getByRole("button", { name: "Install", exact: true }).last().click();
    const row = page.locator("tr", { hasText: id }).first();
    await expect(row).toBeVisible({ timeout: 15_000 });
    await expect(row).toContainText("Enabled");

    // ── Inline detail: tokens + actions ─────────────────────────────────────
    await row.getByRole("button").first().click(); // chevron expand
    await expect(page.getByText(asToken)).toBeVisible({ timeout: 10_000 });
    await expect(page.getByRole("button", { name: "Download" })).toBeVisible();
    await expect(page.getByRole("button", { name: /Copy YAML/i })).toBeVisible();

    // ── Edit (PUT) ──────────────────────────────────────────────────────────
    await page.getByRole("button", { name: "Edit", exact: true }).click();
    await expect(page.getByRole("heading", { name: new RegExp(`Edit ${id}`) })).toBeVisible({
      timeout: 10_000,
    });
    // Id is immutable in edit mode.
    await expect(page.getByPlaceholder("wechat-bridge")).toBeDisabled();
    const urlField = page.getByPlaceholder("http://localhost:29335");
    await urlField.fill("http://localhost:28888");
    await page.getByRole("button", { name: "Save changes" }).click();

    // Row should reflect the new URL after the update round-trips.
    await expect(page.locator("tr", { hasText: id }).first()).toContainText(
      "http://localhost:28888",
      { timeout: 15_000 }
    );

    // ── Disable / Enable toggle ─────────────────────────────────────────────
    const liveRow = page.locator("tr", { hasText: id }).first();
    await liveRow.getByRole("button", { name: "Disable" }).click();
    await expect(page.locator("tr", { hasText: id }).first()).toContainText("Disabled", {
      timeout: 10_000,
    });
    await page
      .locator("tr", { hasText: id })
      .first()
      .getByRole("button", { name: "Enable" })
      .click();
    await expect(page.locator("tr", { hasText: id }).first()).toContainText("Enabled", {
      timeout: 10_000,
    });

    // ── Delete ──────────────────────────────────────────────────────────────
    await page.locator("tr", { hasText: id }).first().getByRole("button", { name: "Delete" }).click();
    await page.getByRole("button", { name: "Delete", exact: true }).last().click();
    await expect(page.locator("tr", { hasText: id })).toHaveCount(0, { timeout: 15_000 });
  });

  test("namespace conflict is rejected", async ({ page }) => {
    await openAppservices(page);
    const base = `e2e-conflict-${Date.now()}`;

    // First appservice claims @e2econflict_.* exclusively.
    const mk = (suffix: string) =>
      [
        `id: ${base}-${suffix}`,
        `as_token: ${token()}`,
        `hs_token: ${token()}`,
        `sender_localpart: e2econflict${suffix}`,
        "namespaces:",
        "  users:",
        "    - exclusive: true",
        `      regex: '@e2econflict_.*'`,
        "",
      ].join("\n");

    async function importAndInstall(yaml: string) {
      await page.getByRole("button", { name: /Import registration\.yaml/i }).click();
      await expect(page.getByRole("heading", { name: /Install Custom Appservice/i })).toBeVisible({
        timeout: 15_000,
      });
      await page.getByPlaceholder(/Paste the registration\.yaml/i).fill(yaml);
      await page.getByRole("button", { name: "Apply paste" }).click();
      await page.getByRole("button", { name: "Install", exact: true }).last().click();
    }

    await importAndInstall(mk("a"));
    await expect(page.locator("tr", { hasText: `${base}-a` }).first()).toBeVisible({
      timeout: 15_000,
    });

    // Second appservice claims the identical exclusive regex → must be rejected.
    await importAndInstall(mk("b"));
    await expect(page.getByText(/Namespace conflict/i)).toBeVisible({ timeout: 15_000 });

    // Clean up: close modal + delete the first one.
    await page.getByRole("button", { name: "Cancel" }).click().catch(() => {});
    await page
      .locator("tr", { hasText: `${base}-a` })
      .first()
      .getByRole("button", { name: "Delete" })
      .click();
    await page.getByRole("button", { name: "Delete", exact: true }).last().click();
  });
});
