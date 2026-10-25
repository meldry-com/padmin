use serde::{Serialize, de::DeserializeOwned};
use std::future::Future;
const MAX_ENTRIES: usize = 128;
const MAX_BYTES: usize = 8 * 1024 * 1024;
fn now() -> f64 {
    chrono::Utc::now().timestamp_millis() as f64
}
pub fn get_cached(key: &str, ttl_ms: f64) -> Option<String> {
    let ctx = crate::ui::context::current()?;
    let mut cache = ctx.cache.borrow_mut();
    if cache
        .get(key)
        .is_some_and(|(time, _)| now() - *time >= ttl_ms)
    {
        cache.remove(key);
        return None;
    }
    cache.get(key).map(|(_, value)| value.clone())
}
pub fn set_cached(key: &str, value: &str) {
    if key.len() + value.len() > MAX_BYTES {
        return;
    }
    let Some(ctx) = crate::ui::context::current() else {
        return;
    };
    let mut cache = ctx.cache.borrow_mut();
    cache.insert(key.to_owned(), (now(), value.to_owned()));
    while cache.len() > MAX_ENTRIES
        || cache
            .iter()
            .map(|(key, (_, value))| key.len() + value.len())
            .sum::<usize>()
            > MAX_BYTES
    {
        let Some(oldest) = cache
            .iter()
            .min_by(|a, b| a.1.0.total_cmp(&b.1.0))
            .map(|(key, _)| key.clone())
        else {
            break;
        };
        cache.remove(&oldest);
    }
}
pub fn remove_cached(key: &str) {
    if let Some(ctx) = crate::ui::context::current() {
        ctx.cache.borrow_mut().remove(key);
    }
}
pub fn invalidate_cached_prefix(prefix: &str) {
    if let Some(ctx) = crate::ui::context::current() {
        ctx.cache
            .borrow_mut()
            .retain(|key, _| !key.starts_with(prefix));
    }
}

/// Read-through cache for an async fetch.
///
/// Returns the cached value if it exists and is younger than `ttl_ms`.
/// Otherwise awaits `fetch`, stores its successful result under `key`, and
/// returns it. Serialization/deserialization failures fall through to the
/// network so a corrupt cache entry can never wedge a caller.
pub async fn cached<T, E, Fut>(key: &str, ttl_ms: f64, fetch: Fut) -> Result<T, E>
where
    T: Serialize + DeserializeOwned,
    Fut: Future<Output = Result<T, E>>,
{
    if let Some(hit) = get_cached(key, ttl_ms) {
        if let Ok(value) = serde_json::from_str::<T>(&hit) {
            return Ok(value);
        }
    }

    let value = fetch.await?;

    if let Ok(serialized) = serde_json::to_string(&value) {
        set_cached(key, &serialized);
    }

    Ok(value)
}
