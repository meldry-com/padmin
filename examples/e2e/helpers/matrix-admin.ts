// Helpers that talk to palpo's MAS (Matrix Authentication Service) admin
// endpoints using the shared mas_secret. These bypass the usual
// pasion-driven provisioning flow when the test needs to guarantee a
// specific user/device state on the homeserver.
//
// mas_secret must match palpo.toml `[admin].mas_secret`. The example
// stack hardcodes `replace-with-a-random-secret`, we mirror that here.

const PALPO_BASE = process.env.PALPO_BASE ?? "http://localhost:8008";
const MAS_SECRET = process.env.PALPO_MAS_SECRET ?? "replace-with-a-random-secret";

async function post(path: string, body: unknown): Promise<Response> {
  return fetch(`${PALPO_BASE}${path}`, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${MAS_SECRET}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify(body),
  });
}

/**
 * Ensure a user device exists in palpo. Palpo's delegated-auth check
 * (crates/server/src/hoops/auth.rs::auth_by_delegated_token) requires
 * *some* user_device row for the calling user; if we don't stamp one,
 * all /_palpo/admin/* calls from that user return 401 with
 * "Device not found (not yet provisioned?)".
 */
export async function upsertDevice(localpart: string, deviceId = "E2E_SEED"): Promise<void> {
  const res = await post("/_palpo/admin/upsert_device", {
    localpart,
    device_id: deviceId,
  });
  if (!res.ok) {
    const text = await res.text().catch(() => "");
    throw new Error(`upsert_device ${localpart}/${deviceId} failed: ${res.status} ${text}`);
  }
}
