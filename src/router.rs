use dioxus::prelude::*;

use crate::api::auth;
use crate::components::layout::AppLayout;
use crate::pages;
use crate::utils::i18n::t;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[route("/login")]
    LoginPage {},

    #[route("/oauth/callback?:code&:state&:error&:error_description")]
    OAuthCallback {
        code: Option<String>,
        state: Option<String>,
        error: Option<String>,
        error_description: Option<String>,
    },

    #[layout(AuthenticatedLayout)]
        #[route("/")]
        Dashboard {},

        #[route("/users")]
        UserList {},
        #[route("/users/create")]
        UserCreate {},
        #[route("/users/:user_id")]
        UserShow { user_id: String },

        #[route("/rooms")]
        RoomList {},
        #[route("/rooms/:room_id")]
        RoomShow { room_id: String },

        #[route("/media")]
        MediaList {},

        #[route("/reports")]
        ReportList {},
        #[route("/reports/:report_id")]
        ReportShow { report_id: String },

        #[route("/destinations")]
        DestinationList {},
        #[route("/destinations/:destination_id")]
        DestinationShow { destination_id: String },

        #[route("/registration-tokens")]
        RegistrationTokenList {},

        #[route("/appservices")]
        AppserviceList {},

        #[route("/auth-status")]
        AuthStatus {},

        #[route("/server-status")]
        ServerStatus {},

        #[route("/server-actions")]
        ServerActions {},

        #[route("/server-notices")]
        ServerNotices {},

        #[route("/server-notifications")]
        ServerNotifications {},

        #[route("/billing")]
        Billing {},

        #[route("/settings/notifications")]
        NotificationPreferences {},

        #[route("/pasion/audit-log")]
        PasionAuditLog {},
        #[route("/pasion/oauth2-sessions")]
        PasionOAuth2Sessions {},
        #[route("/pasion/personal-sessions")]
        PasionPersonalSessions {},
        #[route("/pasion/connector-health")]
        PasionConnectorHealth {},
        #[route("/pasion/upstream-providers")]
        PasionUpstreamProviders {},
        #[route("/pasion/upstream-links")]
        PasionUpstreamLinks {},
        #[route("/pasion/notification-channels")]
        PasionNotificationChannels {},
        #[route("/pasion/notification-templates")]
        PasionNotificationTemplates {},
    #[end_layout]

    #[route("/:..route")]
    NotFound { route: Vec<String> },
}

#[component]
pub fn AppRouter() -> Element {
    rsx! {
        Router::<Route> {}
    }
}

#[component]
fn AuthenticatedLayout() -> Element {
    let nav = use_navigator();

    if !auth::is_authenticated() {
        nav.replace(Route::LoginPage {});
        return rsx! {
            div { "Redirecting..." }
        };
    }

    // Tri-state admin verdict:
    //   Some(true)  = admin, render the normal dashboard layout
    //   Some(false) = authenticated but forbidden, render NotAuthorizedPage
    //   None        = probe still in flight or inconclusive, show a spinner
    //
    // Seed from the cached value written by the previous successful probe
    // so refreshes don't flash the spinner, then re-verify with
    // `verify_admin()` each mount to stay in sync if admin status changes
    // server-side.
    let cached = auth::cached_is_admin();
    let admin_probe = use_resource(move || async move {
        match auth::verify_admin().await {
            Ok(flag) => Some(flag),
            // Probe errored (401, network, ...). If we have a cached
            // verdict, stick with it; otherwise fall through to "assume
            // admin" so we don't dead-end the user on a spinner forever.
            // Individual admin pages still surface their own errors when
            // the token turns out to be bad.
            Err(_) => cached.or(Some(true)),
        }
    });

    let verdict = match admin_probe.read().as_ref() {
        Some(Some(v)) => Some(*v),
        Some(None) | None => cached,
    };

    match verdict {
        Some(true) => rsx! {
            AppLayout {
                Outlet::<Route> {}
            }
        },
        Some(false) => rsx! {
            pages::not_authorized::NotAuthorizedPage {}
        },
        None => rsx! {
            div { class: "flex min-h-screen items-center justify-center",
                crate::components::ui::loading::Spinner { class: String::new() }
            }
        },
    }
}

#[component]
fn LoginPage() -> Element {
    rsx! {
        pages::login::LoginPage {}
    }
}

#[component]
fn OAuthCallback(
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
) -> Element {
    rsx! {
        pages::oauth_callback::OAuthCallback {
            code: code,
            state: state,
            error: error,
            error_description: error_description,
        }
    }
}

#[component]
fn Dashboard() -> Element {
    rsx! {
        pages::dashboard::Dashboard {}
    }
}

#[component]
fn UserList() -> Element {
    rsx! {
        pages::users::list::UserList {}
    }
}

#[component]
fn UserCreate() -> Element {
    rsx! {
        pages::users::create::UserCreate {}
    }
}

#[component]
fn UserShow(user_id: String) -> Element {
    rsx! {
        pages::users::show::UserShow { user_id }
    }
}

#[component]
fn RoomList() -> Element {
    rsx! {
        pages::rooms::list::RoomList {}
    }
}

#[component]
fn RoomShow(room_id: String) -> Element {
    rsx! {
        pages::rooms::show::RoomShow { room_id }
    }
}

#[component]
fn MediaList() -> Element {
    rsx! {
        pages::media::MediaList {}
    }
}

#[component]
fn ReportList() -> Element {
    rsx! {
        pages::reports::ReportList {}
    }
}

#[component]
fn ReportShow(report_id: String) -> Element {
    rsx! {
        pages::reports::ReportShow { report_id }
    }
}

#[component]
fn DestinationList() -> Element {
    rsx! {
        pages::destinations::DestinationList {}
    }
}

#[component]
fn DestinationShow(destination_id: String) -> Element {
    rsx! {
        pages::destinations::DestinationShow { destination_id }
    }
}

#[component]
fn RegistrationTokenList() -> Element {
    rsx! {
        pages::registration_tokens::RegistrationTokenList {}
    }
}

#[component]
fn AppserviceList() -> Element {
    rsx! {
        pages::appservices::AppserviceList {}
    }
}

#[component]
fn AuthStatus() -> Element {
    rsx! {
        pages::auth_status::AuthStatusPage {}
    }
}

#[component]
fn ServerStatus() -> Element {
    rsx! {
        pages::server_status::ServerStatus {}
    }
}

#[component]
fn ServerActions() -> Element {
    rsx! {
        pages::server_actions::ServerActions {}
    }
}

#[component]
fn ServerNotices() -> Element {
    rsx! {
        pages::server_notices::ServerNotices {}
    }
}

#[component]
fn ServerNotifications() -> Element {
    rsx! {
        pages::server_notifications::ServerNotifications {}
    }
}

#[component]
fn Billing() -> Element {
    rsx! {
        pages::billing::Billing {}
    }
}

#[component]
fn NotificationPreferences() -> Element {
    rsx! {
        pages::notification_preferences::NotificationPreferencesPage {}
    }
}

#[component]
fn PasionAuditLog() -> Element {
    rsx! { pages::pasion::audit_log::AuditLogPage {} }
}

#[component]
fn PasionOAuth2Sessions() -> Element {
    rsx! { pages::pasion::oauth2_sessions::OAuth2SessionsPage {} }
}

#[component]
fn PasionPersonalSessions() -> Element {
    rsx! { pages::pasion::personal_sessions::PersonalSessionsPage {} }
}

#[component]
fn PasionConnectorHealth() -> Element {
    rsx! { pages::pasion::connector_health::ConnectorHealthPage {} }
}

#[component]
fn PasionUpstreamProviders() -> Element {
    rsx! { pages::pasion::upstream_providers::UpstreamProvidersPage {} }
}

#[component]
fn PasionUpstreamLinks() -> Element {
    rsx! { pages::pasion::upstream_links::UpstreamLinksPage {} }
}

#[component]
fn PasionNotificationChannels() -> Element {
    rsx! { pages::pasion::notification_channels::NotificationChannelsPage {} }
}

#[component]
fn PasionNotificationTemplates() -> Element {
    rsx! { pages::pasion::notification_templates::NotificationTemplatesPage {} }
}

#[component]
fn NotFound(route: Vec<String>) -> Element {
    rsx! {
        div { class: "flex items-center justify-center min-h-screen",
            div { class: "text-center",
                h1 { class: "text-4xl font-bold mb-4", {t("not_found.title")} }
                p { class: "text-muted-foreground mb-4",
                    {format!("{}: /{}", t("not_found.message"), route.join("/"))}
                }
                Link { to: Route::Dashboard {}, class: "text-primary hover:underline",
                    {t("not_found.go_dashboard")}
                }
            }
        }
    }
}
