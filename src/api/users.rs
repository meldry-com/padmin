use crate::api::client::*;
use crate::types::*;
use crate::utils::cache::{get_cached, invalidate_cached_prefix, remove_cached, set_cached};
use crate::utils::error::HttpError;
use crate::utils::mxid::return_mxid;

const USER_LIST_CACHE_TTL_MS: f64 = 30_000.0;
const USER_LIST_CACHE_PREFIX: &str = "users:list:";
const DASHBOARD_USER_COUNT_CACHE_KEY: &str = "dashboard_user_count";
const DASHBOARD_ACTIVE_USER_COUNT_CACHE_KEY: &str = "dashboard_active_user_count";

fn user_list_cache_key(
    page: u64,
    per_page: u64,
    order_by: &str,
    order: &str,
    search_term: &str,
) -> String {
    let encoded_search = urlencoding::encode(search_term);
    format!("{USER_LIST_CACHE_PREFIX}{page}:{per_page}:{order_by}:{order}:{encoded_search}")
}

fn invalidate_user_related_caches() {
    invalidate_cached_prefix(USER_LIST_CACHE_PREFIX);
    remove_cached(DASHBOARD_USER_COUNT_CACHE_KEY);
    remove_cached(DASHBOARD_ACTIVE_USER_COUNT_CACHE_KEY);
}

fn map_user(user: User) -> UserRecord {
    // Palpo returns creation_ts in milliseconds (Matrix spec).
    let creation_ts_ms = user.creation_ts;
    let id = return_mxid(&user.name);
    let avatar_src = user.avatar_url.clone();
    UserRecord {
        id,
        avatar_src,
        creation_ts_ms,
        user,
    }
}

pub async fn get_user_count() -> Result<u64, HttpError> {
    let url = build_url("/_palpo/admin/v2/users", &[("limit", "1")])?;
    let response: UsersListResponse = api_client(&url, "GET", None).await?;
    Ok(response.total)
}

pub async fn get_active_user_count() -> Result<u64, HttpError> {
    let url = build_url(
        "/_palpo/admin/v2/users",
        &[
            ("limit", "1"),
            ("guests", "false"),
            ("deactivated", "false"),
        ],
    )?;
    let response: UsersListResponse = api_client(&url, "GET", None).await?;
    Ok(response.total)
}

pub async fn get_users(
    page: u64,
    per_page: u64,
    order_by: &str,
    order: &str,
    search_term: &str,
) -> Result<ListResponse<UserRecord>, HttpError> {
    let from = (page - 1) * per_page;
    let from_str = from.to_string();
    let limit_str = per_page.to_string();
    let dir = get_search_order(order);

    let mut params = vec![
        ("from", from_str.as_str()),
        ("limit", limit_str.as_str()),
        ("order_by", order_by),
        ("dir", dir),
    ];

    if !search_term.is_empty() {
        params.push(("search_term", search_term));
    }

    let url = build_url("/_palpo/admin/v2/users", &params)?;
    let response: UsersListResponse = api_client(&url, "GET", None).await?;

    Ok(ListResponse {
        data: response.users.into_iter().map(map_user).collect(),
        total: response.total,
    })
}

pub async fn get_users_cached(
    page: u64,
    per_page: u64,
    order_by: &str,
    order: &str,
    search_term: &str,
) -> Result<ListResponse<UserRecord>, HttpError> {
    let cache_key = user_list_cache_key(page, per_page, order_by, order, search_term);

    if let Some(cached) = get_cached(&cache_key, USER_LIST_CACHE_TTL_MS) {
        if let Ok(response) = serde_json::from_str::<ListResponse<UserRecord>>(&cached) {
            return Ok(response);
        }
    }

    let response = get_users(page, per_page, order_by, order, search_term).await?;

    if let Ok(serialized) = serde_json::to_string(&response) {
        set_cached(&cache_key, &serialized);
    }

    Ok(response)
}

pub async fn get_user(id: &str) -> Result<UserRecord, HttpError> {
    let encoded = urlencoding::encode(id);
    let url = build_url(&format!("/_palpo/admin/v2/users/{encoded}"), &[])?;
    let user: User = api_client(&url, "GET", None).await?;
    Ok(map_user(user))
}

pub async fn create_user(id: &str, data: CreateUserRequest) -> Result<UserRecord, HttpError> {
    let user_id = return_mxid(id);
    let encoded = urlencoding::encode(&user_id);
    let url = build_url(&format!("/_palpo/admin/v2/users/{encoded}"), &[])?;
    let body = serde_json::to_string(&data).map_err(|e| HttpError {
        message: e.to_string(),
        status: 0,
        body: None,
        request_id: None,
    })?;
    let user: User = api_client(&url, "PUT", Some(body)).await?;
    invalidate_user_related_caches();
    Ok(map_user(user))
}

pub async fn update_user(id: &str, data: serde_json::Value) -> Result<UserRecord, HttpError> {
    let encoded = urlencoding::encode(id);
    let url = build_url(&format!("/_palpo/admin/v2/users/{encoded}"), &[])?;
    let body = serde_json::to_string(&data).map_err(|e| HttpError {
        message: e.to_string(),
        status: 0,
        body: None,
        request_id: None,
    })?;
    let user: User = api_client(&url, "PUT", Some(body)).await?;
    invalidate_user_related_caches();
    Ok(map_user(user))
}

pub async fn deactivate_user(id: &str, erase: bool) -> Result<(), HttpError> {
    let mxid = return_mxid(id);
    let encoded = urlencoding::encode(&mxid);
    let url = build_url(&format!("/_palpo/admin/v1/deactivate/{encoded}"), &[])?;
    let body = serde_json::json!({ "erase": erase }).to_string();
    let _: serde_json::Value = api_client(&url, "POST", Some(body)).await?;
    invalidate_user_related_caches();
    Ok(())
}

pub async fn set_user_deactivated(id: &str, deactivated: bool) -> Result<UserRecord, HttpError> {
    let mxid = return_mxid(id);
    let encoded = urlencoding::encode(&mxid);
    let url = build_url(&format!("/_palpo/admin/v2/users/{encoded}"), &[])?;
    let body = serde_json::json!({ "deactivated": deactivated }).to_string();
    let user: User = api_client(&url, "PUT", Some(body)).await?;
    invalidate_user_related_caches();
    Ok(map_user(user))
}

pub async fn erase_user(id: &str) -> Result<(), HttpError> {
    deactivate_user(id, true).await
}

pub async fn get_user_devices(
    user_id: &str,
    page: u64,
    per_page: u64,
) -> Result<ListResponse<DeviceRecord>, HttpError> {
    let from = (page - 1) * per_page;
    let encoded = urlencoding::encode(user_id);
    let from_str = from.to_string();
    let limit_str = per_page.to_string();
    let url = build_url(
        &format!("/_palpo/admin/v2/users/{encoded}/devices"),
        &[("from", &from_str), ("limit", &limit_str)],
    )?;

    #[derive(serde::Deserialize)]
    struct Resp {
        devices: Vec<Device>,
        total: u64,
    }

    let response: Resp = api_client(&url, "GET", None).await?;

    Ok(ListResponse {
        data: response
            .devices
            .into_iter()
            .map(|d| {
                let id = d.device_id.clone();
                DeviceRecord { id, device: d }
            })
            .collect(),
        total: response.total,
    })
}

pub async fn delete_user_device(user_id: &str, device_id: &str) -> Result<(), HttpError> {
    let encoded_user = urlencoding::encode(user_id);
    let url = build_url(
        &format!("/_palpo/admin/v2/users/{encoded_user}/devices/{device_id}"),
        &[],
    )?;
    let _: serde_json::Value = api_client(&url, "DELETE", None).await?;
    Ok(())
}

pub async fn delete_all_user_devices(user_id: &str) -> Result<u64, HttpError> {
    let devices = get_user_devices(user_id, 1, 10000).await?;
    let mut deleted = 0u64;
    for device in &devices.data {
        if delete_user_device(user_id, &device.id).await.is_ok() {
            deleted += 1;
        }
    }
    Ok(deleted)
}

pub async fn delete_user_threepid(
    user_id: &str,
    medium: &str,
    address: &str,
) -> Result<(), HttpError> {
    // Fetch current user, filter out the target 3PID, PUT back
    let user = get_user(user_id).await?;
    let remaining: Vec<serde_json::Value> = user
        .user
        .threepids
        .iter()
        .filter(|tp| !(tp.medium == medium && tp.address == address))
        .map(|tp| serde_json::json!({ "medium": tp.medium, "address": tp.address }))
        .collect();
    let data = serde_json::json!({ "threepids": remaining });
    update_user(user_id, data).await?;
    Ok(())
}

pub async fn get_user_joined_rooms(
    user_id: &str,
    page: u64,
    per_page: u64,
) -> Result<ListResponse<String>, HttpError> {
    let from = (page - 1) * per_page;
    let encoded = urlencoding::encode(user_id);
    let from_str = from.to_string();
    let limit_str = per_page.to_string();
    let url = build_url(
        &format!("/_palpo/admin/v1/users/{encoded}/joined_rooms"),
        &[("from", &from_str), ("limit", &limit_str)],
    )?;

    #[derive(serde::Deserialize)]
    struct Resp {
        joined_rooms: Vec<String>,
        total: u64,
    }

    let response: Resp = api_client(&url, "GET", None).await?;

    Ok(ListResponse {
        data: response.joined_rooms,
        total: response.total,
    })
}

pub async fn check_username_available(username: &str) -> Result<bool, HttpError> {
    let user_id = return_mxid(username);
    let encoded = urlencoding::encode(&user_id);
    let url = build_url(&format!("/_palpo/admin/v2/users/{encoded}"), &[])?;
    match api_client::<User>(&url, "GET", None).await {
        Ok(_) => Ok(false),                    // user exists => not available
        Err(e) if e.status == 404 => Ok(true), // not found => available
        Err(e) => Err(e),
    }
}

pub async fn reset_password(id: &str, new_password: &str) -> Result<(), HttpError> {
    let encoded = urlencoding::encode(id);
    let url = build_url(&format!("/_palpo/admin/v1/reset_password/{encoded}"), &[])?;
    let body = serde_json::json!({
        "new_password": new_password,
        "logout_devices": true,
    })
    .to_string();
    let _: serde_json::Value = api_client(&url, "POST", Some(body)).await?;
    Ok(())
}

pub async fn whois_user(user_id: &str) -> Result<crate::types::Whois, HttpError> {
    let encoded = urlencoding::encode(user_id);
    let url = build_url(&format!("/_palpo/admin/v1/whois/{encoded}"), &[])?;
    let whois: crate::types::Whois = api_client(&url, "GET", None).await?;
    Ok(whois)
}

pub async fn send_server_notice(user_id: &str, body: &str) -> Result<String, HttpError> {
    let mxid = return_mxid(user_id);
    let url = build_url("/_palpo/admin/v1/send_server_notice", &[])?;
    let req_body = serde_json::json!({
        "user_id": mxid,
        "content": {
            "msgtype": "m.text",
            "body": body,
        }
    })
    .to_string();

    #[derive(serde::Deserialize)]
    struct Resp {
        event_id: String,
    }

    let response: Resp = api_client(&url, "POST", Some(req_body)).await?;
    Ok(response.event_id)
}
