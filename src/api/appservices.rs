//! Palpo admin appservice management.
//!
//! Wraps the `/_palpo/admin/v1/appservices` endpoints added in palpo PR #119.
//! These endpoints are mounted under the homeserver itself (not the separate
//! palpo_admin sidecar), so we use the standard `api_client` which targets the
//! same-origin Nginx proxy and reuses the user's Matrix access token.

use crate::api::client::api_client;
use crate::types::{AppserviceRegistration, AppserviceSummary, ListAppservicesResponse};
use crate::utils::error::HttpError;

const BASE: &str = "/_palpo/admin/v1/appservices";

pub async fn list_appservices() -> Result<Vec<AppserviceSummary>, HttpError> {
    let resp: ListAppservicesResponse = api_client(BASE, "GET", None).await?;
    Ok(resp.appservices)
}

pub async fn get_appservice(id: &str) -> Result<AppserviceRegistration, HttpError> {
    let path = format!("{BASE}/{}", urlencoding::encode(id));
    api_client(&path, "GET", None).await
}

pub async fn register_appservice(
    reg: &AppserviceRegistration,
) -> Result<serde_json::Value, HttpError> {
    // This catches exact duplicate claims observed in the current registry.
    // It is not atomic with POST and does not infer arbitrary regex overlap;
    // concurrent writers still require server-side namespace enforcement.
    use futures::stream::{self, StreamExt};
    let installed = list_appservices().await?;
    if installed.iter().any(|item| item.id == reg.id) {
        return Err(HttpError::message(
            "An appservice with this ID already exists",
        ));
    }
    let mut reads = stream::iter(installed.into_iter())
        .map(|item| async move { get_appservice(&item.id).await })
        .buffer_unordered(8);
    while let Some(result) = reads.next().await {
        let existing = result?;
        if let Some(section) = exact_namespace_conflict(&reg.namespaces, &existing.namespaces) {
            return Err(HttpError::message(format!(
                "Namespace conflict: identical {section} expression is claimed by appservice {} (current registry precheck; concurrent server updates are not locked)",
                existing.id
            )));
        }
    }
    let body = serde_json::to_string(reg)
        .map_err(|e| HttpError::message(format!("Failed to serialize registration: {e}")))?;
    api_client(BASE, "POST", Some(body)).await
}

/// Change only a callback URL, comparing the value observed when editing began.
/// The server preserves tokens, namespaces, protocols and enabled state atomically.
pub async fn update_appservice_url(
    id: &str,
    url: &str,
    expected_url: Option<&str>,
) -> Result<serde_json::Value, HttpError> {
    let path = format!("{BASE}/{}/url", urlencoding::encode(id));
    let body = serde_json::json!({"url":url, "expected_url":expected_url}).to_string();
    api_client(&path, "PUT", Some(body)).await
}

pub async fn delete_appservice(id: &str) -> Result<(), HttpError> {
    let path = format!("{BASE}/{}", urlencoding::encode(id));
    let _: serde_json::Value = api_client(&path, "DELETE", None).await?;
    Ok(())
}

pub async fn disable_appservice(id: &str) -> Result<(), HttpError> {
    let path = format!("{BASE}/{}/disable", urlencoding::encode(id));
    let _: serde_json::Value = api_client(&path, "POST", None).await?;
    Ok(())
}

pub async fn enable_appservice(id: &str) -> Result<(), HttpError> {
    let path = format!("{BASE}/{}/enable", urlencoding::encode(id));
    let _: serde_json::Value = api_client(&path, "POST", None).await?;
    Ok(())
}

fn exact_namespace_conflict(
    proposed: &serde_json::Value,
    existing: &serde_json::Value,
) -> Option<&'static str> {
    for section in ["users", "aliases", "rooms"] {
        let Some(new) = proposed.get(section).and_then(|v| v.as_array()) else {
            continue;
        };
        let Some(old) = existing.get(section).and_then(|v| v.as_array()) else {
            continue;
        };
        for left in new {
            let Some(pattern) = left.get("regex").and_then(|v| v.as_str()) else {
                continue;
            };
            for right in old {
                if right.get("regex").and_then(|v| v.as_str()) == Some(pattern)
                    && (left.get("exclusive").and_then(|v| v.as_bool()) == Some(true)
                        || right.get("exclusive").and_then(|v| v.as_bool()) == Some(true))
                {
                    return Some(section);
                }
            }
        }
    }
    None
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identical_namespace_claims_respect_category_and_either_exclusive_flag() {
        for section in ["users", "aliases", "rooms"] {
            for (left, right, conflict) in [
                (false, false, false),
                (true, false, true),
                (false, true, true),
                (true, true, true),
            ] {
                let mut a = serde_json::json!({});
                let mut b = serde_json::json!({});
                a[section] = serde_json::json!([{"regex":"same","exclusive":left}]);
                b[section] = serde_json::json!([{"regex":"same","exclusive":right}]);
                assert_eq!(exact_namespace_conflict(&a, &b).is_some(), conflict);
                b[section][0]["regex"] = serde_json::json!("different");
                assert!(exact_namespace_conflict(&a, &b).is_none());
            }
        }
        assert!(
            exact_namespace_conflict(
                &serde_json::json!({"users":[{"regex":"same","exclusive":true}]}),
                &serde_json::json!({"aliases":[{"regex":"same","exclusive":true}]})
            )
            .is_none()
        );
    }
}
