import {
  APIRequestContext,
  request as playwrightRequest,
} from "@playwright/test";
import { PALPO_URL, PASION_URL } from "./services";
import { whoami } from "./matrix-admin";

type AuthMetadata = {
  registration_endpoint: string;
  device_authorization_endpoint: string;
  token_endpoint: string;
};

type DeviceGrant = {
  device_code: string;
  user_code: string;
  verification_uri: string;
  verification_uri_complete?: string;
  interval?: number;
};

type TokenResponse = {
  access_token: string;
  refresh_token?: string;
  scope?: string;
};

type AcquireTokenOptions = {
  username: string;
  password: string;
  scopes: string[];
  clientName?: string;
  timeoutMs?: number;
};

const DEFAULT_TIMEOUT_MS = 120_000;
const DEFAULT_CLIENT_NAME = "Playwright E2E";
const DEVICE_SCOPE_PREFIX = "urn:matrix:org.matrix.msc2967.client:device:";
const MATRIX_API_SCOPE = "urn:matrix:org.matrix.msc2967.client:api:*";

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function randomSuffix(length: number = 12): string {
  return Math.random().toString(36).slice(2, 2 + length);
}

export function buildMatrixDeviceScope(deviceId: string): string {
  return `${DEVICE_SCOPE_PREFIX}${deviceId}`;
}

export function buildMatrixScopes(deviceId: string, extraScopes: string[] = []): string[] {
  return [MATRIX_API_SCOPE, buildMatrixDeviceScope(deviceId), ...extraScopes];
}

async function getAuthMetadata(request: APIRequestContext): Promise<AuthMetadata> {
  const response = await request.get(
    `${PALPO_URL}/_matrix/client/unstable/org.matrix.msc2965/auth_metadata`
  );

  if (!response.ok()) {
    throw new Error(`Failed to load auth metadata: ${response.status()}`);
  }

  const body = (await response.json()) as Partial<AuthMetadata>;

  if (
    !body.registration_endpoint ||
    !body.device_authorization_endpoint ||
    !body.token_endpoint
  ) {
    throw new Error(`Incomplete auth metadata: ${JSON.stringify(body)}`);
  }

  return {
    registration_endpoint: body.registration_endpoint,
    device_authorization_endpoint: body.device_authorization_endpoint,
    token_endpoint: body.token_endpoint,
  };
}

async function registerDeviceClient(
  request: APIRequestContext,
  registrationEndpoint: string,
  clientName: string
): Promise<string> {
  const response = await request.post(registrationEndpoint, {
    data: {
      client_name: clientName,
      client_uri: "https://github.com/taidge/padmin/",
      grant_types: ["urn:ietf:params:oauth:grant-type:device_code", "refresh_token"],
      application_type: "native",
      token_endpoint_auth_method: "none",
    },
  });

  const body = await response.json().catch(() => ({}));
  if (!response.ok() || !body.client_id) {
    throw new Error(
      `Dynamic client registration failed (${response.status()}): ${JSON.stringify(body)}`
    );
  }

  return body.client_id as string;
}

async function requestDeviceGrant(
  request: APIRequestContext,
  endpoint: string,
  clientId: string,
  scopes: string[]
): Promise<DeviceGrant> {
  const response = await request.post(endpoint, {
    form: {
      client_id: clientId,
      scope: scopes.join(" "),
    },
  });

  const body = (await response.json().catch(() => ({}))) as Partial<DeviceGrant>;
  if (!response.ok() || !body.device_code || !body.user_code) {
    throw new Error(
      `Device authorization failed (${response.status()}): ${JSON.stringify(body)}`
    );
  }

  return {
    device_code: body.device_code,
    user_code: body.user_code,
    verification_uri: body.verification_uri || "",
    verification_uri_complete: body.verification_uri_complete,
    interval: body.interval,
  };
}

async function resolveDeviceGrantId(
  request: APIRequestContext,
  userCode: string
): Promise<string> {
  const response = await request.get(`${PASION_URL}/api/v1/device-link?code=${userCode}`);
  const body = (await response.json().catch(() => ({}))) as {
    status?: string;
    grantId?: string;
  };

  if (!response.ok() || body.status !== "valid" || !body.grantId) {
    throw new Error(
      `Device-link lookup failed (${response.status()}): ${JSON.stringify(body)}`
    );
  }

  return body.grantId;
}

async function loginPasionSession(
  sessionRequest: APIRequestContext,
  username: string,
  password: string,
  timeoutMs: number = 60_000
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  let retryDelayMs = 2_000;

  while (true) {
    const response = await sessionRequest.post(`${PASION_URL}/api/v1/auth/login`, {
      data: { username, password },
    });
    const body = await response.json().catch(() => ({}));

    if (response.ok() && body.status === "success") {
      return;
    }

    const shouldRetry =
      body.error === "rate_limited" && Date.now() + retryDelayMs < deadline;
    if (shouldRetry) {
      await sleep(retryDelayMs);
      retryDelayMs = Math.min(retryDelayMs * 2, 10_000);
      continue;
    }

    throw new Error(`Pasion login failed (${response.status()}): ${JSON.stringify(body)}`);
  }
}

async function approveDeviceGrant(
  sessionRequest: APIRequestContext,
  grantId: string
): Promise<void> {
  const getResponse = await sessionRequest.get(`${PASION_URL}/api/v1/device-consent/${grantId}`);
  const getBody = (await getResponse.json().catch(() => ({}))) as {
    error?: string;
    policyViolation?: boolean;
    policy_violation?: boolean;
  };

  if (!getResponse.ok() || getBody.error === "not_authenticated") {
    throw new Error(
      `Device consent lookup failed (${getResponse.status()}): ${JSON.stringify(getBody)}`
    );
  }

  if (getBody.policyViolation || getBody.policy_violation) {
    throw new Error(`Device consent denied by policy for grant ${grantId}`);
  }

  const postResponse = await sessionRequest.post(
    `${PASION_URL}/api/v1/device-consent/${grantId}`,
    {
      data: { action: "consent" },
    }
  );
  const postBody = await postResponse.json().catch(() => ({}));

  if (!postResponse.ok() || postBody.status !== "fulfilled") {
    throw new Error(
      `Device consent approval failed (${postResponse.status()}): ${JSON.stringify(postBody)}`
    );
  }
}

async function pollForToken(
  request: APIRequestContext,
  tokenEndpoint: string,
  clientId: string,
  deviceCode: string,
  intervalSeconds: number,
  timeoutMs: number
): Promise<TokenResponse> {
  const deadline = Date.now() + timeoutMs;
  let intervalMs = Math.max(intervalSeconds, 1) * 1000;

  while (Date.now() < deadline) {
    const response = await request.post(tokenEndpoint, {
      form: {
        grant_type: "urn:ietf:params:oauth:grant-type:device_code",
        device_code: deviceCode,
        client_id: clientId,
      },
    });

    const body = await response.json().catch(() => ({}));

    if (response.ok() && body.access_token) {
      return body as TokenResponse;
    }

    if (body.error === "authorization_pending") {
      await sleep(intervalMs);
      continue;
    }

    if (body.error === "slow_down") {
      intervalMs += 1000;
      await sleep(intervalMs);
      continue;
    }

    throw new Error(`Token exchange failed (${response.status()}): ${JSON.stringify(body)}`);
  }

  throw new Error(`Timed out waiting for device-code token after ${timeoutMs}ms`);
}

export async function acquireOidcAccessToken(
  request: APIRequestContext,
  options: AcquireTokenOptions
): Promise<{
  accessToken: string;
  refreshToken?: string;
  clientId: string;
  userId: string;
  scope?: string;
}> {
  const timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  const metadata = await getAuthMetadata(request);
  const clientId = await registerDeviceClient(
    request,
    metadata.registration_endpoint,
    options.clientName || DEFAULT_CLIENT_NAME
  );
  const grant = await requestDeviceGrant(
    request,
    metadata.device_authorization_endpoint,
    clientId,
    options.scopes
  );
  const grantId = await resolveDeviceGrantId(request, grant.user_code);

  const sessionRequest = await playwrightRequest.newContext({
    extraHTTPHeaders: { Accept: "application/json" },
    ignoreHTTPSErrors: true,
  });

  try {
    await loginPasionSession(
      sessionRequest,
      options.username,
      options.password,
      Math.min(timeoutMs, 60_000)
    );
    await approveDeviceGrant(sessionRequest, grantId);
  } finally {
    await sessionRequest.dispose();
  }

  const token = await pollForToken(
    request,
    metadata.token_endpoint,
    clientId,
    grant.device_code,
    grant.interval ?? 5,
    timeoutMs
  );

  const me = await whoami(request, token.access_token);
  if (!me) {
    throw new Error("OIDC token was issued but Palpo /whoami rejected it");
  }

  return {
    accessToken: token.access_token,
    refreshToken: token.refresh_token,
    clientId,
    userId: me.user_id,
    scope: token.scope,
  };
}

export function newDeviceId(prefix: string = "PWE2E"): string {
  return `${prefix}${randomSuffix(12)}`.slice(0, 20);
}
