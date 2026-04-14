import { spawnSync } from "node:child_process";

const POSTGRES_CONTAINER = process.env.PASION_PG_CONTAINER ?? "examples-postgres-1";

// Query a database inside the compose postgres container. The SQL is piped
// via stdin to avoid shell-escaping grief on Windows (spawnSync with
// `shell: true` mangles single-quoted string literals in argv on cmd.exe).
function psql(sql: string, db: "pasion" | "palpo" = "pasion"): string {
  const r = spawnSync(
    "docker",
    ["exec", "-i", POSTGRES_CONTAINER, "psql", "-U", "palpo", "-d", db, "-tA"],
    { encoding: "utf8", input: sql, shell: false }
  );
  if (r.status !== 0) {
    throw new Error(`psql failed (status=${r.status}): ${r.stderr || r.stdout}`);
  }
  return r.stdout.trim();
}

/**
 * Clear `is_guest` on all palpo users so their access tokens are accepted
 * by the palpo admin API. Palpo's delegated-auth provisioning path stamps
 * fresh users with `is_guest = true`, which blocks /_palpo/admin/v* calls
 * even when `is_admin = true`. Call after registering all test users.
 */
export function clearGuestFlag(): void {
  psql("UPDATE users SET is_guest = false WHERE is_guest = true;", "palpo");
}

/**
 * Force `is_admin = true` on a specific user. Use to make sure alice is
 * admin even if palpo's "first user is admin" heuristic changes.
 */
export function setAdmin(mxid: string, admin = true): void {
  const escaped = mxid.replace(/'/g, "''");
  psql(`UPDATE users SET is_admin = ${admin ? "true" : "false"} WHERE id = '${escaped}';`, "palpo");
}

/** Read the admin/guest flags for a specific user. */
export function getUserFlags(mxid: string): { is_admin: boolean; is_guest: boolean } | null {
  const escaped = mxid.replace(/'/g, "''");
  const out = psql(`SELECT is_admin, is_guest FROM users WHERE id = '${escaped}';`, "palpo");
  if (!out) return null;
  const [a, g] = out.split("|");
  return { is_admin: a === "t", is_guest: g === "t" };
}

/**
 * Poll until a user shows up in palpo's `users` table. Palpo's delegated
 * auth provisioning happens async via a pasion job queue, so immediately
 * after registering on pasion we may not see the matrix user yet.
 */
export async function waitForPalpoUser(
  mxid: string,
  opts: { timeoutMs?: number } = {}
): Promise<void> {
  const timeout = opts.timeoutMs ?? 30_000;
  const deadline = Date.now() + timeout;
  const escaped = mxid.replace(/'/g, "''");
  const sql = `SELECT id FROM users WHERE id = '${escaped}';`;
  while (Date.now() < deadline) {
    const out = psql(sql, "palpo");
    if (out) return;
    await new Promise((r) => setTimeout(r, 500));
  }
  throw new Error(`Palpo user ${mxid} never appeared (timeout ${timeout}ms)`);
}

/**
 * Fetch the latest email verification code for a pending registration.
 * Pasion tries to send this via SMTP (Gmail in the example config) but the
 * code is persisted in `user_email_authentication_codes` regardless.
 *
 * Pasion writes the code asynchronously from its job queue, so we poll.
 */
export async function getLatestEmailCode(
  email: string,
  opts: { timeoutMs?: number } = {}
): Promise<string> {
  const timeout = opts.timeoutMs ?? 30_000;
  const deadline = Date.now() + timeout;
  const escaped = email.replace(/'/g, "''");
  const sql = `SELECT c.code FROM user_email_authentication_codes c JOIN user_email_authentications a ON a.id = c.user_email_authentication_id WHERE a.email = '${escaped}' ORDER BY c.created_at DESC LIMIT 1;`;
  let lastOut = "";
  while (Date.now() < deadline) {
    try {
      lastOut = psql(sql);
      if (lastOut && /^\d{4,}$/.test(lastOut)) return lastOut;
    } catch (e) {
      lastOut = (e as Error).message;
    }
    await new Promise((r) => setTimeout(r, 500));
  }
  throw new Error(`No email code found for ${email} after ${timeout}ms; last output: ${lastOut}`);
}
