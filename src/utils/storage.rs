use gloo_storage::{LocalStorage, Storage};

pub fn get_item(key: &str) -> Option<String> {
    LocalStorage::get::<String>(key).ok()
}

pub fn set_item(key: &str, value: &str) {
    let _ = LocalStorage::set(key, value.to_string());
}

pub fn remove_item(key: &str) {
    LocalStorage::delete(key);
}

pub fn clear() {
    LocalStorage::clear();
}
