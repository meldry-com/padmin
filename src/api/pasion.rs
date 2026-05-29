use serde::de::DeserializeOwned;

use crate::api::client::raw_fetch;
use crate::types::*;
use crate::utils::error::{HttpError, MatrixError};
use crate::utils::storage;

fn get_pasion_url() -> Option<String> {
    storage::get_item("pasion_url")
}

/// Format a Pasion JSON:API error body. Pasion returns
/// `{"errors":[{"title":"..."}]}` rather than a Matrix errcode, so we
/// surface the raw text and key off the HTTP status. 503 is treated
/// specially as "server in maintenance mode".
fn format_pasion_error(status: u16, text: &str) -> (String, Option<MatrixError>) {
    if status == 503 {
        return ("Server is in maintenance mode".to_string(), None);
    }
    (format!("Pasion error ({status}): {text}"), None)
}

async fn pasion_fetch<T: DeserializeOwned>(
    path: &str,
    method: &str,
    body: Option<String>,
) -> Result<T, HttpError> {
    let base = get_pasion_url().ok_or_else(|| HttpError::message("Pasion URL is not configured"))?;
    let url = format!("{base}/api/admin/v1{path}");

    let result = raw_fetch::<T, _>(&url, method, body.clone(), format_pasion_error).await;

    // On any 401, try to refresh the access token and retry once. If
    // refresh fails, the shared handler clears the stored session so
    // AppLayout's `is_authenticated()` heartbeat redirects to /login.
    if let Err(ref err) = result {
        if err.status == 401 && crate::api::auth::handle_unauthorized().await {
            return raw_fetch::<T, _>(&url, method, body, format_pasion_error).await;
        }
    }

    result
}

// ── JSON:API envelope helpers ──────────────────────────────────────────────
//
// Pasion's admin API wraps resources in a JSON:API envelope:
//
//   List:   {"meta":{"count":N},"data":[{"type":"...","id":"...","attributes":{...},"links":{...}}, ...]}
//   Single: {"data":{"type":"...","id":"...","attributes":{...},"links":{...}}}
//
// Frontend types (see `src/types/pasion.rs`) are flat structs, so these
// helpers hoist `id` into `attributes` before deserializing into `T`.
// Use `pasion_fetch` directly for endpoints that don't follow this
// envelope shape (e.g. `/connector-health` returns a plain object).

fn flatten_resource<T: DeserializeOwned>(item: &serde_json::Value) -> Result<T, HttpError> {
    let id = item.get("id").cloned().unwrap_or(serde_json::Value::Null);
    let mut attrs = item
        .get("attributes")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    if let Some(obj) = attrs.as_object_mut() {
        if !id.is_null() {
            obj.insert("id".to_string(), id);
        }
    }
    serde_json::from_value(attrs).map_err(|e| HttpError::message(format!("JSON parse error: {e}")))
}

async fn pasion_fetch_list<T: DeserializeOwned>(
    path: &str,
) -> Result<PasionListResponse<T>, HttpError> {
    let raw: serde_json::Value = pasion_fetch(path, "GET", None).await?;
    let meta: PasionPaginationMeta = raw
        .get("meta")
        .and_then(|m| serde_json::from_value(m.clone()).ok())
        .unwrap_or_default();
    let data_arr = raw
        .get("data")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();
    let mut data = Vec::with_capacity(data_arr.len());
    for item in &data_arr {
        data.push(flatten_resource::<T>(item)?);
    }
    Ok(PasionListResponse { data, meta })
}

async fn pasion_fetch_one<T: DeserializeOwned>(
    path: &str,
    method: &str,
    body: Option<String>,
) -> Result<T, HttpError> {
    let raw: serde_json::Value = pasion_fetch(path, method, body).await?;
    // Single-resource envelope has the resource under `data`; some
    // endpoints return the bare resource (no envelope), so fall back
    // to the raw value in that case.
    let item = raw.get("data").unwrap_or(&raw);
    flatten_resource::<T>(item)
}

// ── Users ──────────────────────────────────────────────────────────────────

pub async fn pasion_get_users(
    page: u64,
    per_page: u64,
) -> Result<PasionListResponse<PasionUser>, HttpError> {
    pasion_fetch_list(&format!("/users?page[number]={page}&page[size]={per_page}")).await
}

pub async fn pasion_get_user(id: &str) -> Result<PasionUser, HttpError> {
    pasion_fetch_one(&format!("/users/{id}"), "GET", None).await
}

pub async fn pasion_create_user(data: serde_json::Value) -> Result<PasionUser, HttpError> {
    pasion_fetch_one("/users", "POST", Some(data.to_string())).await
}

pub async fn pasion_update_user(
    id: &str,
    data: serde_json::Value,
) -> Result<PasionUser, HttpError> {
    // pasion's admin API uses PATCH on /users/{id}
    pasion_fetch_one(&format!("/users/{id}"), "PATCH", Some(data.to_string())).await
}

pub async fn pasion_set_password(id: &str, password: &str) -> Result<(), HttpError> {
    let body = serde_json::json!({ "password": password });
    let _: serde_json::Value = pasion_fetch(
        &format!("/users/{id}/set-password"),
        "POST",
        Some(body.to_string()),
    )
    .await?;
    Ok(())
}

pub async fn pasion_risk_action(id: &str, action: &str) -> Result<(), HttpError> {
    // Pasion exposes a single POST /users/{id}/risk-action endpoint that
    // takes `{action, reason}` in the body. Valid actions are `lock`,
    // `force_password_reset`, and `terminate_sessions`.
    let body = serde_json::json!({ "action": action });
    let _: serde_json::Value = pasion_fetch(
        &format!("/users/{id}/risk-action"),
        "POST",
        Some(body.to_string()),
    )
    .await?;
    Ok(())
}

pub async fn pasion_batch_invite(emails: Vec<String>) -> Result<serde_json::Value, HttpError> {
    let body = serde_json::json!({ "emails": emails });
    pasion_fetch("/users/batch-invite", "POST", Some(body.to_string())).await
}

// ── User Emails ────────────────────────────────────────────────────────────

pub async fn pasion_get_user_emails(
    user_id: Option<&str>,
) -> Result<Vec<PasionUserEmail>, HttpError> {
    let path = if let Some(uid) = user_id {
        format!("/user-emails?filter[user]={uid}")
    } else {
        "/user-emails".to_string()
    };
    Ok(pasion_fetch_list::<PasionUserEmail>(&path).await?.data)
}

pub async fn pasion_add_user_email(
    user_id: &str,
    email: &str,
) -> Result<PasionUserEmail, HttpError> {
    let body = serde_json::json!({ "user_id": user_id, "email": email });
    pasion_fetch_one("/user-emails", "POST", Some(body.to_string())).await
}

pub async fn pasion_update_user_email(
    id: &str,
    data: serde_json::Value,
) -> Result<PasionUserEmail, HttpError> {
    pasion_fetch_one(
        &format!("/user-emails/{id}"),
        "PATCH",
        Some(data.to_string()),
    )
    .await
}

pub async fn pasion_delete_user_email(id: &str) -> Result<(), HttpError> {
    let _: serde_json::Value = pasion_fetch(&format!("/user-emails/{id}"), "DELETE", None).await?;
    Ok(())
}

// ── Browser Sessions ───────────────────────────────────────────────────────
// Pasion exposes these as /user-sessions on the admin side; we keep the
// "browser session" vocabulary in the frontend names because that's what
// users see in the UI.

pub async fn pasion_get_browser_sessions(
    user_id: Option<&str>,
) -> Result<Vec<PasionBrowserSession>, HttpError> {
    let path = if let Some(uid) = user_id {
        format!("/user-sessions?filter[user]={uid}")
    } else {
        "/user-sessions".to_string()
    };
    Ok(pasion_fetch_list::<PasionBrowserSession>(&path).await?.data)
}

pub async fn pasion_finish_browser_session(id: &str) -> Result<(), HttpError> {
    let _: serde_json::Value =
        pasion_fetch(&format!("/user-sessions/{id}/finish"), "POST", None).await?;
    Ok(())
}

// ── OAuth2 Sessions ────────────────────────────────────────────────────────

pub async fn pasion_get_oauth2_sessions(
    user_id: Option<&str>,
) -> Result<Vec<PasionOAuth2Session>, HttpError> {
    let path = if let Some(uid) = user_id {
        format!("/oauth2-sessions?filter[user]={uid}")
    } else {
        "/oauth2-sessions".to_string()
    };
    Ok(pasion_fetch_list::<PasionOAuth2Session>(&path).await?.data)
}

pub async fn pasion_get_oauth2_session(id: &str) -> Result<PasionOAuth2Session, HttpError> {
    pasion_fetch_one(&format!("/oauth2-sessions/{id}"), "GET", None).await
}

pub async fn pasion_finish_oauth2_session(id: &str) -> Result<(), HttpError> {
    let _: serde_json::Value =
        pasion_fetch(&format!("/oauth2-sessions/{id}/finish"), "POST", None).await?;
    Ok(())
}

// ── Personal Sessions (API Tokens) ─────────────────────────────────────────
// Pasion calls these "personal sessions" internally — what users see as a
// personal access token is implemented as a session with a long-lived
// access token attached.

pub async fn pasion_get_personal_sessions() -> Result<Vec<PasionPersonalSession>, HttpError> {
    Ok(
        pasion_fetch_list::<PasionPersonalSession>("/personal-sessions")
            .await?
            .data,
    )
}

pub async fn pasion_create_personal_session(
    data: serde_json::Value,
) -> Result<PasionPersonalSession, HttpError> {
    pasion_fetch_one("/personal-sessions", "POST", Some(data.to_string())).await
}

pub async fn pasion_get_personal_session(id: &str) -> Result<PasionPersonalSession, HttpError> {
    pasion_fetch_one(&format!("/personal-sessions/{id}"), "GET", None).await
}

pub async fn pasion_regenerate_personal_session(
    id: &str,
) -> Result<PasionPersonalSession, HttpError> {
    pasion_fetch_one(&format!("/personal-sessions/{id}/regenerate"), "POST", None).await
}

pub async fn pasion_revoke_personal_session(id: &str) -> Result<(), HttpError> {
    let _: serde_json::Value =
        pasion_fetch(&format!("/personal-sessions/{id}/revoke"), "POST", None).await?;
    Ok(())
}

// ── Registration Tokens ────────────────────────────────────────────────────
// Pasion exposes these as /user-registration-tokens on the admin side.

pub async fn pasion_get_registration_tokens() -> Result<Vec<PasionRegistrationToken>, HttpError> {
    Ok(
        pasion_fetch_list::<PasionRegistrationToken>("/user-registration-tokens")
            .await?
            .data,
    )
}

pub async fn pasion_create_registration_token(
    data: serde_json::Value,
) -> Result<PasionRegistrationToken, HttpError> {
    pasion_fetch_one("/user-registration-tokens", "POST", Some(data.to_string())).await
}

pub async fn pasion_update_registration_token(
    id: &str,
    data: serde_json::Value,
) -> Result<PasionRegistrationToken, HttpError> {
    pasion_fetch_one(
        &format!("/user-registration-tokens/{id}"),
        "PUT",
        Some(data.to_string()),
    )
    .await
}

pub async fn pasion_revoke_registration_token(id: &str) -> Result<(), HttpError> {
    let _: serde_json::Value = pasion_fetch(
        &format!("/user-registration-tokens/{id}/revoke"),
        "POST",
        None,
    )
    .await?;
    Ok(())
}

pub async fn pasion_unrevoke_registration_token(id: &str) -> Result<(), HttpError> {
    let _: serde_json::Value = pasion_fetch(
        &format!("/user-registration-tokens/{id}/unrevoke"),
        "POST",
        None,
    )
    .await?;
    Ok(())
}

// ── Upstream OAuth Providers ───────────────────────────────────────────────

pub async fn pasion_get_upstream_providers() -> Result<Vec<PasionUpstreamProvider>, HttpError> {
    Ok(
        pasion_fetch_list::<PasionUpstreamProvider>("/upstream-oauth-providers")
            .await?
            .data,
    )
}

pub async fn pasion_get_upstream_provider(id: &str) -> Result<PasionUpstreamProvider, HttpError> {
    pasion_fetch_one(&format!("/upstream-oauth-providers/{id}"), "GET", None).await
}

pub async fn pasion_create_upstream_provider(
    data: serde_json::Value,
) -> Result<PasionUpstreamProvider, HttpError> {
    pasion_fetch_one("/upstream-oauth-providers", "POST", Some(data.to_string())).await
}

pub async fn pasion_update_upstream_provider(
    id: &str,
    data: serde_json::Value,
) -> Result<PasionUpstreamProvider, HttpError> {
    pasion_fetch_one(
        &format!("/upstream-oauth-providers/{id}"),
        "PATCH",
        Some(data.to_string()),
    )
    .await
}

pub async fn pasion_delete_upstream_provider(id: &str) -> Result<(), HttpError> {
    let _: serde_json::Value =
        pasion_fetch(&format!("/upstream-oauth-providers/{id}"), "DELETE", None).await?;
    Ok(())
}

pub async fn pasion_disable_upstream_provider(id: &str) -> Result<(), HttpError> {
    let _: serde_json::Value = pasion_fetch(
        &format!("/upstream-oauth-providers/{id}/disable"),
        "POST",
        None,
    )
    .await?;
    Ok(())
}

pub async fn pasion_enable_upstream_provider(id: &str) -> Result<(), HttpError> {
    let _: serde_json::Value = pasion_fetch(
        &format!("/upstream-oauth-providers/{id}/enable"),
        "POST",
        None,
    )
    .await?;
    Ok(())
}

// ── Upstream OAuth Links ───────────────────────────────────────────────────

pub async fn pasion_get_upstream_links(
    user_id: Option<&str>,
    provider_id: Option<&str>,
) -> Result<Vec<PasionUpstreamLink>, HttpError> {
    let mut params = Vec::new();
    if let Some(uid) = user_id {
        params.push(format!("filter[user]={uid}"));
    }
    if let Some(pid) = provider_id {
        params.push(format!("filter[provider]={pid}"));
    }
    let path = if params.is_empty() {
        "/upstream-oauth-links".to_string()
    } else {
        format!("/upstream-oauth-links?{}", params.join("&"))
    };
    Ok(pasion_fetch_list::<PasionUpstreamLink>(&path).await?.data)
}

pub async fn pasion_create_upstream_link(
    data: serde_json::Value,
) -> Result<PasionUpstreamLink, HttpError> {
    pasion_fetch_one("/upstream-oauth-links", "POST", Some(data.to_string())).await
}

pub async fn pasion_delete_upstream_link(id: &str) -> Result<(), HttpError> {
    let _: serde_json::Value =
        pasion_fetch(&format!("/upstream-oauth-links/{id}"), "DELETE", None).await?;
    Ok(())
}

// ── Audit Log ──────────────────────────────────────────────────────────────

pub async fn pasion_get_audit_feed(limit: u64) -> Result<PasionAuditFeedResponse, HttpError> {
    let path = format!("/audit-feed?limit={limit}");
    pasion_fetch(&path, "GET", None).await
}

// ── Notification Channels ──────────────────────────────────────────────────
// Pasion returns `{channels: [{channel, configured}]}` (not JSON:API).

pub async fn pasion_get_notification_channels() -> Result<Vec<PasionNotificationChannel>, HttpError>
{
    let raw: serde_json::Value = pasion_fetch("/notification-channels", "GET", None).await?;
    let channels = raw
        .get("channels")
        .cloned()
        .unwrap_or_else(|| serde_json::Value::Array(Vec::new()));
    serde_json::from_value(channels)
        .map_err(|e| HttpError::message(format!("JSON parse error: {e}")))
}

// ── Notification Templates ─────────────────────────────────────────────────
// Pasion returns `{templates: [{key, description}]}` (not JSON:API).

pub async fn pasion_get_notification_templates()
-> Result<Vec<PasionNotificationTemplate>, HttpError> {
    let raw: serde_json::Value = pasion_fetch("/notification-templates", "GET", None).await?;
    let templates = raw
        .get("templates")
        .cloned()
        .unwrap_or_else(|| serde_json::Value::Array(Vec::new()));
    serde_json::from_value(templates)
        .map_err(|e| HttpError::message(format!("JSON parse error: {e}")))
}

pub async fn pasion_publish_template(data: serde_json::Value) -> Result<(), HttpError> {
    let _: serde_json::Value = pasion_fetch(
        "/notification-templates/publish",
        "POST",
        Some(data.to_string()),
    )
    .await?;
    Ok(())
}

// ── Site Config ────────────────────────────────────────────────────────────

pub async fn pasion_get_site_config() -> Result<serde_json::Value, HttpError> {
    pasion_fetch("/site-config", "GET", None).await
}

// ── Connector Health ───────────────────────────────────────────────────────

pub async fn pasion_get_connector_health() -> Result<Vec<PasionConnectorHealth>, HttpError> {
    // Endpoint lives at /api/admin/v1/connector-health and returns
    // `{providers: [{provider, homeserver, status, error?}]}` — not the
    // JSON:API envelope, so we fetch raw and pick the inner array.
    let raw: serde_json::Value = pasion_fetch("/connector-health", "GET", None).await?;
    let providers = raw
        .get("providers")
        .cloned()
        .unwrap_or_else(|| serde_json::Value::Array(Vec::new()));
    serde_json::from_value(providers)
        .map_err(|e| HttpError::message(format!("JSON parse error: {e}")))
}

// ── Version ────────────────────────────────────────────────────────────────

pub async fn pasion_get_version() -> Result<PasionVersion, HttpError> {
    pasion_fetch("/version", "GET", None).await
}
