use std::sync::Mutex;
use std::sync::OnceLock;

use crate::utils::storage;

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub restrict_base_url: Option<Vec<String>>,
    pub cors_credentials: String,
    pub external_auth_provider: bool,
    pub palpo_admin: Option<String>,
}

static CONFIG: OnceLock<Mutex<Config>> = OnceLock::new();

fn config_mutex() -> &'static Mutex<Config> {
    CONFIG.get_or_init(|| Mutex::new(Config::default()))
}

pub fn get_config() -> Config {
    config_mutex().lock().unwrap().clone()
}

pub fn set_config(config: Config) {
    *config_mutex().lock().unwrap() = config;
}

pub fn clear_config() {
    *config_mutex().lock().unwrap() = Config::default();
    storage::clear();
}

pub fn set_external_auth_provider(value: bool) {
    config_mutex().lock().unwrap().external_auth_provider = value;
}

pub fn get_base_url() -> Option<String> {
    storage::get_item("base_url")
}

pub fn get_home_server() -> Option<String> {
    storage::get_item("home_server")
}

/// Default OAuth client ID baked into the bundle. Used only as a last-resort
/// fallback when neither `config.json` nor `localStorage["oauth_client_id"]`
/// supplies one — this keeps the build self-contained for the canonical
/// pasion deployment shipped with `examples/pasion.yaml`.
pub const DEFAULT_OAUTH_CLIENT_ID: &str = "01KMQPADM1N000000000000000";

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct RuntimeConfig {
    /// Pasion public URL for browser OAuth2 redirects (e.g. http://localhost:7080)
    #[serde(default)]
    pub pasion_public_url: String,
    /// OAuth2 client_id this padmin instance is registered under in pasion.
    /// Defaults to `DEFAULT_OAUTH_CLIENT_ID` when empty.
    #[serde(default)]
    pub oauth_client_id: String,
}

pub async fn load_runtime_config() -> RuntimeConfig {
    match gloo_net::http::Request::get("/config.json").send().await {
        Ok(resp) => resp.json::<RuntimeConfig>().await.unwrap_or_default(),
        Err(_) => RuntimeConfig::default(),
    }
}

/// Get the Pasion public URL. Falls back to same-origin /authorize if not configured.
pub fn get_pasion_public_url() -> Option<String> {
    storage::get_item("pasion_public_url")
}

/// Get the configured OAuth2 client_id, falling back to the bundle default.
pub fn get_oauth_client_id() -> String {
    storage::get_item("oauth_client_id")
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_OAUTH_CLIENT_ID.to_string())
}
