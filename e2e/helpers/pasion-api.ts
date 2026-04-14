import { APIRequestContext } from "@playwright/test";
import { PASION_URL } from "./services";

/**
 * Register a new user via Pasion REST API (multi-step).
 * Returns the registration session ID.
 */
export async function pasionRegister(
  request: APIRequestContext,
  username: string,
  password: string,
  email?: string,
  phone?: string
): Promise<{ id: string; next_step: string }> {
  const resp = await request.post(`${PASION_URL}/api/v1/auth/register`, {
    data: { username, password, password_confirm: password, email, phone },
  });
  const body = await resp.json();
  if (body.status !== "success") {
    throw new Error(`Pasion registration failed: ${JSON.stringify(body)}`);
  }
  return { id: body.id, next_step: body.next_step };
}

/**
 * Verify email during Pasion registration using a code from the DB.
 */
export async function pasionVerifyEmail(
  request: APIRequestContext,
  registrationId: string,
  code: string
): Promise<{ next_step: string }> {
  const resp = await request.post(
    `${PASION_URL}/api/v1/auth/register/${registrationId}/verify-email`,
    { data: { code } }
  );
  const body = await resp.json();
  if (body.status !== "success") {
    throw new Error(`Email verification failed: ${JSON.stringify(body)}`);
  }
  return { next_step: body.next_step };
}

/**
 * Set display name during registration.
 */
export async function pasionSetDisplayName(
  request: APIRequestContext,
  registrationId: string,
  displayName?: string
): Promise<{ next_step: string }> {
  const resp = await request.post(
    `${PASION_URL}/api/v1/auth/register/${registrationId}/display-name`,
    { data: displayName ? { display_name: displayName } : { skip: true } }
  );
  const body = await resp.json();
  if (body.status !== "success") {
    throw new Error(`Set display name failed: ${JSON.stringify(body)}`);
  }
  return { next_step: body.next_step };
}

/**
 * Finish Pasion registration — creates the user account.
 */
export async function pasionFinishRegistration(
  request: APIRequestContext,
  registrationId: string
): Promise<void> {
  const resp = await request.post(
    `${PASION_URL}/api/v1/auth/register/${registrationId}/finish`,
    { data: {} }
  );
  const body = await resp.json();
  if (body.status !== "success") {
    throw new Error(`Finish registration failed: ${JSON.stringify(body)}`);
  }
}

/**
 * Login via Pasion REST API (creates a session cookie).
 */
export async function pasionLogin(
  request: APIRequestContext,
  username: string,
  password: string
): Promise<{ user_id: string; mxid: string }> {
  const resp = await request.post(`${PASION_URL}/api/v1/auth/login`, {
    data: { username, password },
  });
  const body = await resp.json();
  if (body.status !== "success") {
    throw new Error(`Pasion login failed: ${JSON.stringify(body)}`);
  }
  return { user_id: body.viewer.id, mxid: body.viewer.mxid };
}

/**
 * Get the current viewer info (checks if session is active).
 */
export async function pasionViewer(
  request: APIRequestContext
): Promise<{ id: string; username: string; mxid: string } | null> {
  const resp = await request.get(`${PASION_URL}/api/v1/viewer`);
  const body = await resp.json();
  if (body.status === "success" && body.viewer) {
    return body.viewer;
  }
  return null;
}

/**
 * Get the Pasion site config.
 */
export async function pasionSiteConfig(
  request: APIRequestContext
): Promise<Record<string, unknown>> {
  const resp = await request.get(`${PASION_URL}/api/v1/site-config`);
  return resp.json();
}
