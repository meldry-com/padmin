use gloo_net::http::Request;
use serde::Deserialize;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::RequestMode;

use crate::types::*;
use crate::utils::error::HttpError;
use crate::utils::storage;

// ── OAuth2 client configuration ──────────────────────────────────────────────
// The `client_id` is loaded at runtime from `/config.json` (see
// `crate::utils::config::get_oauth_client_id`). A bundle-level default is
// kept so a fresh container without a custom config still works against the
// canonical pasion deployment shipped in `examples/pasion.yaml`.
const MATRIX_API_SCOPE: &str = "urn:matrix:org.matrix.msc2967.client:api:*";
const MATRIX_DEVICE_SCOPE_PREFIX: &str = "urn:matrix:org.matrix.msc2967.client:device:";
const PALPO_ADMIN_SCOPE: &str = "urn:palpo:admin:*";
const PASION_ADMIN_SCOPE: &str = "urn:pasion:admin";
const OAUTH_DEVICE_ID_STORAGE_KEY: &str = "oauth_device_id";

#[derive(Debug)]
struct TextResponse {
    status: u16,
    text: String,
    sentry_event_id: Option<String>,
}

// - urn:matrix:...:api:* + :device:{device_id}: delegated Matrix client access
//   with a concrete device identifier for compatibility
// - urn:palpo:admin:*: Palpo admin endpoints under /_palpo/admin/*
// - urn:pasion:admin: Pasion admin endpoints under /api/admin/v1/*
fn build_oauth_scope(device_id: &str) -> String {
    format!(
        "{MATRIX_API_SCOPE} {MATRIX_DEVICE_SCOPE_PREFIX}{device_id} {PALPO_ADMIN_SCOPE} {PASION_ADMIN_SCOPE}"
    )
}

fn get_or_create_device_id() -> String {
    if let Some(device_id) = storage::get_item(OAUTH_DEVICE_ID_STORAGE_KEY)
        && is_valid_device_id(&device_id)
    {
        return device_id;
    }

    let device_id = crate::utils::password::generate_device_id();
    storage::set_item(OAUTH_DEVICE_ID_STORAGE_KEY, &device_id);
    device_id
}

fn pasion_public_base() -> Option<String> {
    crate::utils::config::get_pasion_public_url()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
}

fn oauth_public_url(path: &str) -> Option<String> {
    pasion_public_base().map(|base| format!("{base}{path}"))
}

fn sentry_event_id(response: &gloo_net::http::Response) -> Option<String> {
    response
        .headers()
        .get("X-Sentry-Event-Id")
        .filter(|value| value.chars().any(|c| c != '0' && c != '-'))
}

fn error_suffix(response: &TextResponse) -> String {
    response
        .sentry_event_id
        .as_ref()
        .map(|event_id| format!(" [event_id: {event_id}]"))
        .unwrap_or_default()
}

fn make_http_error(prefix: &str, response: &TextResponse) -> HttpError {
    make_err(format!(
        "{prefix} ({}): {}{}",
        response.status,
        response.text,
        error_suffix(response),
    ))
}

fn is_public_oauth_url(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("http://")
}

async fn read_text_response(response: gloo_net::http::Response) -> Result<TextResponse, HttpError> {
    let sentry_event_id = sentry_event_id(&response);
    let status = response.status();
    let text = response.text().await.map_err(|e| make_err(e.to_string()))?;

    Ok(TextResponse {
        status,
        text,
        sentry_event_id,
    })
}

async fn send_form_post(url: &str, body: &str) -> Result<TextResponse, HttpError> {
    let mut builder = Request::post(url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json");

    if is_public_oauth_url(url) {
        builder = builder.mode(RequestMode::Cors);
    }

    let response = builder
        .body(body.to_string())
        .map_err(|e| make_err(e.to_string()))?
        .send()
        .await
        .map_err(|e| make_err(e.to_string()))?;

    read_text_response(response).await
}

async fn send_bearer_get(url: &str, access_token: &str) -> Result<TextResponse, HttpError> {
    let mut builder = Request::get(url)
        .header("Accept", "application/json")
        .header("Authorization", &format!("Bearer {access_token}"));

    if is_public_oauth_url(url) {
        builder = builder.mode(RequestMode::Cors);
    }

    let response = builder.send().await.map_err(|e| make_err(e.to_string()))?;

    read_text_response(response).await
}

async fn send_oauth_form_request(path: &str, body: &str) -> Result<TextResponse, HttpError> {
    match send_form_post(path, body).await {
        Ok(response) if response.status < 500 => Ok(response),
        Ok(proxy_response) => {
            let Some(public_url) = oauth_public_url(path) else {
                return Ok(proxy_response);
            };

            log::warn!(
                "OAuth proxy request to {path} failed with {}, retrying {public_url}",
                proxy_response.status
            );

            match send_form_post(&public_url, body).await {
                Ok(public_response) => Ok(public_response),
                Err(err) => {
                    log::warn!("OAuth direct retry to {public_url} failed: {}", err.message);
                    Ok(proxy_response)
                }
            }
        }
        Err(proxy_err) => {
            let Some(public_url) = oauth_public_url(path) else {
                return Err(proxy_err);
            };

            log::warn!(
                "OAuth proxy request to {path} failed before a response, retrying {public_url}: {}",
                proxy_err.message
            );

            match send_form_post(&public_url, body).await {
                Ok(public_response) => Ok(public_response),
                Err(_) => Err(proxy_err),
            }
        }
    }
}

fn is_valid_device_id(device_id: &str) -> bool {
    device_id.len() >= 10
        && device_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
}

// ── PKCE helpers ─────────────────────────────────────────────────────────────

const PKCE_VERIFIER_KEY: &str = "pkce_code_verifier";
const OAUTH_STATE_KEY: &str = "oauth_state";

fn js_err(prefix: &str, value: wasm_bindgen::JsValue) -> HttpError {
    let detail = value
        .as_string()
        .or_else(|| {
            js_sys::Reflect::get(&value, &wasm_bindgen::JsValue::from_str("message"))
                .ok()
                .and_then(|v| v.as_string())
        })
        .unwrap_or_else(|| format!("{value:?}"));
    make_err(format!("{prefix}: {detail}"))
}

fn window() -> Result<web_sys::Window, HttpError> {
    web_sys::window().ok_or_else(|| make_err("browser window unavailable".into()))
}

fn session_storage() -> Result<web_sys::Storage, HttpError> {
    let storage = window()?
        .session_storage()
        .map_err(|e| js_err("sessionStorage access denied", e))?
        .ok_or_else(|| make_err("sessionStorage unavailable in this context".into()))?;
    Ok(storage)
}

fn crypto() -> Result<web_sys::Crypto, HttpError> {
    window()?
        .crypto()
        .map_err(|e| js_err("window.crypto unavailable", e))
}

/// Generate `len` random bytes via the Web Crypto API.
fn random_bytes<const N: usize>() -> Result<[u8; N], HttpError> {
    let mut buf = [0u8; N];
    crypto()?
        .get_random_values_with_u8_array(&mut buf)
        .map_err(|e| js_err("getRandomValues failed", e))?;
    Ok(buf)
}

/// Generate a cryptographically random code verifier (RFC 7636).
fn generate_code_verifier() -> Result<String, HttpError> {
    let buf = random_bytes::<32>()?;
    base64url_encode(&buf)
}

/// Generate the OAuth `state` value used for CSRF protection on the
/// authorization callback.
fn generate_oauth_state() -> Result<String, HttpError> {
    let buf = random_bytes::<32>()?;
    base64url_encode(&buf)
}

/// Compute SHA-256 of the verifier and return the base64url-encoded challenge.
async fn compute_code_challenge(verifier: &str) -> Result<String, HttpError> {
    let subtle = crypto()?.subtle();
    let data = js_sys::Uint8Array::from(verifier.as_bytes());
    let promise = subtle
        .digest_with_str_and_buffer_source("SHA-256", &data)
        .map_err(|e| js_err("subtle.digest failed", e))?;
    let result = JsFuture::from(promise)
        .await
        .map_err(|e| js_err("subtle.digest await failed", e))?;
    let buffer = result
        .dyn_into::<js_sys::ArrayBuffer>()
        .map_err(|e| js_err("digest result was not an ArrayBuffer", e))?;
    let bytes = js_sys::Uint8Array::new(&buffer).to_vec();
    base64url_encode(&bytes)
}

fn base64url_encode(data: &[u8]) -> Result<String, HttpError> {
    // Use window.btoa for base64 encoding
    let binary: String = data.iter().map(|&b| b as char).collect();
    let b64 = window()?
        .btoa(&binary)
        .map_err(|e| js_err("btoa failed", e))?;
    Ok(b64.replace('+', "-")
        .replace('/', "_")
        .trim_end_matches('=')
        .to_string())
}

// ── OAuth2 Authorization Code + PKCE flow ────────────────────────────────────

/// Start the OAuth2 login flow: redirect the browser to Pasion's /authorize.
///
/// Returns `Err` if any browser primitive (crypto, sessionStorage, …) is
/// unavailable, so the caller can render a useful error instead of having
/// the WASM panic and crash the page.
pub async fn start_oauth_login() -> Result<(), HttpError> {
    let verifier = generate_code_verifier()?;
    let challenge = compute_code_challenge(&verifier).await?;
    let state = generate_oauth_state()?;
    let device_id = get_or_create_device_id();
    let scope = build_oauth_scope(&device_id);

    // Store verifier + state in sessionStorage so the callback can match them.
    let session = session_storage()?;
    session
        .set_item(PKCE_VERIFIER_KEY, &verifier)
        .map_err(|e| js_err("sessionStorage set failed", e))?;
    session
        .set_item(OAUTH_STATE_KEY, &state)
        .map_err(|e| js_err("sessionStorage set failed", e))?;

    let redirect_uri = {
        let location = window()?.location();
        let origin = location
            .origin()
            .map_err(|e| js_err("window.location.origin unavailable", e))?;
        format!("{origin}/oauth/callback")
    };

    // Use Pasion's public URL for the browser redirect.
    // The /authorize endpoint is on Pasion's domain (not proxied through padmin).
    let pasion_base = pasion_public_base()
        .ok_or_else(|| make_err("pasion_public_url is not configured".into()))?;
    let client_id = crate::utils::config::get_oauth_client_id();

    let auth_url = format!(
        "{pasion_base}/authorize?response_type=code\
         &client_id={}\
         &redirect_uri={}\
         &state={}\
         &code_challenge={challenge}\
         &code_challenge_method=S256\
         &scope={}",
        urlencoding::encode(&client_id),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(&state),
        urlencoding::encode(&scope),
    );

    // Full page redirect to Pasion login
    window()?
        .location()
        .set_href(&auth_url)
        .map_err(|e| js_err("location.href assignment failed", e))?;
    Ok(())
}

/// Exchange the authorization code for tokens (called from /oauth/callback).
///
/// `received_state` must be the `state` query parameter the callback page
/// observed. Pass `None` to skip the check (legacy paths only — callers in
/// the browser SHOULD always pass it).
pub async fn handle_oauth_callback(
    code: &str,
    received_state: Option<&str>,
) -> Result<(), HttpError> {
    let session = session_storage()?;
    let verifier = session
        .get_item(PKCE_VERIFIER_KEY)
        .ok()
        .flatten()
        .ok_or_else(|| make_err("Missing PKCE verifier — please restart login".into()))?;
    let stored_state = session.get_item(OAUTH_STATE_KEY).ok().flatten();
    // Always clear the one-shot values so a replay can't reuse them.
    session.remove_item(PKCE_VERIFIER_KEY).ok();
    session.remove_item(OAUTH_STATE_KEY).ok();

    match (stored_state.as_deref(), received_state) {
        (Some(stored), Some(received)) if stored == received => {}
        (Some(_), Some(_)) => {
            return Err(make_err(
                "OAuth state mismatch — refusing token exchange. Please restart login.".into(),
            ));
        }
        (Some(_), None) => {
            return Err(make_err(
                "OAuth callback missing `state` parameter — refusing token exchange.".into(),
            ));
        }
        (None, _) => {
            // No stored state means start_oauth_login wasn't run in this tab,
            // or the user navigated back. Treat as a restart-required error.
            return Err(make_err(
                "Missing OAuth state — please restart login.".into(),
            ));
        }
    }

    let redirect_uri = {
        let origin = window()?
            .location()
            .origin()
            .map_err(|e| js_err("window.location.origin unavailable", e))?;
        format!("{origin}/oauth/callback")
    };

    // Token exchange
    let client_id = crate::utils::config::get_oauth_client_id();
    let form_body = format!(
        "grant_type=authorization_code\
         &code={}\
         &redirect_uri={}\
         &client_id={}\
         &code_verifier={}",
        urlencoding::encode(code),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(&client_id),
        urlencoding::encode(&verifier),
    );

    let response = send_oauth_form_request("/oauth2/token", &form_body).await?;

    if response.status >= 400 {
        return Err(make_http_error("Token exchange failed", &response));
    }

    let token_resp: TokenResponse =
        serde_json::from_str(&response.text).map_err(|e| make_err(e.to_string()))?;

    storage::set_item("access_token", &token_resp.access_token);
    if let Some(ref rt) = token_resp.refresh_token {
        storage::set_item("refresh_token", rt);
    }

    // Resolve the Matrix identity directly from the delegated access token.
    // This keeps the admin dashboard login flow aligned with the tested
    // device-code helper and avoids depending on OIDC userinfo / id_token.
    let whoami = send_bearer_get(
        "/_matrix/client/v3/account/whoami",
        &token_resp.access_token,
    )
    .await?;
    if whoami.status >= 400 {
        return Err(make_http_error("Matrix whoami failed", &whoami));
    }

    let whoami: WhoamiResponse =
        serde_json::from_str(&whoami.text).map_err(|e| make_err(e.to_string()))?;
    storage::set_item("user_id", &whoami.user_id);
    if let Some(device_id) = whoami.device_id.as_deref()
        && is_valid_device_id(device_id)
    {
        storage::set_item(OAUTH_DEVICE_ID_STORAGE_KEY, device_id);
    }

    if let Some((_, displayname, avatar)) = get_identity().await {
        if let Some(name) = displayname {
            storage::set_item("user_display_name", &name);
        }
        if let Some(url) = avatar {
            storage::set_item("user_avatar_url", &url);
        }
    }

    Ok(())
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
}

// ── OAuth2 token refresh ────────────────────────────────────────────────────

/// Shared 401 handler for API clients.
///
/// When any backend (palpo or pasion) returns 401, we first try to
/// refresh the OAuth2 access token. If refresh succeeds, the caller
/// should retry the original request. Otherwise, we clear the cached
/// session state — the `AppLayout` heartbeat polls `is_authenticated()`
/// every 2s and redirects to `/login` once the token is gone, so this
/// function doesn't need to navigate directly.
///
/// Returns `true` if the caller should retry, `false` if the session
/// is gone and the redirect will happen on the next tick.
pub async fn handle_unauthorized() -> bool {
    if refresh_oauth_token().await {
        return true;
    }
    // Refresh failed: clear everything that implies an active session.
    storage::remove_item("access_token");
    storage::remove_item("refresh_token");
    storage::remove_item("is_admin");
    // Drop the session-scoped admin verdict so the next authenticated mount
    // re-probes instead of trusting a now-stale cache.
    crate::router::reset_admin_cache();
    false
}

/// Attempt to refresh the OAuth2 access token using the stored refresh_token.
/// Returns `true` if the token was refreshed successfully.
pub async fn refresh_oauth_token() -> bool {
    let refresh_token = match storage::get_item("refresh_token") {
        Some(rt) => rt,
        None => return false,
    };

    let client_id = crate::utils::config::get_oauth_client_id();
    let form_body = format!(
        "grant_type=refresh_token\
         &refresh_token={}\
         &client_id={}",
        urlencoding::encode(&refresh_token),
        urlencoding::encode(&client_id),
    );

    let response = match send_oauth_form_request("/oauth2/token", &form_body).await {
        Ok(response) => response,
        Err(_) => return false,
    };

    if response.status >= 400 {
        return false;
    }

    let token_resp: TokenResponse = match serde_json::from_str(&response.text) {
        Ok(t) => t,
        Err(_) => return false,
    };

    storage::set_item("access_token", &token_resp.access_token);
    if let Some(ref rt) = token_resp.refresh_token {
        storage::set_item("refresh_token", rt);
    }

    true
}

// ── Admin verification ───────────────────────────────────────────────────────

/// Check whether the currently authenticated user has homeserver admin
/// privileges by probing a cheap admin-only endpoint
/// (`GET /_palpo/admin/v1/server_version`):
///
///   - 200 → admin
///   - 403 M_FORBIDDEN → authenticated but not admin
///   - 401 → token is bad / expired (let the caller refresh)
///   - network / other errors → returned as `HttpError` so the caller can
///     decide whether to retry or treat as "unknown"
///
/// We deliberately avoid `GET /_palpo/admin/v2/users/{self}` here — the
/// `user_id` we have in localStorage is pasion's OAuth subject (a ULID),
/// not the Matrix `@localpart:server_name` that palpo's admin API
/// expects, and palpo rejects it with 400 `M_BAD_JSON`.
///
/// The result is cached in `localStorage.is_admin` so subsequent page
/// loads don't re-probe. [`logout`] clears the cache.
pub async fn verify_admin() -> Result<bool, HttpError> {
    let access_token =
        storage::get_item("access_token").ok_or_else(|| make_err("Not authenticated".into()))?;

    let response = Request::get("/_palpo/admin/v1/server_version")
        .header("Accept", "application/json")
        .header("Authorization", &format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|e| make_err(e.to_string()))?;

    let status = response.status();
    if status == 200 {
        storage::set_item("is_admin", "true");
        return Ok(true);
    }
    if status == 403 {
        storage::set_item("is_admin", "false");
        return Ok(false);
    }
    let text = response.text().await.unwrap_or_default();
    Err(make_err(format!("admin probe HTTP {status}: {text}")))
}

/// Read the cached admin flag populated by [`verify_admin`].
/// Returns `None` if the check hasn't run yet.
pub fn cached_is_admin() -> Option<bool> {
    storage::get_item("is_admin").map(|v| v == "true")
}

// ── Logout ───────────────────────────────────────────────────────────────────

pub async fn logout() -> Result<(), HttpError> {
    // Step 1: revoke the OAuth access token so it can no longer be used
    // against palpo's admin API.
    if let Some(token) = storage::get_item("access_token") {
        let client_id = crate::utils::config::get_oauth_client_id();
        let body = format!(
            "token={}&client_id={}",
            urlencoding::encode(&token),
            urlencoding::encode(&client_id),
        );
        let _ = send_oauth_form_request("/oauth2/revoke", &body).await;
    }

    // Step 2: end the pasion browser session. Without this, clicking
    // "Sign In" again would silently re-use the same pasion session
    // cookie, skip the login prompt entirely, and land on the consent
    // screen — so switching accounts is impossible. Hitting
    // POST /api/v1/auth/logout (proxied to pasion same-origin via nginx)
    // clears pasion's `mas_session` cookie, forcing a fresh login next
    // time.
    let _ = Request::post("/api/v1/auth/logout")
        .header("Accept", "application/json")
        .send()
        .await;

    // Clear cached identity / admin flag so the next login starts fresh.
    storage::remove_item("is_admin");
    crate::utils::config::clear_config();
    Ok(())
}

// ── Session helpers ──────────────────────────────────────────────────────────

pub fn is_authenticated() -> bool {
    storage::get_item("access_token").is_some()
}

pub async fn get_identity() -> Option<(String, Option<String>, Option<String>)> {
    let access_token = storage::get_item("access_token")?;
    let user_id = storage::get_item("user_id")?;

    let url = format!(
        "/_matrix/client/v3/profile/{}",
        urlencoding::encode(&user_id)
    );

    let response = Request::get(&url)
        .header("Accept", "application/json")
        .header("Authorization", &format!("Bearer {access_token}"))
        .send()
        .await
        .ok()?;

    if response.status() >= 400 {
        return Some((user_id, None, None));
    }

    let profile: ProfileResponse = response.json().await.ok()?;
    Some((user_id, profile.displayname, profile.avatar_url))
}

// ── Utility endpoints (used by auth_status page) ─────────────────────────────

fn make_err(msg: String) -> HttpError {
    HttpError::message(msg)
}

#[cfg(test)]
mod tests {
    use super::build_oauth_scope;

    #[test]
    fn oauth_scope_uses_concrete_device_id() {
        let scope = build_oauth_scope("ABCdef123456");

        assert!(scope.contains("urn:matrix:org.matrix.msc2967.client:device:ABCdef123456"));
        assert!(!scope.contains("urn:matrix:org.matrix.msc2967.client:device:*"));
        assert!(scope.contains("urn:palpo:admin:*"));
        assert!(scope.contains("urn:pasion:admin"));
        assert!(!scope.contains("openid"));
    }
}

pub async fn get_login_flows(_base_url: &str) -> Result<Vec<LoginFlow>, HttpError> {
    let response = Request::get("/_matrix/client/v3/login")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| make_err(e.to_string()))?;
    let flows_resp: LoginFlowsResponse =
        response.json().await.map_err(|e| make_err(e.to_string()))?;
    Ok(flows_resp.flows)
}

pub async fn get_auth_issuer(_base_url: &str) -> Result<serde_json::Value, HttpError> {
    let response = Request::get("/_matrix/client/unstable/org.matrix.msc2965/auth_issuer")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| make_err(e.to_string()))?;
    if response.status() >= 400 {
        return Err(make_err(format!("HTTP {}", response.status())));
    }
    response.json().await.map_err(|e| make_err(e.to_string()))
}

pub async fn get_oidc_discovery(issuer_url: &str) -> Result<serde_json::Value, HttpError> {
    let url = format!(
        "{}/.well-known/openid-configuration",
        issuer_url.trim_end_matches('/')
    );
    let response = Request::get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| make_err(e.to_string()))?;
    if response.status() >= 400 {
        return Err(make_err(format!("HTTP {}", response.status())));
    }
    response.json().await.map_err(|e| make_err(e.to_string()))
}
