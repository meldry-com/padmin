import { randomInt, randomUUID } from "node:crypto";
import { Client } from "pg";

const POSTGRES_HOST = process.env.TEST_PG_HOST || "127.0.0.1";
const POSTGRES_PORT = Number(process.env.TEST_PG_PORT || "15432");
const POSTGRES_USER = process.env.TEST_PG_USER || "palpo";
const POSTGRES_PASSWORD = process.env.TEST_PG_PASSWORD || "changeme";

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function withClient<T>(
  database: string,
  callback: (client: Client) => Promise<T>
): Promise<T> {
  const client = new Client({
    host: POSTGRES_HOST,
    port: POSTGRES_PORT,
    user: POSTGRES_USER,
    password: POSTGRES_PASSWORD,
    database,
  });

  await client.connect();
  try {
    return await callback(client);
  } finally {
    await client.end();
  }
}

export async function waitForPostgres(
  database: string,
  timeoutMs: number = 120_000
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  let lastError: unknown;

  while (Date.now() < deadline) {
    try {
      await withClient(database, async (client) => {
        await client.query("SELECT 1");
      });
      return;
    } catch (error) {
      lastError = error;
      await sleep(1_000);
    }
  }

  throw new Error(
    `Postgres database ${database} was not ready within ${timeoutMs}ms: ${String(lastError)}`
  );
}

export async function getLatestPasionEmailCode(
  email: string,
  timeoutMs: number = 30_000
): Promise<string> {
  const deadline = Date.now() + timeoutMs;

  while (Date.now() < deadline) {
    const code = await withClient("pasion", async (client) => {
      const result = await client.query<{ code: string }>(
        `
          SELECT c.code
          FROM user_email_authentication_codes c
          JOIN user_email_authentications a
            ON a.id = c.user_email_authentication_id
          WHERE a.email = $1
            AND a.completed_at IS NULL
          ORDER BY c.created_at DESC
          LIMIT 1
        `,
        [email]
      );
      return result.rows[0]?.code ?? null;
    });

    if (code) {
      return code;
    }

    await sleep(1_000);
  }

  throw new Error(`No email verification code found for ${email} within ${timeoutMs}ms`);
}

export async function createPasionEmailCode(email: string): Promise<string> {
  const authenticationId = await withClient("pasion", async (client) => {
    const result = await client.query<{ id: string }>(
      `
        SELECT id
        FROM user_email_authentications
        WHERE email = $1
          AND completed_at IS NULL
        ORDER BY created_at DESC
        LIMIT 1
      `,
      [email]
    );
    return result.rows[0]?.id ?? null;
  });

  if (!authenticationId) {
    throw new Error(`No pending email authentication found for ${email}`);
  }

  const code = String(randomInt(100_000, 1_000_000));

  await withClient("pasion", async (client) => {
    await client.query(
      `
        INSERT INTO user_email_authentication_codes (
          id,
          user_email_authentication_id,
          code,
          created_at,
          expires_at
        )
        VALUES ($1, $2, $3, NOW(), NOW() + INTERVAL '15 minutes')
      `,
      [randomUUID(), authenticationId, code]
    );
  });

  return code;
}

export async function waitForPalpoUser(
  userId: string,
  timeoutMs: number = 60_000
): Promise<{ id: string; is_admin: boolean }> {
  const deadline = Date.now() + timeoutMs;

  while (Date.now() < deadline) {
    const user = await withClient("palpo", async (client) => {
      const result = await client.query<{ id: string; is_admin: boolean }>(
        "SELECT id, is_admin FROM users WHERE id = $1 LIMIT 1",
        [userId]
      );
      return result.rows[0] ?? null;
    });

    if (user) {
      return user;
    }

    await sleep(1_000);
  }

  throw new Error(`Palpo user ${userId} was not provisioned within ${timeoutMs}ms`);
}

export async function setPalpoUserAdmin(
  userId: string,
  isAdmin: boolean = true
): Promise<void> {
  const rowCount = await withClient("palpo", async (client) => {
    const result = await client.query(
      "UPDATE users SET is_admin = $2 WHERE id = $1",
      [userId, isAdmin]
    );
    return result.rowCount ?? 0;
  });

  if (rowCount === 0) {
    throw new Error(`Could not update admin flag for ${userId}`);
  }
}

export async function setPasionUserCanRequestAdmin(
  username: string,
  canRequestAdmin: boolean = true
): Promise<void> {
  const rowCount = await withClient("pasion", async (client) => {
    const result = await client.query(
      "UPDATE users SET can_request_admin = $2 WHERE username = $1",
      [username, canRequestAdmin]
    );
    return result.rowCount ?? 0;
  });

  if (rowCount === 0) {
    throw new Error(`Could not update can_request_admin for ${username}`);
  }
}

export async function findLatestPasionUsername(prefix: string): Promise<string | null> {
  return withClient("pasion", async (client) => {
    const result = await client.query<{ username: string }>(
      `
        SELECT username
        FROM users
        WHERE username LIKE $1
        ORDER BY created_at DESC
        LIMIT 1
      `,
      [`${prefix}%`]
    );
    return result.rows[0]?.username ?? null;
  });
}

export async function findLatestPasionUsernames(
  prefix: string,
  limit: number
): Promise<string[]> {
  return withClient("pasion", async (client) => {
    const result = await client.query<{ username: string }>(
      `
        SELECT username
        FROM users
        WHERE username LIKE $1
        ORDER BY created_at DESC
        LIMIT $2
      `,
      [`${prefix}%`, limit]
    );
    return result.rows.map((row) => row.username);
  });
}
