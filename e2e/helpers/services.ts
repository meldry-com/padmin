import { APIRequestContext } from "@playwright/test";

// Service URLs matching examples/compose.yml
export const PALPO_URL = process.env.PALPO_URL || "http://localhost:8008";
export const PASION_URL = process.env.PASION_URL || "http://localhost:8090";
export const PADMIN_URL = process.env.PADMIN_URL || "http://localhost:9090";
export const ELEMENT_URL = process.env.ELEMENT_URL || "http://localhost:8080";

const SERVICE_READY_TIMEOUT_MS = Number(process.env.SERVICE_READY_TIMEOUT_MS || "180000");
const SERVICE_RETRY_INTERVAL_MS = 1_500;

/** Check if a service is reachable. */
export async function isServiceReady(
  request: APIRequestContext,
  url: string,
  path: string = "/"
): Promise<boolean> {
  try {
    const resp = await request.get(`${url}${path}`, { timeout: 5_000 });
    return resp.ok();
  } catch {
    return false;
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function waitForService(
  request: APIRequestContext,
  svc: { name: string; url: string; path: string },
  timeoutMs: number = SERVICE_READY_TIMEOUT_MS
): Promise<void> {
  const deadline = Date.now() + timeoutMs;

  while (Date.now() < deadline) {
    if (await isServiceReady(request, svc.url, svc.path)) {
      return;
    }

    await sleep(SERVICE_RETRY_INTERVAL_MS);
  }

  throw new Error(`${svc.name} is not reachable at ${svc.url}${svc.path}`);
}

/** Wait for all compose services to be ready. */
export async function waitForServices(request: APIRequestContext): Promise<void> {
  const checks = [
    { name: "Palpo", url: PALPO_URL, path: "/_matrix/client/versions" },
    { name: "Pasion", url: PASION_URL, path: "/.well-known/openid-configuration" },
    { name: "Padmin", url: PADMIN_URL, path: "/" },
    { name: "Element", url: ELEMENT_URL, path: "/" },
  ];

  for (const svc of checks) {
    await waitForService(request, svc);
  }
}
