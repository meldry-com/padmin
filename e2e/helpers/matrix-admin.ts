import { APIRequestContext } from "@playwright/test";
import { PALPO_URL } from "./services";

/**
 * Check if the Palpo Matrix server is reachable.
 */
export async function isServerReady(
  request: APIRequestContext
): Promise<boolean> {
  try {
    const resp = await request.get(`${PALPO_URL}/_matrix/client/versions`);
    return resp.ok();
  } catch {
    return false;
  }
}

/**
 * Verify an access token by calling whoami.
 */
export async function whoami(
  request: APIRequestContext,
  accessToken: string
): Promise<{ user_id: string } | null> {
  try {
    const resp = await request.get(
      `${PALPO_URL}/_matrix/client/v3/account/whoami`,
      { headers: { Authorization: `Bearer ${accessToken}` } }
    );
    if (resp.ok()) return resp.json();
    return null;
  } catch {
    return null;
  }
}

/**
 * Login via Matrix client API using password (when password login is enabled).
 */
export async function matrixLogin(
  request: APIRequestContext,
  username: string,
  password: string
): Promise<{ user_id: string; access_token: string; device_id: string }> {
  const resp = await request.post(
    `${PALPO_URL}/_matrix/client/v3/login`,
    {
      data: {
        type: "m.login.password",
        identifier: { type: "m.id.user", user: username },
        password,
        initial_device_display_name: "Playwright E2E",
      },
    }
  );
  if (!resp.ok()) {
    const body = await resp.json().catch(() => ({}));
    throw new Error(
      `Matrix login failed (${resp.status()}): ${body.errcode || "unknown"} — ${body.error || ""}`
    );
  }
  return resp.json();
}

/**
 * Logout via Matrix client API and invalidate the access token.
 */
export async function matrixLogout(
  request: APIRequestContext,
  accessToken: string
): Promise<void> {
  const resp = await request.post(`${PALPO_URL}/_matrix/client/v3/logout`, {
    headers: { Authorization: `Bearer ${accessToken}` },
  });

  if (!resp.ok()) {
    throw new Error(`Matrix logout failed: ${resp.status()}`);
  }
}

/**
 * Create a room and optionally invite other users.
 */
export async function matrixCreateRoom(
  request: APIRequestContext,
  accessToken: string,
  options: {
    name: string;
    topic?: string;
    invite?: string[];
    isDirect?: boolean;
  }
): Promise<{ room_id: string }> {
  const resp = await request.post(`${PALPO_URL}/_matrix/client/v3/createRoom`, {
    headers: { Authorization: `Bearer ${accessToken}` },
    data: {
      name: options.name,
      topic: options.topic,
      invite: options.invite || [],
      is_direct: options.isDirect ?? false,
      preset: "private_chat",
    },
  });

  if (!resp.ok()) {
    const body = await resp.text();
    throw new Error(`Create room failed (${resp.status()}): ${body}`);
  }

  return resp.json();
}

/**
 * Join a room using a room ID or alias.
 */
export async function matrixJoinRoom(
  request: APIRequestContext,
  accessToken: string,
  roomIdOrAlias: string
): Promise<void> {
  const encoded = encodeURIComponent(roomIdOrAlias);
  const resp = await request.post(
    `${PALPO_URL}/_matrix/client/v3/join/${encoded}`,
    {
      headers: { Authorization: `Bearer ${accessToken}` },
      data: {},
    }
  );

  if (!resp.ok()) {
    const body = await resp.text();
    throw new Error(`Join room failed (${resp.status()}): ${body}`);
  }
}

/**
 * Send a plain text message to a room.
 */
export async function matrixSendMessage(
  request: APIRequestContext,
  accessToken: string,
  roomId: string,
  body: string
): Promise<{ event_id: string }> {
  const txnId = `${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
  const encodedRoomId = encodeURIComponent(roomId);
  const encodedTxnId = encodeURIComponent(txnId);

  const resp = await request.put(
    `${PALPO_URL}/_matrix/client/v3/rooms/${encodedRoomId}/send/m.room.message/${encodedTxnId}`,
    {
      headers: { Authorization: `Bearer ${accessToken}` },
      data: {
        msgtype: "m.text",
        body,
      },
    }
  );

  if (!resp.ok()) {
    const responseBody = await resp.text();
    throw new Error(`Send message failed (${resp.status()}): ${responseBody}`);
  }

  return resp.json();
}

/**
 * Poll /sync until a room message body becomes visible to the target user.
 */
export async function syncUntilMessage(
  request: APIRequestContext,
  accessToken: string,
  roomId: string,
  body: string,
  timeoutMs: number = 30_000
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  let since: string | undefined;

  while (Date.now() < deadline) {
    const url = new URL(`${PALPO_URL}/_matrix/client/v3/sync`);
    url.searchParams.set("timeout", "1000");
    if (since) {
      url.searchParams.set("since", since);
    }

    const resp = await request.get(url.toString(), {
      headers: { Authorization: `Bearer ${accessToken}` },
      timeout: 10_000,
    });

    if (!resp.ok()) {
      const responseBody = await resp.text();
      throw new Error(`Sync failed (${resp.status()}): ${responseBody}`);
    }

    const payload = (await resp.json()) as {
      next_batch?: string;
      rooms?: {
        join?: Record<string, { timeline?: { events?: Array<Record<string, any>> } }>;
      };
    };

    since = payload.next_batch ?? since;

    const events = payload.rooms?.join?.[roomId]?.timeline?.events ?? [];
    const hasMessage = events.some(
      (event) =>
        event.type === "m.room.message" &&
        event.content &&
        event.content.body === body
    );

    if (hasMessage) {
      return;
    }
  }

  throw new Error(`Timed out waiting for room message "${body}" in ${roomId}`);
}

/**
 * Get the list of users via Palpo admin API.
 */
export async function listUsers(
  request: APIRequestContext,
  accessToken: string,
  from: number = 0,
  limit: number = 10,
  guests: boolean = false
): Promise<{ users: Array<Record<string, unknown>>; total: number }> {
  const resp = await request.get(
    `${PALPO_URL}/_palpo/admin/v2/users?from=${from}&limit=${limit}&guests=${guests}`,
    { headers: { Authorization: `Bearer ${accessToken}` } }
  );
  if (!resp.ok()) {
    throw new Error(`List users failed: ${resp.status()}`);
  }
  return resp.json();
}

/**
 * Get OIDC auth metadata from Palpo.
 */
export async function getAuthMetadata(
  request: APIRequestContext
): Promise<Record<string, unknown> | null> {
  try {
    const resp = await request.get(
      `${PALPO_URL}/_matrix/client/unstable/org.matrix.msc2965/auth_metadata`
    );
    if (resp.ok()) return resp.json();
    return null;
  } catch {
    return null;
  }
}
