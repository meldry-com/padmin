mod api;
mod components;
mod pages;
mod router;
mod types;
mod utils;

use dioxus::prelude::*;

fn main() {
    dioxus_logger::init(dioxus_logger::tracing::Level::INFO).expect("failed to init logger");

    components::theme::apply_theme();

    // Pasion admin API is served through the same-origin Nginx proxy at
    // /api/admin/. Pin `pasion_url` to the current window origin so all
    // Pasion admin features (upstream OIDC providers, sessions, audit log,
    // etc.) are reachable and their sidebar entries become visible on every
    // load — not only on the login page.
    if let Some(origin) = web_sys::window().and_then(|w| w.location().origin().ok()) {
        utils::storage::set_item("pasion_url", &origin);

        // The palpo_admin sidecar (server status, scheduled commands, payments,
        // instance config, …) is reachable through the same-origin Nginx proxy
        // under `/_palpo/admin/v1`. Pin `palpo_admin_url` here — the same way
        // `pasion_url` is pinned to the origin — so `is_palpo_admin_enabled()`
        // is true and the Server Ops sidebar group + instance_config fetch
        // become reachable on every load. A non-empty `palpo_admin_url` in
        // `config.json` overrides this default (handled below).
        utils::storage::set_item("palpo_admin_url", &format!("{origin}/_palpo/admin/v1"));
    }

    // Override the same-origin default with an explicit `palpo_admin_url` from
    // runtime config when one is supplied (mirrors how login.rs applies
    // pasion_public_url / oauth_client_id from config.json).
    wasm_bindgen_futures::spawn_local(async {
        let cfg = utils::config::load_runtime_config().await;
        if !cfg.palpo_admin_url.trim().is_empty() {
            utils::storage::set_item("palpo_admin_url", cfg.palpo_admin_url.trim());
        }
    });

    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        style { {include_str!("./style.css")} }
        router::AppRouter {}
    }
}
