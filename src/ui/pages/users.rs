use crate::ui::widgets as w;
use crate::{
    api::{known_users, user_extras, users as api},
    types::*,
    utils::error::HttpError,
};
use glory::prelude::*;
use glory::render::{Event, Purpose};
use std::result::Result;
use std::{future::Future, rc::Rc};

pub(super) fn operation<F, Fut>(
    scope: &glory::ui::Scope,
    title: &str,
    busy: Cage<bool>,
    message: Cage<String>,
    confirmation: Option<&str>,
    action: F,
) -> crate::ui::widgets::LocalButton
where
    F: Fn() -> Fut + 'static,
    Fut: Future<Output = Result<String, HttpError>> + 'static,
{
    let scope = scope.clone();
    let confirmation = confirmation.map(str::to_owned);
    w::action(title, move || {
        if busy.get() {
            return;
        }
        if confirmation
            .as_ref()
            .is_some_and(|prompt| !w::confirm(prompt))
        {
            return;
        }
        w::submit(&scope, busy, message, action());
    })
    .enabled(busy.map(|v| !v))
}
pub(super) fn toggle(scope: &glory::ui::Scope, label: &str, value: Cage<bool>) -> glory::ui::Check {
    check()
        .label(w::localized(scope, label.into()))
        .checked(value)
        .on(checked_changed, move |event| {
            if let Event::CheckedChanged(next) = event {
                value.revise(|v| *v = next);
            }
        })
}
pub(super) fn write<T: 'static>(cage: Cage<T>, value: T) {
    cage.revise(|slot| *slot = value);
}
pub(super) fn required(value: String, field: &str) -> Result<String, HttpError> {
    let value = value.trim().to_string();
    if value.is_empty() {
        Err(HttpError::message(format!("{field} is required")))
    } else {
        Ok(value)
    }
}
pub(super) fn optional_u64(value: String, field: &str) -> Result<Option<u64>, HttpError> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        value
            .parse()
            .map(Some)
            .map_err(|_| HttpError::message(format!("{field} must be a nonnegative integer")))
    }
}
fn path(prefix: &str, id: &str) -> String {
    format!("{prefix}/{}", urlencoding::encode(id))
}
pub(super) fn stored(key: &str, default: &str) -> String {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(v) = web_sys::window()
            .and_then(|w| w.session_storage().ok().flatten())
            .and_then(|s| s.get_item(key).ok().flatten())
        {
            return v;
        }
    }
    let _ = key;
    default.into()
}
pub(super) fn remember(key: &str, value: &str) {
    #[cfg(target_arch = "wasm32")]
    if let Some(s) = web_sys::window().and_then(|w| w.session_storage().ok().flatten()) {
        let _ = s.set_item(key, value);
    }
    let _ = (key, value);
}

pub(super) fn column(
    scope: &glory::ui::Scope,
    key: &'static str,
    title: &str,
) -> (Cage<bool>, glory::ui::Check) {
    let visible = scope.cage(stored(key, "true") == "true");
    let control = check()
        .label(w::localized(scope, title.into()))
        .checked(visible)
        .on(checked_changed, move |event| {
            if let Event::CheckedChanged(value) = event {
                write(visible, value);
                remember(key, if value { "true" } else { "false" });
            }
        });
    (visible, control)
}

/// Finite page-size choices keep server requests and mounted rows bounded.
pub(super) fn page_size(scope: &glory::ui::Scope, key: &str) -> Cage<u64> {
    let saved = stored(key, "25").parse().unwrap_or(25);
    scope.cage(if [10, 25, 50, 100].contains(&saved) {
        saved
    } else {
        25
    })
}
pub(super) fn page_options(
    size: Cage<u64>,
    page: Cage<u64>,
    busy: Cage<bool>,
    key: &'static str,
    load: Rc<dyn Fn()>,
) -> glory::ui::View {
    let mut options = view()
        .name("Page size")
        .fill(text().content(size.map(|n| format!("Rows per page: {n}"))));
    for count in [10u64, 25, 50, 100] {
        let load = load.clone();
        options = options.fill(
            w::action(&format!("{count} rows per page"), move || {
                if !busy.get() {
                    write(size, count);
                    write(page, 1);
                    remember(key, &count.to_string());
                    load();
                }
            })
            .enabled(busy.map(|b| !b)),
        );
    }
    options
}

/// Paginate endpoints that return a full collection; only clone and mount a bounded slice.
pub(super) fn collection_window<T: Clone + 'static>(
    scope: &glory::ui::Scope,
    data: Cage<Vec<T>>,
    name: &'static str,
) -> (Bond<Vec<T>>, glory::ui::View) {
    let offset = scope.cage(0usize);
    let visible = data.map(move |rows| {
        let start = offset.get().min(rows.len().saturating_sub(1) / 25 * 25);
        rows.iter().skip(start).take(25).cloned().collect()
    });
    let controls = view().name(format!("{name} pagination")).fill((
        text().content(data.map(move |rows| {
            let start = offset.get().min(rows.len().saturating_sub(1) / 25 * 25);
            format!(
                "{name}: {}–{} of {}",
                if rows.is_empty() { 0 } else { start + 1 },
                (start + 25).min(rows.len()),
                rows.len()
            )
        })),
        w::action(&format!("Previous {name}"), move || {
            offset.revise(|n| *n = n.saturating_sub(25))
        })
        .enabled(offset.map(|n| *n > 0)),
        w::action(&format!("Next {name}"), move || {
            let len = data.with(Vec::len);
            offset.revise(|n| {
                let current = (*n).min(len.saturating_sub(1) / 25 * 25);
                if current + 25 < len {
                    *n = current + 25;
                }
            });
        })
        .enabled(data.map(move |rows| offset.get() + 25 < rows.len())),
    ));
    (visible, controls)
}

pub fn render(scope: &glory::ui::Scope, route: &crate::router::Route) -> glory::ui::View {
    match route.path.as_str() {
        "/users/create" => create(scope),
        "/users/known" => known(scope),
        "/users/known/:user_id" => {
            known_detail(scope, route.params.first().cloned().unwrap_or_default())
        }
        "/users/:user_id" => detail(scope, route.params.first().cloned().unwrap_or_default()),
        _ => list(scope),
    }
}
fn list(scope: &glory::ui::Scope) -> glory::ui::View {
    let (search, search_input) = w::field(scope, "Search users");
    write(search, stored("users.search", ""));
    let page = scope.cage(stored("users.page", "1").parse::<u64>().unwrap_or(1).max(1));
    let order = scope.cage(stored("users.sort", "name"));
    let descending = scope.cage(stored("users.desc", "false") == "true");
    let size = page_size(scope, "users.size");
    let rows = scope.cage(Vec::<UserRecord>::new());
    let total = scope.cage(0u64);
    let message = scope.cage("Loading users…".to_string());
    let busy = scope.cage(false);
    let selection = scope.cage(Vec::<String>::new());
    let (show_profile, profile_column) =
        column(scope, "users.column.profile", "Show user profile column");
    let (show_access, access_column) =
        column(scope, "users.column.access", "Show user access column");
    let (show_admin, admin_column) =
        column(scope, "users.column.admin", "Show administrator column");
    let (show_created, created_column) =
        column(scope, "users.column.created", "Show creation time column");
    let applied_search = scope.cage(search.get());
    let accepted = scope.cage((
        page.get(),
        size.get(),
        order.get(),
        descending.get(),
        search.get(),
    ));
    let load: Rc<dyn Fn()> = {
        let scope = scope.clone();
        Rc::new(move || {
            if busy.get() {
                return;
            }
            let requested_size = size.get();
            let (p, s, o, d) = (
                page.get(),
                applied_search.get(),
                order.get(),
                descending.get(),
            );
            remember("users.page", &p.to_string());
            remember("users.search", &s);
            remember("users.sort", &o);
            remember("users.desc", &d.to_string());
            write(busy, true);
            write(message, "Loading users…".into());
            write(selection, Vec::new());
            w::spawn(&scope, async move {
                match api::get_users(p, requested_size, &o, if d { "DESC" } else { "ASC" }, &s)
                    .await
                {
                    Ok(result) => {
                        write(accepted, (p, requested_size, o.clone(), d, s.clone()));
                        write(total, result.total);
                        write(
                            message,
                            if result.data.is_empty() {
                                "No users found".into()
                            } else {
                                format!("{} users · page {p}", result.total)
                            },
                        );
                        write(rows, result.data);
                    }
                    Err(e) => {
                        let (p, n, o, d, s) = accepted.get();
                        write(page, p);
                        write(size, n);
                        write(order, o.clone());
                        write(descending, d);
                        write(applied_search, s.clone());
                        remember("users.page", &p.to_string());
                        remember("users.size", &n.to_string());
                        remember("users.sort", &o);
                        remember("users.desc", &d.to_string());
                        remember("users.search", &s);
                        write(message, e.to_string());
                    }
                }
                write(busy, false);
            });
        })
    };
    load();
    let mut out = w::page("Users").fill((
        search_input,
        profile_column,
        admin_column,
        created_column,
        access_column,
        page_options(size, page, busy, "users.size", load.clone()),
        w::status(message),
        w::action("Search / refresh", {
            let load = load.clone();
            move || {
                if !busy.get() {
                    write(page, 1);
                    write(applied_search, search.get());
                    load();
                }
            }
        }),
        w::action("Previous page", {
            let load = load.clone();
            move || {
                if !busy.get() && page.get() > 1 {
                    page.revise(|p| *p -= 1);
                    load();
                }
            }
        }),
        w::action("Next page", {
            let load = load.clone();
            move || {
                if !busy.get() && page.get() * size.get() < total.get() {
                    page.revise(|p| *p += 1);
                    load();
                }
            }
        }),
        w::action("Create user", || crate::ui::navigate("/users/create")),
        w::action("Known users", || crate::ui::navigate("/users/known")),
    ));
    for (label, key) in [
        ("Sort by user ID", "name"),
        ("Sort by display name", "displayname"),
        ("Sort by administrator", "admin"),
        ("Sort by creation time", "creation_ts"),
    ] {
        let load = load.clone();
        out = out.fill(w::action(label, move || {
            if !busy.get() {
                write(order, key.into());
                write(page, 1);
                load();
            }
        }));
    }
    out = out.fill((
        w::action("Reverse sort", {
            let load = load.clone();
            move || {
                if !busy.get() {
                    descending.revise(|v| *v = !*v);
                    write(page, 1);
                    load();
                }
            }
        }),
        each(
            rows,
            |row| row.id.clone(),
            move |row, _| {
                view().class("record").fill((
                    check()
                        .label(row.map(|u| format!("Select {}", u.id)))
                        .checked(row.map(move |u| selection.with(|s| s.contains(&u.id))))
                        .on(checked_changed, move |event| {
                            if let Event::CheckedChanged(selected) = event {
                                let id = row.get().id;
                                selection.revise(|s| {
                                    s.retain(|v| v != &id);
                                    if selected {
                                        s.push(id);
                                    }
                                });
                            }
                        }),
                    text().content(row.map(move |u| {
                        let mut value = u.id.clone();
                        if show_profile.get() {
                            value.push_str(&format!(
                                " · {}",
                                u.user.displayname.as_deref().unwrap_or("—")
                            ));
                        }
                        if show_admin.get() {
                            value.push_str(if u.user.admin {
                                " · administrator"
                            } else {
                                " · regular account"
                            });
                        }
                        if show_access.get() {
                            value.push_str(if u.user.deactivated {
                                " · deactivated"
                            } else {
                                " · active"
                            });
                        }
                        if show_created.get() {
                            value.push_str(&format!(" · Created: {}", u.creation_ts_ms));
                        }
                        value
                    })),
                    w::action("User details", move || {
                        crate::ui::navigate(&path("/users", &row.get().id))
                    }),
                ))
            },
        ),
    ));
    for (title, kind) in [
        ("Deactivate selected", 0),
        ("Reactivate selected", 1),
        ("Erase selected", 2),
        ("Delete selected users' devices", 3),
        ("Grant administrator to selected", 4),
    ] {
        out=out.fill(operation(scope,title,busy,message,Some("Apply this operation to all selected users? Erasure and device removal cannot be undone."),move||{let ids=selection.get();async move {
            if ids.is_empty(){return Err(HttpError::message("Select at least one user"));}
            let mut succeeded=0;let mut errors=Vec::new();
            for id in ids {let result=match kind {0|2=>api::deactivate_account(&id,kind==2).await.map(|_|()),1=>api::reactivate_account(&id).await.map(|_|()),3=>delete_devices_exact(&id).await,_=>grant_admin(&id).await};match result {Ok(())=>succeeded+=1,Err(e)=>errors.push(format!("{id}: {e}"))}}
            let fresh = api::get_users(page.get(), size.get(), &order.get(), if descending.get() { "DESC" } else { "ASC" }, &applied_search.get()).await;
            match fresh { Ok(data) => { write(total, data.total); write(rows, data.data); write(selection, Vec::new()); }, Err(e) => errors.push(format!("refresh: {e}")) }
            if errors.is_empty(){Ok(format!("Applied to {succeeded} users. List refreshed."))}else{Err(HttpError::message(format!("Applied to {succeeded}; {} failed: {}",errors.len(),errors.join("; "))))}
        }}));
    }
    out.fill((
        w::action("Select all users on this page", move || {
            write(
                selection,
                rows.with(|rows| rows.iter().map(|r| r.id.clone()).collect()),
            )
        }),
        w::action("Clear selection", move || write(selection, Vec::new())),
        import_panel(scope),
        export_panel(scope),
    ))
}
async fn delete_devices_exact(id: &str) -> Result<(), HttpError> {
    // Delete the first page repeatedly so removals never shift an unseen page.
    loop {
        let result = api::get_user_devices(id, 1, 100).await?;
        if result.data.is_empty() {
            return Ok(());
        }
        for device in result.data {
            api::delete_user_device(id, &device.id).await?;
        }
    }
}
fn password_strength(password: &str) -> &'static str {
    let upper = password.chars().any(|c| c.is_ascii_uppercase());
    let lower = password.chars().any(|c| c.is_ascii_lowercase());
    let digit = password.chars().any(|c| c.is_ascii_digit());
    let special = password.chars().any(|c| !c.is_alphanumeric());
    if password.is_empty() {
        ""
    } else if password.len() >= 10 && upper && lower && digit && special {
        "Strong"
    } else if password.len() >= 8 && upper && lower {
        "Medium"
    } else {
        "Weak"
    }
}
fn create(scope: &glory::ui::Scope) -> glory::ui::View {
    let (username, username_input) = w::field(scope, "Username");
    let (name, name_input) = w::field(scope, "Display name");
    let (password, password_input) = w::field(scope, "Password");
    let (email, email_input) = w::field(scope, "Email");
    let (phone, phone_input) = w::field(scope, "Phone number (international format)");
    let (kind, kind_input) = w::field(scope, "User type: regular, bot or support");
    write(kind, "regular".into());
    let admin = scope.cage(false);
    let locked = scope.cage(false);
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let availability = scope.cage(String::new());
    let has_pasion = crate::utils::storage::get_item("pasion_url").is_some();
    let sequence = scope.cage(0u64);
    let username_input = username_input.on(value_changed, {
        let scope = scope.clone();
        move |event| {
            if let Event::ValueChanged { value, .. } = event {
                write(username, value.clone());
                sequence.revise(|v| *v += 1);
                let revision = sequence.get();
                write(availability, "Checking username…".into());
                w::spawn(&scope, async move {
                    gloo_timers::future::TimeoutFuture::new(500).await;
                    if sequence.get() != revision {
                        return;
                    }
                    let result = if value.trim().is_empty() {
                        Ok(String::new())
                    } else {
                        api::check_username_available(value.trim())
                            .await
                            .map(|available| {
                                if available {
                                    "Username available".into()
                                } else {
                                    "Username already exists".into()
                                }
                            })
                    };
                    if sequence.get() == revision {
                        write(availability, result.unwrap_or_else(|e| e.to_string()));
                    }
                });
            }
        }
    });
    let mut out = w::page("Create user").fill((
        username_input,
        w::status(availability),
        name_input,
        password_input.purpose(Purpose::Password),
        when(
            password.map(|value| password_strength(value) == "Weak"),
            || w::label("Password strength: Weak"),
        ),
        when(
            password.map(|value| password_strength(value) == "Medium"),
            || w::label("Password strength: Medium"),
        ),
        when(
            password.map(|value| password_strength(value) == "Strong"),
            || w::label("Password strength: Strong"),
        ),
        email_input,
        phone_input,
        kind_input,
        toggle(scope, "Locked account", locked),
    ));
    if !has_pasion {
        out = out.fill(toggle(scope, "Administrator", admin));
    } else {
        out = out.fill(w::label(
            "Administrator status is owned by Pasion; manage it on the Pasion account.",
        ));
    }
    let context = crate::ui::context::get(scope);
    out.fill((
        w::status(message),
        w::action("Cancel", || crate::ui::navigate("/users")),
        operation(scope, "Create user", busy, message, None, move || {
            let context = context.clone();
            async move {
                let id = required(username.get(), "Username")?;
                let pass = required(password.get(), "Password")?;
                let email = email.get();
                let phone = phone.get();
                validate_contacts(&email, &phone)?;
                let kind = kind.get();
                if !["regular", "bot", "support"].contains(&kind.as_str()) {
                    return Err(HttpError::message("Choose regular, bot or support"));
                }
                if !api::check_username_available(&id).await? {
                    return Err(HttpError::message("Username is already taken"));
                }
                let mut threepids = Vec::new();
                if !email.is_empty() {
                    threepids.push(Threepid {
                        medium: "email".into(),
                        address: email,
                        ..Default::default()
                    });
                }
                if !phone.is_empty() {
                    threepids.push(Threepid {
                        medium: "msisdn".into(),
                        address: phone,
                        ..Default::default()
                    });
                }
                let user = api::create_user(
                    &id,
                    CreateUserRequest {
                        password: Some(pass),
                        displayname: Some(name.get()),
                        admin: !has_pasion && admin.get(),
                        locked: Some(locked.get()),
                        user_type: if kind == "regular" { None } else { Some(kind) },
                        threepids,
                        ..Default::default()
                    },
                )
                .await?;
                if let Some(context) = context {
                    context.notify(
                        "user_registered",
                        format!("User {} created", user.id),
                        false,
                    );
                }
                write(password, String::new());
                crate::ui::navigate(&path("/users", &user.id));
                Ok("User created".into())
            }
        }),
    ))
}
fn validate_contacts(email: &str, phone: &str) -> Result<(), HttpError> {
    if !email.is_empty()
        && !regex_lite::Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$")
            .unwrap()
            .is_match(email)
    {
        return Err(HttpError::message("Enter a valid email address"));
    }
    if !phone.is_empty()
        && !regex_lite::Regex::new(r"^\+?[1-9][0-9]{5,14}$")
            .unwrap()
            .is_match(phone)
    {
        return Err(HttpError::message(
            "Enter an international phone number with 6–15 digits",
        ));
    }
    Ok(())
}
fn known(scope: &glory::ui::Scope) -> glory::ui::View {
    let (search, search_input) = w::field(scope, "Search known users");
    let (server, server_input) = w::field(scope, "Server name filter");
    write(search, stored("known.search", ""));
    write(server, stored("known.server", ""));
    let origin = scope.cage(match stored("known.origin", "all").as_str() {
        "local" => known_users::Origin::Local,
        "remote" => known_users::Origin::Remote,
        _ => known_users::Origin::All,
    });
    let size = page_size(scope, "known.size");
    let page = scope.cage(stored("known.page", "1").parse::<u64>().unwrap_or(1).max(1));
    let total = scope.cage(0u64);
    let rows = scope.cage(Vec::<known_users::KnownUser>::new());
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let descending = scope.cage(stored("known.desc", "false") == "true");
    let order = scope.cage(stored("known.sort", "user_id"));
    let filters = scope.cage((search.get(), server.get()));
    let accepted = scope.cage((
        page.get(),
        size.get(),
        origin.get(),
        order.get(),
        descending.get(),
        filters.get(),
    ));
    let load: Rc<dyn Fn()> = {
        let scope = scope.clone();
        Rc::new(move || {
            if busy.get() {
                return;
            }
            write(busy, true);
            write(message, "Loading known users…".into());
            let (p, s, h, o, k, d) = (
                page.get(),
                filters.get().0,
                filters.get().1,
                origin.get(),
                order.get(),
                descending.get(),
            );
            remember("known.search", &s);
            remember("known.server", &h);
            remember("known.page", &p.to_string());
            remember("known.desc", &d.to_string());
            remember("known.sort", &k);
            remember(
                "known.origin",
                match o {
                    known_users::Origin::Local => "local",
                    known_users::Origin::Remote => "remote",
                    _ => "all",
                },
            );
            let requested_size = size.get();
            w::spawn(&scope, async move {
                match known_users::get_known_users(p, requested_size, &s, &h, o, &k, d).await {
                    Ok(data) => {
                        write(
                            accepted,
                            (p, requested_size, o, k.clone(), d, (s.clone(), h.clone())),
                        );
                        write(total, data.total);
                        write(
                            message,
                            if data.data.is_empty() {
                                "No known users found".into()
                            } else {
                                format!("{} known users · page {p}", data.total)
                            },
                        );
                        write(rows, data.data);
                    }
                    Err(e) => {
                        let (p, n, o, k, d, f) = accepted.get();
                        write(page, p);
                        write(size, n);
                        write(origin, o);
                        write(order, k);
                        write(descending, d);
                        write(filters, f);
                        remember("known.page", &p.to_string());
                        remember("known.size", &n.to_string());
                        remember(
                            "known.origin",
                            match o {
                                known_users::Origin::Local => "local",
                                known_users::Origin::Remote => "remote",
                                _ => "all",
                            },
                        );
                        remember("known.sort", &order.get());
                        remember("known.desc", &d.to_string());
                        let (s, h) = filters.get();
                        remember("known.search", &s);
                        remember("known.server", &h);
                        write(message, e.to_string());
                    }
                }
                write(busy, false);
            });
        })
    };
    load();
    let mut out = w::page("Known users").fill((
        w::label("Remote users are read only; account changes belong to their home server."),
        search_input,
        server_input,
        page_options(size, page, busy, "known.size", load.clone()),
        w::status(message),
        w::action("Search / refresh", {
            let load = load.clone();
            move || {
                if !busy.get() {
                    write(page, 1);
                    write(filters, (search.get(), server.get()));
                    load();
                }
            }
        }),
    ));
    for (label, value) in [
        ("All origins", known_users::Origin::All),
        ("Local users", known_users::Origin::Local),
        ("Remote users", known_users::Origin::Remote),
    ] {
        let load = load.clone();
        out = out.fill(w::action(label, move || {
            if !busy.get() {
                write(origin, value);
                write(filters, (search.get(), server.get()));
                write(page, 1);
                load();
            }
        }));
    }
    for (label, key) in [
        ("Sort by user ID", "user_id"),
        ("Sort by room count", "total_rooms"),
        ("Sort by membership time", "last_membership_at"),
        ("Sort by server name", "server_name"),
        ("Sort by joined rooms", "joined_rooms"),
    ] {
        let load = load.clone();
        out = out.fill(w::action(label, move || {
            if !busy.get() {
                write(order, key.into());
                write(page, 1);
                load();
            }
        }));
    }
    out.fill((
        w::action("Reverse sort", {
            let load = load.clone();
            move || {
                if !busy.get() {
                    descending.revise(|v| *v = !*v);
                    write(page, 1);
                    load();
                }
            }
        }),
        w::action("Previous page", {
            let load = load.clone();
            move || {
                if !busy.get() && page.get() > 1 {
                    page.revise(|p| *p -= 1);
                    load();
                }
            }
        }),
        w::action("Next page", {
            let load = load.clone();
            move || {
                if !busy.get() && page.get() * size.get() < total.get() {
                    page.revise(|p| *p += 1);
                    load();
                }
            }
        }),
        each(
            rows,
            |u| u.user_id.clone(),
            |row, _| {
                view().class("record").fill((
                    text().content(row.map(|u| {
                        format!(
                            "{} · {} · {} · {} joined / {} invited / {} left / {} banned",
                            u.user_id,
                            u.displayname.as_deref().unwrap_or("—"),
                            u.server_name,
                            u.joined_rooms,
                            u.invited_rooms,
                            u.left_rooms,
                            u.banned_rooms
                        )
                    })),
                    w::action("Known user details", move || {
                        crate::ui::navigate(&path("/users/known", &row.get().user_id))
                    }),
                ))
            },
        ),
    ))
}
fn known_detail(scope: &glory::ui::Scope, id: String) -> glory::ui::View {
    let info = scope.cage(String::new());
    let rooms = scope.cage(Vec::<known_users::KnownUserRoom>::new());
    let (visible_rooms, room_pages) = collection_window(scope, rooms, "known user rooms");
    let local = scope.cage(false);
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let load = {
        let id = id.clone();
        move || {
            let id = id.clone();
            async move {
                let user = known_users::get_known_user(&id).await?;
                write(local, user.is_local && user.has_account);
                write(
                    info,
                    format!(
                        "{}\n{}\nHome server: {} · {}",
                        user.user_id,
                        user.displayname.unwrap_or_default(),
                        user.server_name,
                        if user.is_local { "local" } else { "remote" }
                    ),
                );
                write(rooms, user.rooms);
                Ok::<String, HttpError>("Known user loaded".into())
            }
        }
    };
    let first = load();
    w::submit(scope, busy, message, first);
    w::page(&format!("Known user {id}")).fill((
        w::status(info),
        w::status(message),
        operation(scope, "Refresh", busy, message, None, load),
        when(local, {
            let id = id.clone();
            move || {
                let id = id.clone();
                w::action("Manage local account", move || {
                    crate::ui::navigate(&path("/users", &id))
                })
            }
        }),
        room_pages,
        each(
            visible_rooms,
            |r| r.room_id.clone(),
            |row, _| {
                view().class("record").fill((
                    text().content(row.map(|r| {
                        format!(
                            "{} · {} · {} · updated {}",
                            r.room_id,
                            r.name.as_deref().unwrap_or("—"),
                            r.membership,
                            r.updated_ts
                        )
                    })),
                    w::action("Room details", move || {
                        crate::ui::navigate(&path("/rooms", &row.get().room_id))
                    }),
                ))
            },
        ),
    ))
}
fn detail(scope: &glory::ui::Scope, id: String) -> glory::ui::View {
    let info = scope.cage(String::from("Loading profile…"));
    let current = scope.cage(None::<UserRecord>);
    let device_rows = scope.cage(Vec::<DeviceRecord>::new());
    let device_total = scope.cage(0u64);
    let device_page = scope.cage(1u64);
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let (name, name_input) = w::field(scope, "Display name");
    let admin = scope.cage(false);
    let shadow = scope.cage(false);
    let suspended = scope.cage(false);
    let locked = scope.cage(false);
    let has_pasion = crate::utils::storage::get_item("pasion_url").is_some();
    let load = {
        let id = id.clone();
        move || {
            let id = id.clone();
            async move {
                let user = api::get_user(&id).await?;
                write(
                    info,
                    format!(
                        "{}\n{}\nAdministrator: {} · Deactivated: {} · Erased: {} · Guest: {}\nCreated: {}\nExternal identities: {}",
                        user.id,
                        user.user.displayname.as_deref().unwrap_or("—"),
                        user.user.admin,
                        user.user.deactivated,
                        user.user.erased,
                        user.user.is_guest,
                        user.creation_ts_ms,
                        user.user
                            .external_ids
                            .iter()
                            .map(|v| format!("{}: {}", v.auth_provider, v.external_id))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                );
                write(name, user.user.displayname.clone().unwrap_or_default());
                write(admin, user.user.admin);
                write(shadow, user.user.shadow_banned);
                write(suspended, user.user.suspended);
                write(locked, user.user.locked);
                write(current, Some(user));
                Ok::<String, HttpError>("Profile loaded".into())
            }
        }
    };
    let first = load();
    w::submit(scope, busy, message, first);
    let mut out = w::page(&format!("User {id}")).fill((
        text().content(current.map(|u| {
            u.as_ref()
                .map(|u| {
                    format!(
                        "{} · {} · Administrator: {} · Deactivated: {} · Locked: {}",
                        u.id,
                        u.user.displayname.as_deref().unwrap_or("—"),
                        u.user.admin,
                        u.user.deactivated,
                        u.user.locked
                    )
                })
                .unwrap_or_else(|| "Loading profile…".into())
        })),
        w::status(message),
        operation(scope, "Refresh profile", busy, message, None, load),
        name_input,
    ));
    if !has_pasion {
        out = out.fill(toggle(scope, "Administrator", admin));
    } else {
        let uid = id.clone();
        out = out.fill((
            w::label("Administrator status belongs to Pasion."),
            operation(
                scope,
                "Open Pasion account",
                busy,
                message,
                None,
                move || {
                    let uid = uid.clone();
                    async move {
                        let username = uid
                            .trim_start_matches('@')
                            .split(':')
                            .next()
                            .unwrap_or_default();
                        let account =
                            crate::api::pasion::pasion_get_user_by_username(username).await?;
                        crate::ui::navigate(&path("/pasion/accounts", &account.id));
                        Ok("Opening Pasion account".into())
                    }
                },
            ),
        ));
    }
    out=out.fill((toggle(scope, "Shadow banned",shadow),toggle(scope, "Suspended",suspended),toggle(scope, "Locked",locked),operation(scope,"Save profile",busy,message,None,{let id=id.clone();move||{let id=id.clone();async move{if current.get().is_none(){return Err(HttpError::message("Load the user before editing"));}let mut body=serde_json::json!({"displayname":name.get(),"shadow_banned":shadow.get(),"suspended":suspended.get(),"locked":locked.get()});if !has_pasion{body["admin"]=admin.get().into();}let saved=api::update_user(&id,body).await?;write(current,Some(saved));Ok("Profile saved".into())}}})));
    for (title, deactivate, erase) in [
        ("Deactivate account", true, false),
        ("Deactivate and erase", true, true),
        ("Reactivate account", false, false),
    ] {
        let id = id.clone();
        out = out.fill(operation(
            scope,
            title,
            busy,
            message,
            Some(
                "Change this account's access? Erasure removes personal data and cannot be undone.",
            ),
            move || {
                let id = id.clone();
                async move {
                    let owner = if deactivate {
                        api::deactivate_account(&id, erase).await?
                    } else {
                        api::reactivate_account(&id).await?
                    };
                    write(current, Some(api::get_user(&id).await?));
                    Ok(format!(
                        "Account change accepted by {owner:?}; server state reloaded."
                    ))
                }
            },
        ));
    }
    let (password, password_input) = w::field(scope, "New password");
    out = out.fill((
        password_input.purpose(Purpose::Password),
        operation(
            scope,
            "Reset password and log out devices",
            busy,
            message,
            Some("Reset this password and log out all devices?"),
            {
                let id = id.clone();
                move || {
                    let id = id.clone();
                    async move {
                        let pass = required(password.get(), "Password")?;
                        api::reset_password(&id, &pass).await?;
                        let fresh = api::get_user_devices(&id, 1, 25).await?;
                        write(device_page, 1);
                        write(device_rows, fresh.data);
                        write(device_total, fresh.total);
                        write(password, String::new());
                        Ok("Password reset".into())
                    }
                }
            },
        ),
    ));
    out.fill((
        threepids_panel(scope, id.clone(), current),
        devices_panel(scope, id.clone(), device_rows, device_total, device_page),
        joined_panel(scope, id.clone()),
        whois_panel(scope, id.clone()),
        extras_panel(scope, id.clone()),
        notice_panel(scope, id),
    ))
}
fn threepids_panel(
    scope: &glory::ui::Scope,
    id: String,
    current: Cage<Option<UserRecord>>,
) -> glory::ui::View {
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let medium = scope.cage(false);
    let (address, address_input) = w::field(scope, "New email or phone address");
    view().class("panel").fill((
        w::label("Third-party identifiers"),
        w::status(message),
        each(
            current.map(|u| {
                u.as_ref()
                    .map(|u| u.user.threepids.clone())
                    .unwrap_or_default()
            }),
            |p| format!("{}:{}", p.medium, p.address),
            {
                let scope = scope.clone();
                let id = id.clone();
                move |row, _| {
                    let uid = id.clone();
                    view().class("record").fill((
                        text().content(row.map(|p| format!("{}: {}", p.medium, p.address))),
                        operation(
                            &scope,
                            "Remove identifier",
                            busy,
                            message,
                            Some("Remove this identifier from the user?"),
                            move || {
                                let uid = uid.clone();
                                let p = row.get();
                                async move {
                                    api::delete_user_threepid(&uid, &p.medium, &p.address).await?;
                                    write(current, Some(api::get_user(&uid).await?));
                                    Ok("Identifier removed".into())
                                }
                            },
                        ),
                    ))
                }
            },
        ),
        address_input,
        toggle(scope, "Phone identifier (otherwise email)", medium),
        operation(scope, "Add identifier", busy, message, None, move || {
            let id = id.clone();
            async move {
                let address = required(address.get(), "Address")?;
                if medium.get() {
                    validate_contacts("", &address)?;
                } else {
                    validate_contacts(&address, "")?;
                }
                let user = api::get_user(&id).await?;
                let mut all = user.user.threepids;
                let kind = if medium.get() { "msisdn" } else { "email" };
                if all.iter().any(|p| p.medium == kind && p.address == address) {
                    return Err(HttpError::message("Identifier already exists"));
                }
                all.push(Threepid {
                    medium: kind.into(),
                    address,
                    ..Default::default()
                });
                let user = api::update_user(&id, serde_json::json!({"threepids":all})).await?;
                write(current, Some(user));
                Ok("Identifier added".into())
            }
        }),
    ))
}
fn devices_panel(
    scope: &glory::ui::Scope,
    id: String,
    rows: Cage<Vec<DeviceRecord>>,
    total: Cage<u64>,
    page: Cage<u64>,
) -> glory::ui::View {
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let load = {
        let id = id.clone();
        move || {
            let id = id.clone();
            async move {
                let data = api::get_user_devices(&id, page.get(), 25).await?;
                write(total, data.total);
                write(rows, data.data);
                Ok(format!("{} devices · page {}", data.total, page.get()))
            }
        }
    };
    let scope_next = scope.clone();
    let next = load.clone();
    let scope_prev = scope.clone();
    let prev = load.clone();
    view().class("panel").fill((
        w::label("Devices"),
        w::status(message),
        operation(scope, "Load / refresh devices", busy, message, None, load),
        w::action("Previous device page", move || {
            if !busy.get() && page.get() > 1 {
                page.revise(|p| *p -= 1);
                w::submit(&scope_prev, busy, message, prev());
            }
        }),
        w::action("Next device page", move || {
            if !busy.get() && page.get() * 25 < total.get() {
                page.revise(|p| *p += 1);
                w::submit(&scope_next, busy, message, next());
            }
        }),
        each(rows, |d| d.id.clone(), {
            let scope = scope.clone();
            let id = id.clone();
            move |row, _| {
                let id = id.clone();
                view().class("record").fill((
                    text().content(row.map(|d| {
                        format!(
                            "{} · {} · IP {} · last seen {:?}",
                            d.id,
                            d.device.display_name.as_deref().unwrap_or("—"),
                            d.device.last_seen_ip.as_deref().unwrap_or("—"),
                            d.device.last_seen_ts
                        )
                    })),
                    operation(
                        &scope,
                        "Delete device",
                        busy,
                        message,
                        Some("Delete this device and invalidate its session?"),
                        move || {
                            let id = id.clone();
                            let device = row.get().id;
                            async move {
                                api::delete_user_device(&id, &device).await?;
                                let fresh = api::get_user_devices(&id, page.get(), 25).await?;
                                write(rows, fresh.data);
                                write(total, fresh.total);
                                Ok("Device deleted".into())
                            }
                        },
                    ),
                ))
            }
        }),
        operation(
            scope,
            "Delete all devices",
            busy,
            message,
            Some("Delete every device for this user?"),
            move || {
                let id = id.clone();
                async move {
                    delete_devices_exact(&id).await?;
                    write(rows, Vec::new());
                    write(total, 0);
                    Ok("All devices deleted".into())
                }
            },
        ),
    ))
}
fn joined_panel(scope: &glory::ui::Scope, id: String) -> glory::ui::View {
    let page = scope.cage(1u64);
    let total = scope.cage(0u64);
    let rows = scope.cage(Vec::<String>::new());
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let load = move || {
        let id = id.clone();
        async move {
            let data = api::get_user_joined_rooms(&id, page.get(), 25).await?;
            write(total, data.total);
            write(rows, data.data);
            Ok(format!("{} joined rooms · page {}", data.total, page.get()))
        }
    };
    let prev = load.clone();
    let next = load.clone();
    let ps = scope.clone();
    let ns = scope.clone();
    view().class("panel").fill((
        w::label("Joined rooms"),
        w::status(message),
        operation(
            scope,
            "Load / refresh joined rooms",
            busy,
            message,
            None,
            load,
        ),
        w::action("Previous joined-room page", move || {
            if !busy.get() && page.get() > 1 {
                page.revise(|p| *p -= 1);
                w::submit(&ps, busy, message, prev());
            }
        }),
        w::action("Next joined-room page", move || {
            if !busy.get() && page.get() * 25 < total.get() {
                page.revise(|p| *p += 1);
                w::submit(&ns, busy, message, next());
            }
        }),
        each(
            rows,
            |r| r.clone(),
            |row, _| {
                view().class("record").fill((
                    text().content(row),
                    w::action("Open joined room", move || {
                        crate::ui::navigate(&path("/rooms", &row.get()))
                    }),
                ))
            },
        ),
    ))
}
fn whois_panel(scope: &glory::ui::Scope, id: String) -> glory::ui::View {
    let content = scope.cage(String::new());
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    view().class("panel").fill((
        w::label("Sessions and connections"),
        w::status(content),
        w::status(message),
        operation(
            scope,
            "Load / refresh sessions",
            busy,
            message,
            None,
            move || {
                let id = id.clone();
                async move {
                    let data = api::whois_user(&id).await?;
                    let mut lines = Vec::new();
                    for (device, value) in data.devices {
                        for session in value.sessions {
                            for connection in session.connections {
                                lines.push(format!(
                                    "Device {device} · IP {} · last seen {:?} · {}",
                                    connection.ip.unwrap_or_default(),
                                    connection.last_seen,
                                    connection.user_agent.unwrap_or_default()
                                ));
                            }
                        }
                    }
                    if lines.is_empty() {
                        lines.push("No sessions found".into());
                    }
                    write(content, lines.join("\n"));
                    Ok("Sessions loaded".into())
                }
            },
        ),
    ))
}
fn extras_panel(scope: &glory::ui::Scope, id: String) -> glory::ui::View {
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let data = scope.cage(String::new());
    let (rate, rate_input) = w::field(scope, "Messages per second (blank uses default)");
    let (burst, burst_input) = w::field(scope, "Burst count (blank uses default)");
    let features = scope.cage(Vec::<(String, bool)>::new());
    view()
        .class("panel")
        .fill((
            w::label("Account data"),
            w::status(data),
            operation(scope, "Load account data", busy, message, None, {
                let id = id.clone();
                move || {
                    let id = id.clone();
                    async move {
                        let value = user_extras::account_data(&id).await?;
                        write(
                            data,
                            serde_json::to_string_pretty(&value.account_data)
                                .map_err(|e| HttpError::message(e.to_string()))?,
                        );
                        Ok("Account data loaded".into())
                    }
                }
            }),
            w::label("Rate limits"),
            rate_input,
            burst_input,
            operation(scope, "Load rate limits", busy, message, None, {
                let id = id.clone();
                move || {
                    let id = id.clone();
                    async move {
                        let value = user_extras::rate_limits(&id).await?;
                        write(
                            rate,
                            value
                                .messages_per_second
                                .map(|v| v.to_string())
                                .unwrap_or_default(),
                        );
                        write(
                            burst,
                            value.burst_count.map(|v| v.to_string()).unwrap_or_default(),
                        );
                        Ok("Rate limits loaded".into())
                    }
                }
            }),
            operation(scope, "Save rate limits", busy, message, None, {
                let id = id.clone();
                move || {
                    let id = id.clone();
                    async move {
                        user_extras::save_rate_limits(
                            &id,
                            optional_u64(rate.get(), "Messages per second")?,
                            optional_u64(burst.get(), "Burst count")?,
                        )
                        .await?;
                        Ok("Rate limits saved".into())
                    }
                }
            }),
        ))
        .fill((
            w::label("Experimental features"),
            operation(scope, "Load / refresh features", busy, message, None, {
                let id = id.clone();
                move || {
                    let id = id.clone();
                    async move {
                        let value = user_extras::features(&id).await?;
                        let mut values: Vec<_> = value.features.into_iter().collect();
                        values.sort_by(|a, b| a.0.cmp(&b.0));
                        write(features, values);
                        Ok("Experimental features loaded".into())
                    }
                }
            }),
            each(features, |v| v.0.clone(), {
                let scope = scope.clone();
                move |row, _| {
                    let id = id.clone();
                    view().class("record").fill((
                        text().content(row.map(|v| {
                            format!("{}: {}", v.0, if v.1 { "enabled" } else { "disabled" })
                        })),
                        operation(&scope, "Toggle feature", busy, message, None, move || {
                            let id = id.clone();
                            let (name, enabled) = row.get();
                            async move {
                                user_extras::set_feature(&id, &name, !enabled).await?;
                                features.revise(|all| {
                                    if let Some(value) = all.iter_mut().find(|v| v.0 == name) {
                                        value.1 = !enabled;
                                    }
                                });
                                Ok(format!("{name} updated"))
                            }
                        }),
                    ))
                }
            }),
            w::status(message),
        ))
}
fn notice_panel(scope: &glory::ui::Scope, id: String) -> glory::ui::View {
    let (body, body_input) = w::field(scope, "Server notice message");
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    view().class("panel").fill((
        w::label("Send server notice"),
        body_input.multiline(true),
        w::status(message),
        operation(
            scope,
            "Send notice",
            busy,
            message,
            Some("Send this server notice to the user?"),
            move || {
                let id = id.clone();
                async move {
                    let value = required(body.get(), "Message")?;
                    let event = api::send_server_notice(&id, &value).await?;
                    write(body, String::new());
                    Ok(format!("Notice sent: {event}"))
                }
            },
        ),
    ))
}
#[derive(Debug, PartialEq)]
struct ImportRow {
    id: String,
    name: String,
    password: String,
    admin: bool,
    deactivated: bool,
}
fn parse_csv(input: &str) -> Result<Vec<Vec<String>>, HttpError> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut closed = false;
    let mut chars = input.trim_start_matches('\u{feff}').chars().peekable();
    while let Some(ch) = chars.next() {
        if quoted {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    quoted = false;
                    closed = true;
                }
            } else {
                field.push(ch);
            }
            continue;
        }
        if closed && !matches!(ch, ',' | '\r' | '\n') {
            return Err(HttpError::message(
                "Unexpected text after a quoted CSV field",
            ));
        }
        match ch {
            '"' if field.is_empty() => quoted = true,
            '"' => return Err(HttpError::message("A CSV quote must start a field")),
            ',' => {
                row.push(std::mem::take(&mut field));
                closed = false;
            }
            '\n' | '\r' => {
                if ch == '\r' && chars.peek() == Some(&'\n') {
                    chars.next();
                }
                row.push(std::mem::take(&mut field));
                if row.iter().any(|v| !v.is_empty()) {
                    rows.push(std::mem::take(&mut row));
                } else {
                    row.clear();
                }
                closed = false;
            }
            _ => field.push(ch),
        }
    }
    if quoted {
        return Err(HttpError::message("Unterminated quoted CSV field"));
    }
    if !field.is_empty() || !row.is_empty() || closed {
        row.push(field);
        rows.push(row);
    }
    Ok(rows)
}
fn import_rows(input: &str) -> Result<Vec<ImportRow>, HttpError> {
    let mut records = parse_csv(input)?.into_iter();
    let header = records
        .next()
        .ok_or_else(|| HttpError::message("CSV is empty"))?;
    let id = header
        .iter()
        .position(|v| v.trim() == "id")
        .ok_or_else(|| HttpError::message("CSV must contain an id column"))?;
    let column = |name: &str| header.iter().position(|v| v.trim() == name);
    let (name, password, admin, deactivated) = (
        column("displayname"),
        column("password"),
        column("admin"),
        column("deactivated"),
    );
    let mut result = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (index, row) in records.enumerate() {
        if row.len() != header.len() {
            return Err(HttpError::message(format!(
                "CSV row {} has {} columns; expected {}",
                index + 2,
                row.len(),
                header.len()
            )));
        }
        let get = |i: Option<usize>| i.and_then(|i| row.get(i)).cloned().unwrap_or_default();
        let id = required(row[id].clone(), &format!("Row {} user ID", index + 2))?;
        if !seen.insert(id.clone()) {
            return Err(HttpError::message(format!("Duplicate user ID: {id}")));
        }
        let flag = |v: String| match v.trim().to_ascii_lowercase().as_str() {
            "true" | "1" => Ok(true),
            "false" | "0" | "" => Ok(false),
            _ => Err(HttpError::message(format!(
                "Invalid boolean at CSV row {}",
                index + 2
            ))),
        };
        result.push(ImportRow {
            id,
            name: get(name),
            password: get(password),
            admin: flag(get(admin))?,
            deactivated: flag(get(deactivated))?,
        });
        if result.len() > 5000 {
            return Err(HttpError::message("Import at most 5,000 users per batch"));
        }
    }
    if result.is_empty() {
        return Err(HttpError::message("No data rows in CSV"));
    }
    Ok(result)
}
fn import_panel(scope: &glory::ui::Scope) -> glory::ui::View {
    let (csv, csv_input) = w::field(
        scope,
        "CSV content: id,displayname,password,admin,deactivated",
    );
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let preview = scope.cage(String::new());
    let has_pasion = crate::utils::storage::get_item("pasion_url").is_some();
    view().class("panel").fill((w::label("Import users from CSV"),w::label("Passwords left blank are generated securely. Pasion owns administrator privileges when configured. Maximum 5,000 rows."),csv_input.multiline(true),w::status(preview),w::status(message),w::action("Validate import",move||{write(preview,match import_rows(&csv.get()){Ok(rows)=>format!("{} rows; {} administrators; {} generated passwords",rows.len(),rows.iter().filter(|r|r.admin).count(),rows.iter().filter(|r|r.password.is_empty()).count()),Err(e)=>e.to_string()});}),operation(scope,"Import users",busy,message,Some("Create these user accounts from the CSV? Existing users will be skipped, never overwritten."),move||async move{
        let rows=import_rows(&csv.get())?;let mut created=0;let mut skipped=0;let mut failures=Vec::new();
        for row in rows {
            match api::check_username_available(&row.id).await{Ok(false)=>{skipped+=1;continue;},Err(e)=>{failures.push(format!("{}: {e}",row.id));continue;},Ok(true)=>{}}
            let password=if row.password.is_empty(){crate::utils::password::generate_random_password()}else{row.password};
            match api::create_user(&row.id,CreateUserRequest{password:Some(password),displayname:Some(row.name),admin:!has_pasion&&row.admin,deactivated:Some(row.deactivated),..Default::default()}).await{Ok(_)=>created+=1,Err(e)=>failures.push(format!("{}: {e}",row.id))}
        }
        let summary=format!("Import: {created} created, {skipped} existing users skipped, {} failed",failures.len());
        if failures.is_empty(){write(csv,String::new());Ok(summary)}else{Err(HttpError::message(format!("{summary}\n{}",failures.join("\n"))))}
    })))
}
fn csv_cell(value: &str) -> String {
    let value = if value.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        format!("'{value}")
    } else {
        value.to_string()
    };
    format!("\"{}\"", value.replace('"', "\"\""))
}
fn export_panel(scope: &glory::ui::Scope) -> glory::ui::View {
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    view().class("panel").fill((
        w::label("Export users"),
        w::status(message),
        operation(
            scope,
            "Download all users as CSV",
            busy,
            message,
            None,
            move || async move {
                let users = api::get_all_users_for_export("name", "ASC").await?;
                let mut output =
                    String::from("user_id,displayname,admin,deactivated,creation_ts\r\n");
                for row in &users {
                    output.push_str(&format!(
                        "{},{},{},{},{}\r\n",
                        csv_cell(&row.id),
                        csv_cell(row.user.displayname.as_deref().unwrap_or("")),
                        row.user.admin,
                        row.user.deactivated,
                        row.creation_ts_ms
                    ));
                }
                w::download_text("users_export.csv", &output, "text/csv;charset=utf-8")?;
                Ok(format!("Exported {} users", users.len()))
            },
        ),
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn csv_export_keeps_user_values_inert_in_spreadsheets() {
        for value in [
            "=SUM(1,2)",
            "+1",
            "-1",
            "@alice:server",
            "\tformula",
            "\rformula",
        ] {
            assert!(csv_cell(value).starts_with("\"'"));
        }
        assert_eq!(csv_cell("Alice \"A\""), "\"Alice \"\"A\"\"\"");
    }
    #[test]
    fn csv_supports_quotes_and_multiline_names() {
        let rows=import_rows("id,displayname,password,admin,deactivated\r\nalice,\"Alice, \"\"A\"\"\nAdmin\",secret,false,true\r\n").unwrap();
        assert_eq!(rows[0].name, "Alice, \"A\"\nAdmin");
        assert!(rows[0].deactivated);
    }
    #[test]
    fn csv_rejects_malformed_or_ambiguous_rows() {
        for value in [
            "id,admin\na,maybe",
            "id\na\na",
            "id,name\na",
            "id\n\"unfinished",
        ] {
            assert!(import_rows(value).is_err(), "{value}");
        }
    }
    #[test]
    fn contact_validation() {
        assert!(validate_contacts("a@example.org", "+861234567890").is_ok());
        assert!(validate_contacts("a@", "bad").is_err());
    }
}

async fn grant_admin(id: &str) -> Result<(), HttpError> {
    if crate::utils::storage::get_item("pasion_url").is_some() {
        let username = id
            .trim_start_matches('@')
            .split(':')
            .next()
            .unwrap_or_default();
        let account = crate::api::pasion::pasion_get_user_by_username(username).await?;
        crate::api::pasion::pasion_update_user(&account.id, serde_json::json!({"admin":true}))
            .await?;
    } else {
        api::update_user(id, serde_json::json!({"admin":true})).await?;
    }
    Ok(())
}
