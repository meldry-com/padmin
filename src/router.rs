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

        #[route("/settings/notifications")]
        NotificationPreferences {},

        #[route("/pasion/accounts")]
        PasionAccounts {},
        #[route("/pasion/accounts/:user_id")]
        PasionAccountShow { user_id: String },
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

/// Session-scoped cache of the admin verdict, shared across the whole router.
///
///   Some(true)  = verified admin
///   Some(false) = verified non-admin (forbidden)
///   None        = not yet probed this session
///
/// `verify_admin()` is an HTTP round-trip, so we probe at most once per
/// session and trust this signal for every subsequent in-session navigation.
/// The probe is only re-run when this is `None`, which happens on the first
/// authenticated mount and again after `auth::handle_unauthorized()` clears
/// the session on a 401 (see `reset_admin_cache`).
static ADMIN_VERDICT: GlobalSignal<Option<bool>> = GlobalSignal::new(|| None);

/// Clear the cached admin verdict so the next navigation re-probes. Call this
/// when the session is invalidated (e.g. after a 401).
pub fn reset_admin_cache() {
    *ADMIN_VERDICT.write() = None;
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
    // Trust the session-scoped `ADMIN_VERDICT` for in-session navigation: the
    // probe only runs when the signal is still `None` (first authenticated
    // mount this session, or after a 401 reset). Seed the signal from the
    // `is_admin` value persisted by a previous tab/session so a hard refresh
    // doesn't flash the spinner.
    if ADMIN_VERDICT.peek().is_none() {
        if let Some(persisted) = auth::cached_is_admin() {
            *ADMIN_VERDICT.write() = Some(persisted);
        }
    }

    use_resource(move || async move {
        // Already decided this session — don't re-probe on navigation.
        if ADMIN_VERDICT.peek().is_some() {
            return;
        }
        match auth::verify_admin().await {
            Ok(flag) => *ADMIN_VERDICT.write() = Some(flag),
            // Probe errored (401, network, ...). Fall through to "assume
            // admin" so we don't dead-end the user on a spinner forever.
            // Individual admin pages still surface their own errors when the
            // token turns out to be bad, and a 401 there resets this cache.
            Err(_) => *ADMIN_VERDICT.write() = Some(true),
        }
    });

    match *ADMIN_VERDICT.read() {
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
fn NotificationPreferences() -> Element {
    rsx! {
        pages::notification_preferences::NotificationPreferencesPage {}
    }
}

#[component]
fn PasionAccounts() -> Element {
    rsx! { pages::pasion::accounts::PasionAccountsPage {} }
}

#[component]
fn PasionAccountShow(user_id: String) -> Element {
    rsx! { pages::pasion::account_show::PasionAccountShowPage { user_id } }
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
