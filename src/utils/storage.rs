#[cfg(not(target_arch = "wasm32"))]
use std::{cell::RefCell, collections::HashMap};
#[cfg(target_arch = "wasm32")]
fn local() -> Option<web_sys::Storage> {
    web_sys::window().and_then(|w| w.local_storage().ok().flatten())
}
#[cfg(target_arch = "wasm32")]
fn session() -> Option<web_sys::Storage> {
    web_sys::window().and_then(|w| w.session_storage().ok().flatten())
}
#[cfg(not(target_arch = "wasm32"))]
thread_local! {static SETTINGS:RefCell<HashMap<String,String>>=RefCell::new(HashMap::new());}
fn session_only(key: &str) -> bool {
    matches!(
        key,
        "access_token"
            | "refresh_token"
            | "access_token_expires_at"
            | "user_id"
            | "user_display_name"
            | "user_avatar_url"
            | "home_server"
    )
}
fn purge_legacy(key: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(storage) = local() {
            let _ = storage.remove_item(key);
        }
        if let Some(storage) = session() {
            let _ = storage.remove_item(key);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = key;
}
pub fn get_item(key: &str) -> Option<String> {
    if session_only(key) {
        purge_legacy(key);
        return if let Some(ctx) = crate::ui::context::current() {
            ctx.secrets.borrow().get(key).cloned()
        } else {
            None
        };
    }
    if let Some(value) =
        crate::ui::context::current().and_then(|ctx| ctx.settings.borrow().get(key).cloned())
    {
        return Some(value);
    }
    #[cfg(target_arch = "wasm32")]
    {
        local()
            .and_then(|storage| storage.get_item(key).ok().flatten())
            .and_then(|value| serde_json::from_str(&value).ok())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        SETTINGS.with(|v| v.borrow().get(key).cloned())
    }
}
pub fn set_item(key: &str, value: &str) {
    if session_only(key) {
        purge_legacy(key);
        if let Some(ctx) = crate::ui::context::current() {
            ctx.secrets.borrow_mut().insert(key.into(), value.into());
        }
        return;
    }
    if let Some(ctx) = crate::ui::context::current() {
        ctx.settings.borrow_mut().insert(key.into(), value.into());
    }
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(storage) = local() {
            if let Ok(value) = serde_json::to_string(value) {
                let _ = storage.set_item(key, &value);
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        SETTINGS.with(|v| {
            v.borrow_mut().insert(key.into(), value.into());
        });
    }
}
pub fn remove_item(key: &str) {
    purge_legacy(key);
    if let Some(ctx) = crate::ui::context::current() {
        ctx.settings.borrow_mut().remove(key);
    }
    if let Some(ctx) = crate::ui::context::current() {
        ctx.secrets.borrow_mut().remove(key);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        SETTINGS.with(|v| {
            v.borrow_mut().remove(key);
        });
    }
}
pub fn clear() {
    if let Some(ctx) = crate::ui::context::current() {
        ctx.settings.borrow_mut().clear();
    }
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(storage) = local() {
            let _ = storage.clear();
        }
        if let Some(storage) = session() {
            let _ = storage.clear();
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        SETTINGS.with(|v| v.borrow_mut().clear());
    }
    if let Some(ctx) = crate::ui::context::current() {
        ctx.secrets.borrow_mut().clear();
    }
}
