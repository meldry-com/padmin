use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static CACHE: RefCell<HashMap<String, (f64, String)>> = RefCell::new(HashMap::new());
}

pub fn get_cached(key: &str, ttl_ms: f64) -> Option<String> {
    CACHE.with(|c| {
        let cache = c.borrow();
        if let Some((timestamp, value)) = cache.get(key) {
            let now = js_sys::Date::now();
            if now - timestamp < ttl_ms {
                return Some(value.clone());
            }
        }
        None
    })
}

pub fn set_cached(key: &str, value: &str) {
    CACHE.with(|c| {
        let mut cache = c.borrow_mut();
        cache.insert(key.to_string(), (js_sys::Date::now(), value.to_string()));
    });
}

pub fn remove_cached(key: &str) {
    CACHE.with(|c| {
        let mut cache = c.borrow_mut();
        cache.remove(key);
    });
}

pub fn invalidate_cached_prefix(prefix: &str) {
    CACHE.with(|c| {
        let mut cache = c.borrow_mut();
        cache.retain(|key, _| !key.starts_with(prefix));
    });
}
