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
    }

    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        style { {include_str!("./style.css")} }
        router::AppRouter {}
    }
}
