use gloo_net::http::Request;
use serde::Deserialize;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use crate::types::*;
use crate::utils::error::{HttpError, MatrixError, display_error};
use crate::utils::storage;

// ── OAuth2 client configuration ──────────────────────────────────────────────
// Must match the client_id registered in pasion.yaml
const OAUTH_CLIENT_ID: &str = "01KMQPADM1N000000000000000";
// - openid: userinfo access
// - urn:matrix:...:api:*, :device:*  palpo admin endpoints gated by delegated
//   introspection
// - urn:pasion:admin: pasion's /api/admin/v1/* endpoints (upstream providers,
//   personal sessions, audit feed, …). Without it every pasion admin call
//   returns 401 "Missing admin scope".
const OAUTH_SCOPE: &str = "openid urn:matrix:org.matrix.msc2967.client:api:* urn:matrix:org.matrix.msc2967.client:device:* urn:pasion:admin";

// ── PKCE helpers ─────────────────────────────────────────────────────────────

/// Generate a cryptographically random code verifier (RFC 7636).
fn generate_code_verifier() -> String {
    let crypto = web_sys::window().unwrap().crypto().unwrap();
    let mut buf = [0u8; 32];
    crypto
        .get_random_values_with_u8_array(&mut buf)
        .expect("get_random_values failed");
    base64url_encode(&buf)
}

/// Compute SHA-256 of the verifier and return the base64url-encoded challenge.
async fn compute_code_challenge(verifier: &str) -> String {
    let crypto = web_sys::window().unwrap().crypto().unwrap();
    let subtle = crypto.subtle();
    let data = js_sys::Uint8Array::from(verifier.as_bytes());
    let promise = subtle
        .digest_with_str_and_buffer_source("SHA-256", &data)
        .expect("digest failed");
    let result = JsFuture::from(promise).await.expect("digest await failed");
    let buffer = result.dyn_into::<js_sys::ArrayBuffer>().unwrap();
    let bytes = js_sys::Uint8Array::new(&buffer).to_vec();
    base64url_encode(&bytes)
}

fn base64url_encode(data: &[u8]) -> String {
    // Use window.btoa for base64 encoding
    let binary: String = data.iter().map(|&b| b as char).collect();
    let b64 = web_sys::window()
        .unwrap()
        .btoa(&binary)
        .expect("btoa failed");
    b64.replace('+', "-")
        .replace('/', "_")
        .trim_end_matches('=')
        .to_string()
}

// ── OAuth2 Authorization Code + PKCE flow ────────────────────────────────────

/// Start the OAuth2 login flow: redirect the browser to Pasion's /authorize.
pub async fn start_oauth_login() {
    let verifier = generate_code_verifier();
    let challenge = compute_code_challenge(&verifier).await;

    // Store verifier in sessionStorage for the callback
    let session = web_sys::window()
        .unwrap()
        .session_storage()
        .unwrap()
        .unwrap();
    session
        .set_item("pkce_code_verifier", &verifier)
        .expect("sessionStorage set failed");

    let redirect_uri = {
        let location = web_sys::window().unwrap().location();
        let origin = location.origin().unwrap();
        format!("{origin}/oauth/callback")
    };

    // Use Pasion's public URL for the browser redirect.
    // The /authorize endpoint is on Pasion's domain (not proxied through padmin).
    let pasion_base = crate::utils::config::get_pasion_public_url()
        .unwrap_or_default();

    let auth_url = format!(
        "{pasion_base}/authorize?response_type=code\
         &client_id={OAUTH_CLIENT_ID}\
         &redirect_uri={}\
         &code_challenge={challenge}\
         &code_challenge_method=S256\
         &scope={}",
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(OAUTH_SCOPE),
    );

    // Full page redirect to Pasion login
    web_sys::window()
        .unwrap()
        .location()
        .set_href(&auth_url)
        .expect("redirect failed");
}

/// Exchange the authorization code for tokens (called from /oauth/callback).
pub async fn handle_oauth_callback(code: &str) -> Result<(), HttpError> {
    let session = web_sys::window()
        .unwrap()
        .session_storage()
        .unwrap()
        .unwrap();
    let verifier = session
        .get_item("pkce_code_verifier")
        .ok()
        .flatten()
        .ok_or_else(|| make_err("Missing PKCE verifier — please restart login".into()))?;
    session.remove_item("pkce_code_verifier").ok();

    let redirect_uri = {
        let origin = web_sys::window().unwrap().location().origin().unwrap();
        format!("{origin}/oauth/callback")
    };

    // Token exchange
    let form_body = format!(
        "grant_type=authorization_code\
         &code={}\
         &redirect_uri={}\
         &client_id={OAUTH_CLIENT_ID}\
         &code_verifier={}",
        urlencoding::encode(code),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(&verifier),
    );

    let response = Request::post("/oauth2/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .body(form_body)
        .map_err(|e| make_err(e.to_string()))?
        .send()
        .await
        .map_err(|e| make_err(e.to_string()))?;

    let status = response.status();
    let text = response.text().await.map_err(|e| make_err(e.to_string()))?;

    if status >= 400 {
        return Err(make_err(format!("Token exchange failed ({status}): {text}")));
    }

    let token_resp: TokenResponse =
        serde_json::from_str(&text).map_err(|e| make_err(e.to_string()))?;

    storage::set_item("access_token", &token_resp.access_token);
    if let Some(ref rt) = token_resp.refresh_token {
        storage::set_item("refresh_token", rt);
    }

    // Fetch user identity from userinfo endpoint
    let userinfo = Request::get("/oauth2/userinfo")
        .header("Accept", "application/json")
        .header(
            "Authorization",
            &format!("Bearer {}", token_resp.access_token),
        )
        .send()
        .await
        .map_err(|e| make_err(e.to_string()))?;

    if userinfo.status() < 400 {
        let info: serde_json::Value =
            userinfo.json().await.map_err(|e| make_err(e.to_string()))?;
        if let Some(sub) = info.get("sub").and_then(|v| v.as_str()) {
            storage::set_item("user_id", sub);
        }
        // Store human-readable display name for the header
        let display = info
            .get("username")
            .or_else(|| info.get("preferred_username"))
            .or_else(|| info.get("name"))
            .or_else(|| info.get("email"))
            .and_then(|v| v.as_str());
        if let Some(name) = display {
            storage::set_item("user_display_name", name);
        }
        if let Some(picture) = info.get("picture").and_then(|v| v.as_str()) {
            storage::set_item("user_avatar_url", picture);
        }
    }

    // Fallback: if userinfo didn't provide a human-readable name, try the
    // Matrix profile endpoint for the just-logged-in user. This covers
    // upstream OIDC providers whose claims_imports don't map `name` /
    // `preferred_username` into Pasion's userinfo response.
    if storage::get_item("user_display_name").is_none()
        || storage::get_item("user_avatar_url").is_none()
    {
        if let Some((_, displayname, avatar)) = get_identity().await {
            if let Some(name) = displayname {
                storage::set_item("user_display_name", &name);
            }
            if let Some(url) = avatar {
                storage::set_item("user_avatar_url", &url);
            }
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
    false
}

/// Attempt to refresh the OAuth2 access token using the stored refresh_token.
/// Returns `true` if the token was refreshed successfully.
pub async fn refresh_oauth_token() -> bool {
    let refresh_token = match storage::get_item("refresh_token") {
        Some(rt) => rt,
        None => return false,
    };

    let form_body = format!(
        "grant_type=refresh_token\
         &refresh_token={}\
         &client_id={OAUTH_CLIENT_ID}",
        urlencoding::encode(&refresh_token),
    );

    let response = match Request::post("/oauth2/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .body(form_body)
    {
        Ok(req) => match req.send().await {
            Ok(resp) => resp,
            Err(_) => return false,
        },
        Err(_) => return false,
    };

    if response.status() >= 400 {
        return false;
    }

    let text = match response.text().await {
        Ok(t) => t,
        Err(_) => return false,
    };

    let token_resp: TokenResponse = match serde_json::from_str(&text) {
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
    let access_token = storage::get_item("access_token")
        .ok_or_else(|| make_err("Not authenticated".into()))?;

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
        let body = format!(
            "token={}&client_id={OAUTH_CLIENT_ID}",
            urlencoding::encode(&token)
        );
        if let Ok(req) = Request::post("/oauth2/revoke")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
        {
            let _ = req.send().await;
        }
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
    HttpError { message: msg, status: 0, body: None, request_id: None }
}

pub async fn get_login_flows(_base_url: &str) -> Result<Vec<LoginFlow>, HttpError> {
    let response = Request::get("/_matrix/client/v3/login")
        .header("Accept", "application/json")
        .send().await.map_err(|e| make_err(e.to_string()))?;
    let flows_resp: LoginFlowsResponse = response.json().await.map_err(|e| make_err(e.to_string()))?;
    Ok(flows_resp.flows)
}

pub async fn get_auth_issuer(_base_url: &str) -> Result<serde_json::Value, HttpError> {
    let response = Request::get("/_matrix/client/unstable/org.matrix.msc2965/auth_issuer")
        .header("Accept", "application/json")
        .send().await.map_err(|e| make_err(e.to_string()))?;
    if response.status() >= 400 {
        return Err(make_err(format!("HTTP {}", response.status())));
    }
    response.json().await.map_err(|e| make_err(e.to_string()))
}

pub async fn get_oidc_discovery(issuer_url: &str) -> Result<serde_json::Value, HttpError> {
    let url = format!("{}/.well-known/openid-configuration", issuer_url.trim_end_matches('/'));
    let response = Request::get(&url)
        .header("Accept", "application/json")
        .send().await.map_err(|e| make_err(e.to_string()))?;
    if response.status() >= 400 {
        return Err(make_err(format!("HTTP {}", response.status())));
    }
    response.json().await.map_err(|e| make_err(e.to_string()))
}
