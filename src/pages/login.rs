use dioxus::prelude::*;

use crate::api::auth;
use crate::components::ui::button::Button;
use crate::components::ui::loading::Spinner;
use crate::utils::i18n::t;

/// Login page: redirects the user to Pasion for OAuth2 authentication.
#[component]
pub fn LoginPage() -> Element {
    let mut loading = use_signal(|| false);
    let mut ready = use_signal(|| false);

    // Load runtime config on mount (need Pasion public URL for OAuth redirect)
    use_effect(move || {
        spawn(async move {
            let cfg = crate::utils::config::load_runtime_config().await;
            if !cfg.pasion_public_url.is_empty() {
                crate::utils::storage::set_item("pasion_public_url", &cfg.pasion_public_url);
            }
            ready.set(true);
        });
    });

    let handle_login = move |_evt: MouseEvent| {
        loading.set(true);
        spawn(async move {
            auth::start_oauth_login().await;
            loading.set(false);
        });
    };

    let is_ready = *ready.read();
    let is_loading = *loading.read();

    rsx! {
        div { class: "flex min-h-screen items-center justify-center bg-background p-4",
            div { class: "w-full max-w-md space-y-6",
                // Logo & title
                div { class: "text-center space-y-2",
                    div { class: "mx-auto h-16 w-16 rounded-full bg-primary/10 flex items-center justify-center",
                        crate::components::ui::icons::Icon {
                            name: "shield".to_string(),
                            class: "h-8 w-8 text-primary".to_string(),
                        }
                    }
                    h1 { class: "text-2xl font-bold tracking-tight", "Palpo Admin" }
                    p { class: "text-muted-foreground", {t("auth.sign_in_subtitle")} }
                }

                // Login card
                div { class: "rounded-lg border glass-panel p-6 shadow-sm space-y-4",
                    p { class: "text-sm text-center text-muted-foreground",
                        {t("auth.oauth_hint")}
                    }

                    Button {
                        class: "w-full".to_string(),
                        disabled: !is_ready || is_loading,
                        onclick: handle_login,
                        if is_loading {
                            Spinner { class: "mr-2".to_string() }
                        }
                        {t("auth.sign_in")}
                    }
                }

                // Footer
                p { class: "text-center text-xs text-muted-foreground",
                    {t("auth.footer")}
                }
            }
        }
    }
}
