//! Dedicated user detail services extracted from the former UI components.
use crate::api::client::{api_client, build_url};
use crate::types::{AccountDataModel, ExperimentalFeaturesModel, RateLimitsModel};
use crate::utils::{error::HttpError, mxid::return_mxid};

fn user_path(user: &str, suffix: &str) -> Result<String, HttpError> {
    let user = return_mxid(user);
    build_url(
        &format!(
            "/_palpo/admin/v1/users/{}/{suffix}",
            urlencoding::encode(&user)
        ),
        &[],
    )
}
pub async fn account_data(user: &str) -> Result<AccountDataModel, HttpError> {
    api_client(&user_path(user, "accountdata")?, "GET", None).await
}
pub async fn rate_limits(user: &str) -> Result<RateLimitsModel, HttpError> {
    api_client(&user_path(user, "override_ratelimit")?, "GET", None).await
}
pub async fn save_rate_limits(
    user: &str,
    rate: Option<u64>,
    burst: Option<u64>,
) -> Result<(), HttpError> {
    let mut body = serde_json::Map::new();
    if let Some(rate) = rate {
        body.insert("messages_per_second".into(), rate.into());
    }
    if let Some(burst) = burst {
        body.insert("burst_count".into(), burst.into());
    }
    let empty = body.is_empty();
    let _: serde_json::Value = api_client(
        &user_path(user, "override_ratelimit")?,
        if empty { "DELETE" } else { "POST" },
        if empty {
            None
        } else {
            Some(serde_json::Value::Object(body).to_string())
        },
    )
    .await?;
    Ok(())
}
fn feature_path(user: &str) -> Result<String, HttpError> {
    let user = return_mxid(user);
    build_url(
        &format!(
            "/_palpo/admin/v1/experimental_features/{}",
            urlencoding::encode(&user)
        ),
        &[],
    )
}
pub async fn features(user: &str) -> Result<ExperimentalFeaturesModel, HttpError> {
    api_client(&feature_path(user)?, "GET", None).await
}
pub async fn set_feature(user: &str, feature: &str, enabled: bool) -> Result<(), HttpError> {
    let mut features = serde_json::Map::new();
    features.insert(feature.into(), enabled.into());
    let _: serde_json::Value = api_client(
        &feature_path(user)?,
        "PUT",
        Some(serde_json::json!({ "features": features }).to_string()),
    )
    .await?;
    Ok(())
}
