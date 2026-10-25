use crate::api::client::*;
use crate::types::*;
use crate::utils::cache::{cached, invalidate_cached_prefix};
use crate::utils::error::HttpError;

const MEDIA_LIST_CACHE_TTL_MS: f64 = 30_000.0;
const MEDIA_LIST_CACHE_PREFIX: &str = "media:list:";

fn media_list_cache_key(
    page: u64,
    per_page: u64,
    order_by: &str,
    order: &str,
    search_term: &str,
) -> String {
    let encoded_search = urlencoding::encode(search_term);
    format!("{MEDIA_LIST_CACHE_PREFIX}{page}:{per_page}:{order_by}:{order}:{encoded_search}")
}

pub fn invalidate_media_related_caches() {
    invalidate_cached_prefix(MEDIA_LIST_CACHE_PREFIX);
}

pub async fn get_user_media_statistics(
    page: u64,
    per_page: u64,
    order_by: &str,
    order: &str,
    search_term: &str,
) -> Result<ListResponse<UserMediaStatisticRecord>, HttpError> {
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

    let url = build_url("/_palpo/admin/v1/statistics/users/media", &params)?;
    let response: MediaStatisticsResponse = api_client(&url, "GET", None).await?;

    Ok(ListResponse {
        data: response
            .users
            .into_iter()
            .map(|s| {
                let id = s.user_id.clone();
                UserMediaStatisticRecord { id, statistic: s }
            })
            .collect(),
        total: response.total,
    })
}

pub async fn get_user_media_statistics_cached(
    page: u64,
    per_page: u64,
    order_by: &str,
    order: &str,
    search_term: &str,
) -> Result<ListResponse<UserMediaStatisticRecord>, HttpError> {
    let cache_key = media_list_cache_key(page, per_page, order_by, order, search_term);
    cached(
        &cache_key,
        MEDIA_LIST_CACHE_TTL_MS,
        get_user_media_statistics(page, per_page, order_by, order, search_term),
    )
    .await
}

pub async fn delete_local_media(
    before_ts: u64,
    size_gt: u64,
    keep_profiles: bool,
) -> Result<DeleteMediaResult, HttpError> {
    if keep_profiles {
        return Err(HttpError::message(
            "This server does not support preserving profile media during bulk cleanup. Uncheck Keep profile media only if you intend to include it.",
        ));
    }
    let before = media_integer(before_ts, "Before timestamp")?.to_string();
    let size = media_integer(size_gt, "Minimum size")?.to_string();
    let url = build_url(
        "/_palpo/admin/v1/media/delete",
        &[("before_ts", &before), ("size_gt", &size)],
    )?;
    let result = api_client(&url, "POST", None).await?;
    invalidate_media_related_caches();
    Ok(result)
}

pub async fn purge_remote_media(before_ts: u64) -> Result<DeleteMediaResult, HttpError> {
    let before = media_integer(before_ts, "Before timestamp")?.to_string();
    let url = build_url(
        "/_palpo/admin/v1/purge_media_cache",
        &[("before_ts", &before)],
    )?;
    #[derive(serde::Deserialize)]
    struct Purged {
        deleted: u64,
    }
    let result: Purged = api_client(&url, "POST", None).await?;
    let result = DeleteMediaResult {
        deleted_media: Vec::new(),
        total: result.deleted,
    };
    invalidate_media_related_caches();
    Ok(result)
}

pub async fn delete_media(media_id: &str) -> Result<(), HttpError> {
    let media_id = urlencoding::encode(media_id);
    let home_server = urlencoding::encode(&get_home_server()?).into_owned();
    let url = build_url(
        &format!("/_palpo/admin/v1/media/{home_server}/{media_id}"),
        &[],
    )?;
    let _: serde_json::Value = api_client(&url, "DELETE", None).await?;
    invalidate_media_related_caches();
    Ok(())
}

pub async fn quarantine_media(media_id: &str) -> Result<(), HttpError> {
    let media_id = urlencoding::encode(media_id);
    let home_server = urlencoding::encode(&get_home_server()?).into_owned();
    let url = build_url(
        &format!("/_palpo/admin/v1/media/quarantine/{home_server}/{media_id}"),
        &[],
    )?;
    let _: serde_json::Value = api_client(&url, "POST", None)
        .await
        .map_err(unsupported_media_operation)?;
    invalidate_media_related_caches();
    Ok(())
}

pub async fn protect_media(media_id: &str) -> Result<(), HttpError> {
    let media_id = urlencoding::encode(media_id);
    let url = build_url(&format!("/_palpo/admin/v1/media/protect/{media_id}"), &[])?;
    let _: serde_json::Value = api_client(&url, "POST", None)
        .await
        .map_err(unsupported_media_operation)?;
    invalidate_media_related_caches();
    Ok(())
}

fn media_integer(value: u64, name: &str) -> Result<i64, HttpError> {
    i64::try_from(value)
        .map_err(|_| HttpError::message(format!("{name} exceeds the server integer range")))
}

fn unsupported_media_operation(mut error: HttpError) -> HttpError {
    if matches!(error.status, 404 | 405) {
        error.message = format!(
            "This server does not support this media operation (HTTP {}): {}",
            error.status, error.message
        );
    }
    error
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn media_parameters_fit_the_server_integer_contract() {
        assert_eq!(media_integer(i64::MAX as u64, "size").unwrap(), i64::MAX);
        assert!(media_integer(i64::MAX as u64 + 1, "size").is_err());
        assert!(
            unsupported_media_operation(HttpError::from_status(405, "method not allowed"))
                .message
                .contains("does not support")
        );
        assert_eq!(
            unsupported_media_operation(HttpError::from_status(500, "server failure")).message,
            "server failure"
        );
    }
}
