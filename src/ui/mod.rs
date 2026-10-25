pub mod context;
pub mod pages;
pub mod widgets;
use crate::utils::{
    i18n::{self, Language},
    storage,
};
use context::{Command, Context};
use glory::prelude::*;
use glory::ui::{View, widget};
use std::rc::Rc;

fn queue(command: Command) {
    if let Some(ctx) = context::current() {
        ctx.queue(command);
        return;
    }
    #[cfg(target_arch = "wasm32")]
    crate::web::queue(command);
}
pub fn navigate(path: &str) {
    queue(Command::Navigate(path.into()));
}
pub fn enqueue_path(path: &str) {
    queue(Command::Pop(path.into()));
}
pub fn invalidate_auth() {
    queue(Command::Reverify);
}

pub fn app(scope: &Scope, initial: String) -> View {
    let ctx = context::install(scope);
    let focus = ctx.focus_request;
    let focus_scope = scope.clone();
    scope.effect(move || {
        if let Some(node) = focus.get() {
            let _ = focus_scope.action(glory::render::Action::Focus(node));
        }
    });
    let route = scope.cage(initial);
    let verdict = scope.cage(0i32);
    let authenticated = scope.cage(false);
    let error = scope.cage(String::new());
    let retry = scope.cage(true);
    let ready = scope.cage(false);
    let config_retry = scope.cage(true);
    let config_error = scope.cage(String::new());
    widgets::spawn(scope, async move {
        loop {
            gloo_timers::future::TimeoutFuture::new(25).await;
            if !config_retry.get() {
                continue;
            }
            config_retry.revise(|v| *v = false);
            match crate::utils::config::try_load_runtime_config().await {
                Ok(config) => {
                    for (key, value) in [
                        ("pasion_public_url", config.pasion_public_url),
                        ("oauth_client_id", config.oauth_client_id),
                        ("palpo_admin_url", config.palpo_admin_url),
                    ] {
                        if !value.trim().is_empty() {
                            storage::set_item(key, value.trim());
                        }
                    }
                    config_error.revise(|v| v.clear());
                    ready.revise(|v| *v = true);
                }
                Err(e) => config_error.revise(|v| *v = e.to_string()),
            }
        }
    });
    let weak = Rc::downgrade(&ctx);
    widgets::spawn(scope, async move {
        loop {
            gloo_timers::future::TimeoutFuture::new(25).await;
            let Some(ctx) = weak.upgrade() else {
                break;
            };
            let identity = (
                storage::get_item("user_id").unwrap_or_default(),
                storage::get_item("user_display_name").unwrap_or_default(),
            );
            if ctx.identity.with(|v| *v != identity) {
                ctx.identity.revise(|v| *v = identity);
            }
            let now = chrono::Utc::now().timestamp_millis();
            if ctx
                .toasts
                .with(|items| items.iter().any(|item| item.expires <= now))
            {
                ctx.toasts
                    .revise(|items| items.retain(|item| item.expires > now));
            }
            let commands: Vec<_> = ctx.commands.borrow_mut().drain(..).collect();
            for command in commands {
                match command {
                    Command::Navigate(path) => {
                        ctx.drawer.revise(|v| *v = false);
                        #[cfg(target_arch = "wasm32")]
                        if let Some(window) = web_sys::window() {
                            let _ = window.history().and_then(|h| {
                                h.push_state_with_url(
                                    &wasm_bindgen::JsValue::NULL,
                                    "",
                                    Some(&crate::utils::config::app_path(&path)),
                                )
                            });
                        }
                        route.revise(|v| *v = path);
                    }
                    Command::Pop(path) => {
                        ctx.drawer.revise(|v| *v = false);
                        route.revise(|v| *v = path);
                    }
                    Command::Reverify => {
                        retry.revise(|v| *v = true);
                    }
                    Command::Retire => {
                        crate::api::auth::retire_session();
                        ctx.queue(Command::Navigate("/login".into()));
                    }
                    Command::SystemTheme(dark) => ctx.dark.revise(|v| *v = dark),
                    Command::Key(key, now) => {
                        if key == "Escape" {
                            ctx.help.revise(|v| *v = false);
                        } else if key == "Tab" || key == "Shift+Tab" {
                            if ctx.help.get() {
                                ctx.focus_request
                                    .revise(|v| *v = *ctx.dialog_focus.borrow());
                            }
                        } else if key == "?" {
                            ctx.help.revise(|v| *v = !*v);
                        } else if key == "g" {
                            *ctx.pending_key.borrow_mut() = Some(now);
                        } else {
                            let pending = ctx.pending_key.borrow_mut().take();
                            if pending.is_some_and(|t| now - t <= 500.0) {
                                if let Some(path) = match key.as_str() {
                                    "d" => Some("/"),
                                    "u" => Some("/users"),
                                    "r" => Some("/rooms"),
                                    _ => None,
                                } {
                                    ctx.queue(Command::Navigate(path.into()));
                                }
                            }
                        }
                    }
                }
            }
            let auth = crate::api::auth::is_authenticated();
            if authenticated.get() != auth {
                authenticated.revise(|v| *v = auth);
                if !auth {
                    verdict.revise(|v| *v = 0);
                } else {
                    retry.revise(|v| *v = true);
                }
            }
        }
    });
    widgets::spawn(scope, async move {
        let mut ticks = 0u32;
        loop {
            gloo_timers::future::TimeoutFuture::new(25).await;
            if !ready.get() || !crate::api::auth::is_authenticated() {
                continue;
            }
            ticks += 1;
            if !retry.get() && ticks < 1200 {
                continue;
            }
            retry.revise(|v| *v = false);
            ticks = 0;
            let generation = crate::api::auth::session_generation();
            let probe = Box::pin(crate::api::auth::verify_admin());
            let timeout = Box::pin(gloo_timers::future::TimeoutFuture::new(15000));
            let result = match futures::future::select(probe, timeout).await {
                futures::future::Either::Left((result, _)) => result,
                futures::future::Either::Right(_) => Err(crate::utils::error::HttpError::message(
                    "Administrator verification timed out",
                )),
            };
            if generation != crate::api::auth::session_generation()
                || !crate::api::auth::is_authenticated()
            {
                continue;
            }
            match result {
                Ok(value) => {
                    error.revise(|v| v.clear());
                    if verdict.get() != if value { 1 } else { -1 } {
                        verdict.revise(|v| *v = if value { 1 } else { -1 });
                    }
                }
                Err(e) => {
                    error.revise(|v| *v = e.to_string());
                    verdict.revise(|v| *v = 0);
                }
            }
        }
    });
    let current = scope.bond(move || {
        let path = route.get();
        // Public OAuth processing has a one-shot Owner. Auth probes cannot remount
        // it between token publication and completion of its final profile read.
        let public = matches!(
            crate::router::parse(&path, "").path.as_str(),
            "/login" | "/oauth/callback"
        );
        vec![(
            path,
            if public { 0 } else { verdict.get() },
            ready.get(),
            if public { false } else { authenticated.get() },
        )]
    });
    let page_ctx = ctx.clone();
    let content = each(
        current,
        |v| v.clone(),
        move |item, _| {
            let ctx = page_ctx.clone();
            widget(move |scope| {
                context::enter(&ctx, || {
                    let (path, verdict, ready, auth) = item.get();
                    let parsed = crate::router::parse(&path, "");
                    let widget = if !ready {
                        widgets::page("Loading configuration…").fill((
                            widgets::status(config_error),
                            widgets::action("Retry configuration", move || {
                                config_retry.revise(|v| *v = true)
                            }),
                        ))
                    } else if parsed.path == "/login" {
                        login(scope)
                    } else if parsed.path == "/oauth/callback" {
                        callback(scope)
                    } else if parsed.path == "/*" {
                        widgets::page("Page not found").fill((
                            widgets::label("404"),
                            widgets::action("Go to Dashboard", || navigate("/")),
                        ))
                    } else if !auth {
                        widgets::page("Sign in required")
                            .fill(widgets::action("Sign in", || navigate("/login")))
                    } else if verdict != 1 {
                        widgets::page(if verdict < 0 {
                            "Not authorized"
                        } else {
                            "Checking administrator access"
                        })
                        .fill((
                            widgets::status(error),
                            widgets::action("Retry", move || retry.revise(|v| *v = true)),
                        ))
                    } else {
                        pages::render(scope, &parsed)
                    };
                    let mounted = scope.mount_with(|| widget)?;
                    scope.retain(mounted)
                })
            })
        },
    );
    let header = header(scope, &ctx);
    let public_route = route.map(|p| matches!(p.as_str(), "/login" | "/oauth/callback"));
    let nav = view()
        .role(Role::Navigation)
        .name(widgets::localized(scope, "Administration".into()))
        .class(ctx.drawer.map(|open| {
            if *open {
                "sidebar open".into()
            } else {
                "sidebar".into()
            }
        }))
        .fill((view().class("sidebar-brand").fill((widgets::label("◇").class("brand-shield"),widgets::label("Palpo Admin").class("brand-name"))),view().class("sidebar-nav").fill(
            [("",vec!["/"]),("Identity",vec!["/users","/registration-tokens","/auth-status"]),("Moderation",vec!["/rooms","/reports","/server-notices"]),("Infrastructure",vec!["/destinations","/media"]),("Server operations",vec!["/server-status","/server-actions","/server-notifications","/appservices"]),("Pasion",vec!["/pasion/accounts","/pasion/audit-log","/pasion/oauth2-sessions","/pasion/personal-sessions","/pasion/upstream-providers","/pasion/upstream-links","/pasion/notification-templates","/pasion/connector-health","/pasion/notification-channels"]),("",vec!["/users/known","/settings/notifications"])].into_iter().map(|(label,paths)| {
                view().class("sidebar-section").fill(widgets::label(label).class("sidebar-section-label")).fill(paths.into_iter().map(|p|link().target(crate::utils::config::app_path(p)).label(widgets::localized(scope,route_title(p))).class(route.map(move|current| if current==p {"nav nav-link sidebar-nav-button active sidebar-nav-active".into()}else{"nav nav-link sidebar-nav-button".into()}))).collect::<Vec<_>>())
            }).collect::<Vec<_>>(),
        )));
    let language = ctx.language;
    let theme = ctx.theme;
    let dark = ctx.dark;
    let contrast = ctx.contrast;
    let class = scope.bond(move || {
        format!(
            "padmin {} {} {}",
            if theme.get() == "dark" || (theme.get() == "system" && dark.get()) {
                "dark"
            } else {
                "light"
            },
            if contrast.get() { "high-contrast" } else { "" },
            if public_route.get() {
                "public-layout"
            } else {
                "admin-layout"
            }
        )
    });
    let drawer = ctx.drawer;
    let backdrop = when(drawer, move || {
        widgets::action("Close navigation", move || drawer.revise(|v| *v = false))
            .class("nav-backdrop")
    });
    view().class(class).fill((
        backdrop,
        view()
            .class(ctx.collapsed.map(|v| {
                if *v {
                    "shell compact".into()
                } else {
                    "shell".into()
                }
            }))
            .fill((
                nav,
                view()
                    .class("workspace")
                    .fill((header, view().role(Role::Main).class("main").fill(content))),
            )),
        help(scope, &ctx),
        toast_panel(&ctx),
        text()
            .class("sr-only")
            .content(language.map(|v| v.code().to_owned())),
    ))
}
fn header(scope: &Scope, ctx: &Rc<Context>) -> View {
    let owner = scope.clone();
    let language = ctx.language;
    let contrast = ctx.contrast;
    let drawer = ctx.drawer;
    let collapsed = ctx.collapsed;
    let identity = ctx.identity;
    let logging_out = scope.cage(false);
    let mut head = view().class("header").fill((
        widgets::action("Toggle navigation", move || drawer.revise(|v| *v = !*v))
            .class("menu-toggle"),
        widgets::label("Palpo Admin").class("header-brand"),
        text()
            .class("session-avatar")
            .content(identity.map(|(id, name)| {
                name.chars()
                    .next()
                    .or_else(|| id.trim_start_matches('@').chars().next())
                    .unwrap_or('A')
                    .to_uppercase()
                    .to_string()
            })),
        text()
            .class("session-identity")
            .content(identity.map(|(id, name)| {
                if name.is_empty() {
                    id.clone()
                } else {
                    format!("{name} ({id})")
                }
            })),
        widgets::action("Toggle sidebar width", move || {
            collapsed.revise(|v| *v = !*v)
        })
        .class("desktop-toggle"),
        sapid::native_select("header-language")
            .unstyled()
            .name("Language")
            .class("header-language")
            .value(language.map(|value| Some(value.code().to_owned())))
            .options([
                sapid::select_option("en", "English"),
                sapid::select_option("zh-CN", "中文"),
            ])
            .on_value_change(move |value| {
                let next = if value.as_deref() == Some("zh-CN") {
                    Language::ZhCn
                } else {
                    Language::En
                };
                language.revise(|v| *v = next);
                i18n::set_language(next);
            }),
    ));
    let theme = ctx.theme;
    head = head.fill(
        sapid::button()
            .unstyled()
            .class("theme-toggle")
            .label(theme.map(|value| {
                if value == "dark" {
                    "Light".into()
                } else {
                    "Dark".into()
                }
            }))
            .on_press(move || {
                let next = if theme.get() == "dark" {
                    "light"
                } else {
                    "dark"
                };
                theme.revise(|v| *v = next.into());
                storage::set_item("theme", next);
            }),
    );
    let help = ctx.help;
    head.fill((
        notification_panel(scope, ctx),
        widgets::action("High contrast", move || {
            contrast.revise(|v| *v = !*v);
            storage::set_item(
                "high_contrast",
                if contrast.get() { "true" } else { "false" },
            );
        }),
        {
            let ctx = ctx.clone();
            widget(move |scope| {
                let mounted = scope.mount_with(|| {
                    widgets::action("Keyboard shortcuts", move || help.revise(|v| *v = !*v))
                })?;
                *ctx.help_origin.borrow_mut() = mounted.roots().first().copied();
                scope.retain(mounted)
            })
        },
        widgets::action("Sign out", move || {
            if logging_out.get() {
                return;
            }
            logging_out.revise(|v| *v = true);
            widgets::spawn(&owner, async move {
                let _ = crate::api::auth::logout().await;
                logging_out.revise(|v| *v = false);
                navigate("/login");
            });
        })
        .enabled(logging_out.map(|v| !*v)),
    ))
}
fn help(_scope: &Scope, ctx: &Rc<Context>) -> impl Widget {
    let show = ctx.help;
    let ctx = ctx.clone();
    when(show, move || {
        let ctx = ctx.clone();
        widget(move |scope| {
            let close_ctx = ctx.clone();
            let close = widget(move |scope| {
                let mounted = scope
                    .mount_with(|| widgets::action("Close", move || show.revise(|v| *v = false)))?;
                *close_ctx.dialog_focus.borrow_mut() = mounted.roots().first().copied();
                scope.retain(mounted)
            });
            let dialog = view()
                .role(Role::Dialog)
                .name(widgets::localized(scope, "Keyboard shortcuts".into()))
                .class("shortcut-help")
                .fill((
                    widgets::label("Keyboard shortcuts"),
                    widgets::label(
                        "g d: Dashboard · g u: Users · g r: Rooms · ?: Help · Escape: Close",
                    ),
                    close,
                ));
            let mounted = scope.mount_with(|| dialog)?;
            scope.retain(mounted)?;
            ctx.focus_request
                .revise(|value| *value = *ctx.dialog_focus.borrow());
            let weak = Rc::downgrade(&ctx);
            scope.on_cleanup(move || {
                if let Some(ctx) = weak.upgrade() {
                    let _ = ctx
                        .focus_request
                        .try_revise(|v| *v = *ctx.help_origin.borrow());
                    ctx.dialog_focus.borrow_mut().take();
                }
            })?;
            Ok(())
        })
    })
}
fn notification_panel(scope: &Scope, ctx: &Rc<Context>) -> View {
    let items = ctx.notifications;
    let language = ctx.language;
    let show = scope.cage(false);
    let caption = scope.bond(move || {
        format!(
            "{} ({})",
            i18n::translate_text("Notifications", language.get()),
            items.with(|rows| rows.iter().filter(|v| !v.read).count())
        )
    });
    view().class("notifications").fill((
        button()
            .label(caption)
            .on(activate, move |_| show.revise(|v| *v = !*v)),
        when(show, move || {
            view().class("notification-list").fill((
                widgets::action("Mark all read", move || {
                    items.revise(|rows| {
                        for row in rows {
                            row.read = true;
                        }
                    })
                }),
                widgets::action("Clear notifications", move || items.revise(Vec::clear)),
                each(
                    items,
                    |v| v.id,
                    move |row, _| {
                        text().content(
                            row.map(|v| {
                                format!("{} · {} · {}", v.timestamp, v.severity, v.message)
                            }),
                        )
                    },
                ),
            ))
        }),
    ))
}
fn login(scope: &Scope) -> View {
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let owner = scope.clone();
    view().class("page login-page").fill(
        view().class("login-container").fill((
            view().class("login-brand").fill((
                widgets::label("◇").class("login-shield"),
                widgets::label("Palpo Admin").kind(glory::render::TextKind::Heading(1)),
                widgets::label("Sign in to manage your server").class("login-subtitle"),
            )),
            view().class("login-card glass-panel").fill((
                widgets::label("Sign in to Padmin")
                    .class("sr-only")
                    .kind(glory::render::TextKind::Heading(2)),
                widgets::label("Sign in with your account to access the admin dashboard")
                    .class("login-hint"),
                widgets::status(message),
                widgets::action("Sign in", move || {
                    widgets::submit(&owner, busy, message, async {
                        crate::api::auth::start_oauth_login().await?;
                        Ok("Redirecting…".into())
                    })
                })
                .enabled(busy.map(|v| !*v))
                .class("login-submit btn-primary"),
            )),
            widgets::label("Palpo Admin - Matrix Server Management").class("login-footer"),
        )),
    )
}
fn callback(scope: &Scope) -> View {
    let message = scope.cage("Completing sign in…".to_owned());
    widgets::spawn(scope, async move {
        #[cfg(target_arch = "wasm32")]
        let result = async {
            let search = web_sys::window()
                .unwrap()
                .location()
                .search()
                .unwrap_or_default();
            let params = web_sys::UrlSearchParams::new_with_str(&search)
                .map_err(|_| crate::utils::error::HttpError::message("Invalid callback"))?;
            if let Some(error) = params.get("error") {
                let description = params.get("error_description").filter(|v| !v.is_empty());
                return Err(crate::utils::error::HttpError::message(
                    description
                        .map(|description| format!("{error}: {description}"))
                        .unwrap_or(error),
                ));
            }
            let code = params.get("code").ok_or_else(|| {
                crate::utils::error::HttpError::message("Missing authorization code")
            })?;
            crate::api::auth::handle_oauth_callback(&code, params.get("state").as_deref()).await
        }
        .await;
        #[cfg(not(target_arch = "wasm32"))]
        let result: std::result::Result<(), crate::utils::error::HttpError> =
            Err(crate::utils::error::HttpError::message("Browser required"));
        match result {
            Ok(()) => {
                invalidate_auth();
                navigate("/");
            }
            Err(e) => message.revise(|v| *v = e.to_string()),
        }
    });
    widgets::page("OAuth callback").fill((
        widgets::status(message),
        widgets::action("Restart sign in", || navigate("/login")),
    ))
}

fn route_title(path: &str) -> String {
    if path == "/" {
        return "Dashboard".into();
    }
    path.trim_matches('/')
        .split('/')
        .map(|s| {
            s.split('-')
                .map(|word| {
                    let mut chars = word.chars();
                    chars
                        .next()
                        .map(|c| c.to_uppercase().to_string() + chars.as_str())
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

fn toast_panel(ctx: &Rc<Context>) -> impl Widget {
    let items = ctx.toasts;
    view().class("toasts").fill(each(
        items,
        |row| row.id,
        move |item, _| {
            let id = item.get().id;
            view()
                .role(if item.get().failed {
                    Role::Alert
                } else {
                    Role::Status
                })
                .class("toast")
                .fill((
                    text().content(item.map(|v| v.message.clone())),
                    widgets::action("Dismiss", move || {
                        items.revise(|rows| rows.retain(|v| v.id != id))
                    }),
                ))
        },
    ))
}
