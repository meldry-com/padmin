use dioxus::prelude::*;

use crate::components::ui::icons::Icon;
use crate::router::Route;
use crate::utils::i18n::t;

struct NavItem {
    title: String,
    route: Route,
    icon: &'static str,
}

struct NavSection {
    label: String,
    items: Vec<NavItem>,
}

#[component]
pub fn AppSidebar(collapsed: Signal<bool>, mobile_open: Signal<bool>) -> Element {
    let mut mobile_open = mobile_open;
    let nav = use_navigator();
    let current_path = use_route::<Route>();

    let has_palpo_admin = crate::utils::instance_config::is_palpo_admin_enabled();
    let icfg = crate::utils::instance_config::get_instance_config();

    // Build grouped navigation sections
    let mut sections: Vec<NavSection> = Vec::new();

    // Dashboard (standalone)
    sections.push(NavSection {
        label: String::new(),
        items: vec![NavItem {
            title: t("nav.dashboard"),
            route: Route::Dashboard {},
            icon: "layout-dashboard",
        }],
    });

    // Identity
    sections.push(NavSection {
        label: t("nav.section_identity"),
        items: vec![
            NavItem {
                title: t("nav.users"),
                route: Route::UserList {},
                icon: "users",
            },
            NavItem {
                title: t("nav.registration_tokens"),
                route: Route::RegistrationTokenList {},
                icon: "key",
            },
            NavItem {
                title: t("nav.auth_status"),
                route: Route::AuthStatus {},
                icon: "shield",
            },
        ],
    });

    // Moderation
    sections.push(NavSection {
        label: t("nav.section_moderation"),
        items: vec![
            NavItem {
                title: t("nav.rooms"),
                route: Route::RoomList {},
                icon: "message-square",
            },
            NavItem {
                title: t("nav.reports"),
                route: Route::ReportList {},
                icon: "flag",
            },
            NavItem {
                title: t("nav.server_notices"),
                route: Route::ServerNotices {},
                icon: "megaphone",
            },
        ],
    });

    // Federation & Media
    sections.push(NavSection {
        label: t("nav.section_infrastructure"),
        items: vec![
            NavItem {
                title: t("nav.federation"),
                route: Route::DestinationList {},
                icon: "globe",
            },
            NavItem {
                title: t("nav.media"),
                route: Route::MediaList {},
                icon: "image",
            },
        ],
    });

    // Server Ops
    // Most items are Palpo-admin-sidecar specific and stay gated on
    // `has_palpo_admin`. Appservices is the exception — it hits
    // `/_palpo/admin/v1/appservices` on the main homeserver via the
    // same-origin Nginx proxy, so it is always available to admins.
    let mut server_items = Vec::new();
    if has_palpo_admin {
        if !icfg.disabled.monitoring {
            server_items.push(NavItem {
                title: t("nav.server_status"),
                route: Route::ServerStatus {},
                icon: "activity",
            });
        }
        if !icfg.disabled.actions {
            server_items.push(NavItem {
                title: t("nav.server_actions"),
                route: Route::ServerActions {},
                icon: "server",
            });
        }
        if !icfg.disabled.notifications {
            server_items.push(NavItem {
                title: t("nav.notifications"),
                route: Route::ServerNotifications {},
                icon: "bell",
            });
        }
        if !icfg.disabled.payments {
            server_items.push(NavItem {
                title: t("nav.billing"),
                route: Route::Billing {},
                icon: "credit-card",
            });
        }
    }
    server_items.push(NavItem {
        title: t("nav.appservices"),
        route: Route::AppserviceList {},
        icon: "plug",
    });
    if !server_items.is_empty() {
        sections.push(NavSection {
            label: t("nav.section_server_ops"),
            items: server_items,
        });
    }

    // Pasion / Identity Provider (only when configured)
    let has_pasion = crate::utils::storage::get_item("pasion_url").is_some();
    if has_pasion {
        sections.push(NavSection {
            label: t("nav.section_pasion"),
            items: vec![
                NavItem {
                    title: t("nav.audit_log"),
                    route: Route::PasionAuditLog {},
                    icon: "scroll-text",
                },
                NavItem {
                    title: t("nav.oauth2_sessions"),
                    route: Route::PasionOAuth2Sessions {},
                    icon: "key",
                },
                NavItem {
                    title: t("nav.personal_tokens"),
                    route: Route::PasionPersonalSessions {},
                    icon: "fingerprint",
                },
                NavItem {
                    title: t("nav.upstream_providers"),
                    route: Route::PasionUpstreamProviders {},
                    icon: "link",
                },
                NavItem {
                    title: t("nav.connector_health"),
                    route: Route::PasionConnectorHealth {},
                    icon: "heart-pulse",
                },
                NavItem {
                    title: t("nav.notification_channels"),
                    route: Route::PasionNotificationChannels {},
                    icon: "mail",
                },
            ],
        });
    }

    // Settings
    sections.push(NavSection {
        label: String::new(),
        items: vec![NavItem {
            title: t("nav.notification_prefs"),
            route: Route::NotificationPreferences {},
            icon: "settings",
        }],
    });

    let is_mobile_open = *mobile_open.read();
    let is_collapsed = *collapsed.read() && !is_mobile_open;
    let width_class = if is_collapsed { "w-16" } else { "w-64" };
    let mobile_state_class = if is_mobile_open { "sidebar-open" } else { "" };
    let nav_item_layout_class = if is_collapsed {
        "justify-center px-0"
    } else {
        "px-3"
    };

    rsx! {
        aside {
            class: "sidebar-shell sidebar-transition flex h-screen flex-col bg-sidebar border-r border-sidebar-border {width_class} {mobile_state_class}",

            // Header
            div { class: "flex h-14 items-center gap-2 border-b border-sidebar-border px-4",
                if !is_collapsed {
                    div { class: "flex items-center gap-2",
                        Icon { name: "shield".to_string(), class: "h-6 w-6 text-sidebar-primary".to_string() }
                        span { class: "truncate font-semibold text-sidebar-foreground", {t("nav.palpo_admin")} }
                    }
                } else {
                    div { class: "flex justify-center w-full",
                        Icon { name: "shield".to_string(), class: "h-6 w-6 text-sidebar-primary".to_string() }
                    }
                }
                button {
                    class: "sidebar-mobile-close inline-flex h-9 w-9 items-center justify-center rounded-lg text-sidebar-foreground transition-colors hover:bg-sidebar-accent hover:text-sidebar-accent-foreground touch-target",
                    title: t("header.close"),
                    onclick: move |_| mobile_open.set(false),
                    Icon { name: "x".to_string(), class: "h-4 w-4".to_string() }
                }
            }

            // Navigation
            nav { class: "sidebar-nav flex-1 overflow-y-auto py-2",
                for section in sections.iter() {
                    div { class: "sidebar-section px-2 mb-1",
                        if !is_collapsed && !section.label.is_empty() {
                            p { class: "sidebar-section-label text-xs font-semibold text-sidebar-foreground/50 px-3 pt-3 pb-1 uppercase tracking-wider",
                                {section.label.clone()}
                            }
                        }
                        for item in section.items.iter() {
                            {
                                let is_active = is_route_active(&current_path, &item.route);
                                let active_class = if is_active {
                                    "sidebar-nav-active"
                                } else {
                                    ""
                                };
                                let route = item.route.clone();
                                let title = item.title.clone();
                                rsx! {
                                    button {
                                        class: "sidebar-nav-button flex w-full items-center gap-3 py-2 text-sm font-medium transition-colors {nav_item_layout_class} {active_class}",
                                        onclick: move |_| {
                                            mobile_open.set(false);
                                            let _ = nav.push(route.clone());
                                        },
                                        Icon { name: item.icon.to_string(), class: "h-4 w-4 shrink-0".to_string() }
                                        if !is_collapsed {
                                            span { "{title}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Footer
            div { class: "border-t border-sidebar-border px-2 py-2",
                button {
                    class: "sidebar-nav-button flex w-full items-center gap-3 py-2 text-sm font-medium text-sidebar-foreground transition-colors {nav_item_layout_class}",
                    onclick: move |_| {
                        mobile_open.set(false);
                        spawn(async move {
                            let _ = crate::api::auth::logout().await;
                            nav.push(Route::LoginPage {});
                        });
                    },
                    Icon { name: "log-out".to_string(), class: "h-4 w-4 shrink-0".to_string() }
                    if !is_collapsed {
                        span { {t("nav.logout")} }
                    }
                }
            }
        }
    }
}

fn is_route_active(current: &Route, target: &Route) -> bool {
    let current_str = format!("{:?}", current);
    let target_str = format!("{:?}", target);

    if current_str == target_str {
        return true;
    }

    match target {
        Route::UserList {} => matches!(
            current,
            Route::UserList {} | Route::UserShow { .. } | Route::UserCreate {}
        ),
        Route::RoomList {} => matches!(current, Route::RoomList {} | Route::RoomShow { .. }),
        Route::ReportList {} => matches!(
            current,
            Route::ReportList {} | Route::ReportShow { .. }
        ),
        Route::DestinationList {} => matches!(
            current,
            Route::DestinationList {} | Route::DestinationShow { .. }
        ),
        _ => false,
    }
}
