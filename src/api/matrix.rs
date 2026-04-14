use gloo_net::http::Request;

use crate::types::*;
use crate::utils::error::HttpError;

pub fn is_valid_base_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

pub fn split_mxid(mxid: &str) -> Option<(String, String)> {
    let mxid = mxid.strip_prefix('@')?;
    let parts: Vec<&str> = mxid.splitn(2, ':').collect();
    if parts.len() == 2 {
        Some((parts[0].to_string(), parts[1].to_string()))
    } else {
        None
    }
}

pub async fn get_well_known_url(domain: &str) -> Result<String, HttpError> {
    let url = format!("https://{domain}/.well-known/matrix/client");

    let response = Request::get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| HttpError {
            message: e.to_string(),
            status: 0,
            body: None,
            request_id: None,
        })?;

    let json: serde_json::Value = response.json().await.map_err(|e| HttpError {
        message: e.to_string(),
        status: 0,
        body: None,
        request_id: None,
    })?;

    json["m.homeserver"]["base_url"]
        .as_str()
        .map(|s| s.trim_end_matches('/').to_string())
        .ok_or_else(|| HttpError {
            message: "No base_url in well-known".to_string(),
            status: 0,
            body: None,
            request_id: None,
        })
}

pub async fn get_server_version(base_url: &str) -> Result<String, HttpError> {
    let url = format!("{base_url}/_palpo/admin/v1/server_version");

    let response = Request::get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| HttpError {
            message: e.to_string(),
            status: 0,
            body: None,
            request_id: None,
        })?;

    let resp: ServerVersionResponse = response.json().await.map_err(|e| HttpError {
        message: e.to_string(),
        status: 0,
        body: None,
        request_id: None,
    })?;

    Ok(resp.server_version)
}

pub async fn get_supported_features(base_url: &str) -> Result<SupportedFeatures, HttpError> {
    let url = format!("{base_url}/_matrix/client/versions");

    let response = Request::get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| HttpError {
            message: e.to_string(),
            status: 0,
            body: None,
            request_id: None,
        })?;

    response.json().await.map_err(|e| HttpError {
        message: e.to_string(),
        status: 0,
        body: None,
        request_id: None,
    })
}

pub async fn get_supported_login_flows(base_url: &str) -> Result<Vec<LoginFlow>, HttpError> {
    let url = format!("{base_url}/_matrix/client/v3/login");

    let response = Request::get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| HttpError {
            message: e.to_string(),
            status: 0,
            body: None,
            request_id: None,
        })?;

    let flows_resp: LoginFlowsResponse = response.json().await.map_err(|e| HttpError {
        message: e.to_string(),
        status: 0,
        body: None,
        request_id: None,
    })?;

    Ok(flows_resp.flows)
}

pub async fn refresh_access_token() -> Result<bool, HttpError> {
    let refresh_token = crate::utils::storage::get_item("refresh_token");
    let base_url = crate::utils::storage::get_item("base_url");

    let (refresh_token, base_url) = match (refresh_token, base_url) {
        (Some(rt), Some(bu)) => (rt, bu),
        _ => return Ok(false),
    };

    let url = format!("{base_url}/_matrix/client/v3/refresh");
    let body = serde_json::json!({ "refresh_token": refresh_token }).to_string();

    let response = Request::post(&url)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .body(body)
        .map_err(|e| HttpError {
            message: e.to_string(),
            status: 0,
            body: None,
            request_id: None,
        })?
        .send()
        .await
        .map_err(|e| HttpError {
            message: e.to_string(),
            status: 0,
            body: None,
            request_id: None,
        })?;

    if response.status() >= 400 {
        return Ok(false);
    }

    #[derive(serde::Deserialize)]
    struct RefreshResponse {
        access_token: String,
        #[serde(default)]
        refresh_token: Option<String>,
        #[serde(default)]
        expires_in_ms: Option<u64>,
    }

    let resp: RefreshResponse = response.json().await.map_err(|e| HttpError {
        message: e.to_string(),
        status: 0,
        body: None,
        request_id: None,
    })?;

    crate::utils::storage::set_item("access_token", &resp.access_token);
    if let Some(rt) = resp.refresh_token {
        crate::utils::storage::set_item("refresh_token", &rt);
    }
    if let Some(expires_in) = resp.expires_in_ms {
        let expires_at = js_sys::Date::now() as u64 + expires_in;
        crate::utils::storage::set_item("access_token_expires_at", &expires_at.to_string());
    }

    Ok(true)
}
