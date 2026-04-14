use gloo_net::http::Request;
use serde::de::DeserializeOwned;

use crate::types::*;
use crate::utils::error::HttpError;
use crate::utils::storage;

async fn palpo_admin_fetch<T: DeserializeOwned>(
    palpo_admin_url: &str,
    path: &str,
    method: &str,
    body: Option<String>,
) -> Result<T, HttpError> {
    let url = format!("{palpo_admin_url}{path}");
    let token = storage::get_item("access_token");

    let mut builder = match method {
        "POST" => Request::post(&url),
        "PUT" => Request::put(&url),
        "DELETE" => Request::delete(&url),
        _ => Request::get(&url),
    }
    .header("Accept", "application/json");

    if let Some(ref token) = token {
        builder = builder.header("Authorization", &format!("Bearer {token}"));
    }

    if body.is_some() {
        builder = builder.header("Content-Type", "application/json");
    }

    let request = if let Some(body) = body {
        builder.body(body)
    } else {
        builder.build()
    }
    .map_err(|e| HttpError {
        message: e.to_string(),
        status: 0,
        body: None,
        request_id: None,
    })?;

    let response = request.send().await.map_err(|e| HttpError {
        message: e.to_string(),
        status: 0,
        body: None,
        request_id: None,
    })?;

    let status = response.status();

    if status == 503 {
        return Err(HttpError {
            message: "Server is in maintenance mode".to_string(),
            status: 503,
            body: None,
            request_id: None,
        });
    }

    if status == 204 {
        return serde_json::from_str::<T>("{}")
            .or_else(|_| serde_json::from_str::<T>("null"))
            .map_err(|e| HttpError {
                message: e.to_string(),
                status,
                body: None,
                request_id: None,
            });
    }

    let text = response.text().await.map_err(|e| HttpError {
        message: e.to_string(),
        status,
        body: None,
        request_id: None,
    })?;

    if status >= 400 {
        return Err(HttpError {
            message: format!("Palpo admin error ({status}): {text}"),
            status,
            body: None,
            request_id: None,
        });
    }

    serde_json::from_str(&text).map_err(|e| HttpError {
        message: format!("JSON parse error: {e}"),
        status,
        body: None,
        request_id: None,
    })
}

pub async fn get_server_status(palpo_admin_url: &str) -> Result<ServerStatusResponse, HttpError> {
    palpo_admin_fetch(palpo_admin_url, "/status", "GET", None).await
}

pub async fn get_server_running_process(
    palpo_admin_url: &str,
) -> Result<ServerProcessResponse, HttpError> {
    palpo_admin_fetch(palpo_admin_url, "/lock", "GET", None).await
}

pub async fn get_server_notifications(
    palpo_admin_url: &str,
) -> Result<ServerNotificationsResponse, HttpError> {
    palpo_admin_fetch(palpo_admin_url, "/notifications", "GET", None).await
}

pub async fn delete_server_notifications(palpo_admin_url: &str) -> Result<(), HttpError> {
    let _: serde_json::Value =
        palpo_admin_fetch(palpo_admin_url, "/notifications", "DELETE", None).await?;
    Ok(())
}

pub async fn get_server_commands(
    palpo_admin_url: &str,
) -> Result<std::collections::HashMap<String, ServerCommand>, HttpError> {
    palpo_admin_fetch(palpo_admin_url, "/commands", "GET", None).await
}

pub async fn run_server_command(
    palpo_admin_url: &str,
    command: &str,
    additional_args: Option<serde_json::Value>,
) -> Result<serde_json::Value, HttpError> {
    let mut body = serde_json::json!({ "command": command });
    if let Some(args) = additional_args {
        body["additionalArgs"] = args;
    }
    palpo_admin_fetch(palpo_admin_url, "/commands", "POST", Some(body.to_string())).await
}

pub async fn get_scheduled_commands(
    palpo_admin_url: &str,
) -> Result<Vec<ScheduledCommand>, HttpError> {
    palpo_admin_fetch(palpo_admin_url, "/schedules", "GET", None).await
}

pub async fn create_scheduled_command(
    palpo_admin_url: &str,
    command: &ScheduledCommand,
) -> Result<ScheduledCommand, HttpError> {
    let body = serde_json::to_string(command).unwrap_or_default();
    palpo_admin_fetch(palpo_admin_url, "/schedules", "POST", Some(body)).await
}

pub async fn update_scheduled_command(
    palpo_admin_url: &str,
    command: &ScheduledCommand,
) -> Result<ScheduledCommand, HttpError> {
    let body = serde_json::to_string(command).unwrap_or_default();
    palpo_admin_fetch(palpo_admin_url, "/schedules", "PUT", Some(body)).await
}

pub async fn delete_scheduled_command(palpo_admin_url: &str, id: &str) -> Result<(), HttpError> {
    let _: serde_json::Value =
        palpo_admin_fetch(palpo_admin_url, &format!("/schedules/{id}"), "DELETE", None).await?;
    Ok(())
}

pub async fn get_recurring_commands(
    palpo_admin_url: &str,
) -> Result<Vec<RecurringCommand>, HttpError> {
    palpo_admin_fetch(palpo_admin_url, "/recurrings", "GET", None).await
}

pub async fn create_recurring_command(
    palpo_admin_url: &str,
    command: &RecurringCommand,
) -> Result<RecurringCommand, HttpError> {
    let body = serde_json::to_string(command).unwrap_or_default();
    palpo_admin_fetch(palpo_admin_url, "/recurrings", "POST", Some(body)).await
}

pub async fn update_recurring_command(
    palpo_admin_url: &str,
    command: &RecurringCommand,
) -> Result<RecurringCommand, HttpError> {
    let body = serde_json::to_string(command).unwrap_or_default();
    palpo_admin_fetch(palpo_admin_url, "/recurrings", "PUT", Some(body)).await
}

pub async fn delete_recurring_command(palpo_admin_url: &str, id: &str) -> Result<(), HttpError> {
    let _: serde_json::Value = palpo_admin_fetch(
        palpo_admin_url,
        &format!("/recurrings/{id}"),
        "DELETE",
        None,
    )
    .await?;
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct PaymentsResponse {
    #[serde(default)]
    pub payments: Vec<serde_json::Value>,
    #[serde(default)]
    pub total: u64,
    #[serde(default)]
    pub maintenance: bool,
    #[serde(default)]
    pub subscription: Option<serde_json::Value>,
    #[serde(default)]
    pub payment_method: Option<String>,
    #[serde(default)]
    pub invoices: Vec<serde_json::Value>,
}

pub async fn get_payments(palpo_admin_url: &str) -> Result<PaymentsResponse, HttpError> {
    palpo_admin_fetch(palpo_admin_url, "/payments", "GET", None).await
}
