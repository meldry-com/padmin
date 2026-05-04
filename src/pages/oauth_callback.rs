use dioxus::prelude::*;

use crate::api::auth;
use crate::components::ui::button::{Button, ButtonVariant};
use crate::components::ui::loading::Spinner;
use crate::router::Route;
use crate::utils::i18n::t;

/// OAuth2 callback page: exchanges the authorization code for tokens,
/// verifies admin status, and redirects to the dashboard.
#[component]
pub fn OAuthCallback(
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
) -> Element {
    let nav = use_navigator();
    let mut error_msg = use_signal(|| Option::<String>::None);
    let mut processing = use_signal(|| true);

    // Process the OAuth callback parameters
    use_effect(move || {
        let code = code.clone();
        let state = state.clone();
        let error = error.clone();
        let error_description = error_description.clone();

        spawn(async move {
            if let Some(err) = error {
                let desc = error_description.unwrap_or_else(|| err.clone());
                processing.set(false);
                error_msg.set(Some(desc));
                return;
            }

            let Some(code) = code else {
                processing.set(false);
                error_msg.set(Some("No authorization code received".into()));
                return;
            };

            // Step 1: Exchange code for tokens (validates `state` against the
            // value stashed in sessionStorage by start_oauth_login).
            if let Err(e) = auth::handle_oauth_callback(&code, state.as_deref()).await {
                processing.set(false);
                error_msg.set(Some(e.message));
                return;
            }

            // Step 2: Best-effort admin check. We always navigate to the
            // dashboard so the user lands somewhere recognizable; individual
            // admin pages will surface their own permission errors if the
            // user isn't actually an admin. This avoids dead-ending users
            // who just logged in via an upstream provider and aren't yet a
            // Matrix admin.
            let _ = auth::verify_admin().await;
            processing.set(false);
            nav.replace(Route::Dashboard {});
        });
    });

    let is_processing = *processing.read();

    rsx! {
        div { class: "flex min-h-screen items-center justify-center bg-background p-4",
            div { class: "w-full max-w-md space-y-6 text-center",
                div { class: "mx-auto h-16 w-16 rounded-full bg-primary/10 flex items-center justify-center",
                    crate::components::ui::icons::Icon {
                        name: "shield".to_string(),
                        class: "h-8 w-8 text-primary".to_string(),
                    }
                }

                if is_processing {
                    div { class: "space-y-3",
                        Spinner { class: "mx-auto".to_string() }
                        p { class: "text-muted-foreground", {t("auth.processing")} }
                    }
                }

                if let Some(ref err) = *error_msg.read() {
                    div { class: "rounded-lg border glass-panel p-6 shadow-sm space-y-4",
                        div { class: "rounded-md bg-destructive/10 p-3 text-sm text-destructive",
                            "{err}"
                        }
                        Button {
                            variant: ButtonVariant::Outline,
                            class: "w-full".to_string(),
                            onclick: move |_evt: MouseEvent| { nav.push(Route::LoginPage {}); },
                            {t("auth.try_again")}
                        }
                    }
                }
            }
        }
    }
}
