use super::users::{
    collection_window, column, operation, page_options, page_size, remember, required, stored,
    toggle, write,
};
use crate::ui::widgets as w;
use crate::{api::rooms as api, types::*, utils::error::HttpError};
use glory::prelude::*;
use std::rc::Rc;
use std::result::Result;

fn room_path(id: &str) -> String {
    format!("/rooms/{}", urlencoding::encode(id))
}
pub fn render(scope: &glory::ui::Scope, route: &crate::router::Route) -> glory::ui::View {
    if route.path == "/rooms/:room_id" {
        detail(scope, route.params.first().cloned().unwrap_or_default())
    } else {
        list(scope)
    }
}
fn list(scope: &glory::ui::Scope) -> glory::ui::View {
    let (search, search_input) = w::field(scope, "Search rooms");
    write(search, stored("rooms.search", ""));
    let page = scope.cage(stored("rooms.page", "1").parse::<u64>().unwrap_or(1).max(1));
    let size = page_size(scope, "rooms.size");
    let total = scope.cage(0u64);
    let (show_metadata, metadata_column) =
        column(scope, "rooms.column.metadata", "Show room metadata column");
    let rows = scope.cage(Vec::<RoomRecord>::new());
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let order = scope.cage(stored("rooms.sort", "name"));
    let descending = scope.cage(stored("rooms.desc", "false") == "true");
    let applied_search = scope.cage(search.get());
    let accepted = scope.cage((
        page.get(),
        size.get(),
        order.get(),
        descending.get(),
        search.get(),
    ));
    let selection = scope.cage(Vec::<String>::new());
    let kind = scope.cage(stored("rooms.kind", "all"));
    let visible = rows.map(move |rows| {
        rows.iter()
            .filter(|row| match kind.get().as_str() {
                "public" => row.room.public,
                "private" => !row.room.public,
                "encrypted" => row.is_encrypted,
                _ => true,
            })
            .cloned()
            .collect::<Vec<_>>()
    });
    let load: Rc<dyn Fn()> = {
        let scope = scope.clone();
        Rc::new(move || {
            if busy.get() {
                return;
            }
            write(busy, true);
            write(selection, Vec::new());
            write(message, "Loading rooms…".into());
            let requested_size = size.get();
            let (p, s, o, d) = (
                page.get(),
                applied_search.get(),
                order.get(),
                descending.get(),
            );
            remember("rooms.page", &p.to_string());
            remember("rooms.search", &s);
            remember("rooms.sort", &o);
            remember("rooms.desc", &d.to_string());
            w::spawn(&scope, async move {
                match api::get_rooms(p, requested_size, &o, if d { "DESC" } else { "ASC" }, &s)
                    .await
                {
                    Ok(result) => {
                        write(accepted, (p, requested_size, o.clone(), d, s.clone()));
                        write(total, result.total);
                        write(
                            message,
                            if result.data.is_empty() {
                                "No rooms found".into()
                            } else {
                                format!("{} rooms · page {p}", result.total)
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
                        remember("rooms.page", &p.to_string());
                        remember("rooms.size", &n.to_string());
                        remember("rooms.sort", &o);
                        remember("rooms.desc", &d.to_string());
                        remember("rooms.search", &s);
                        write(message, e.to_string());
                    }
                }
                write(busy, false);
            });
        })
    };
    load();
    let mut out = w::page("Rooms").fill((
        search_input,
        metadata_column,
        page_options(size, page, busy, "rooms.size", load.clone()),
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
    ));
    for (label, key) in [
        ("Sort by room name", "name"),
        ("Sort by member count", "joined_members"),
        ("Sort by room ID", "room_id"),
        ("Sort by state events", "state_events"),
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
    for (caption, value) in [
        ("All room types", "all"),
        ("Public rooms", "public"),
        ("Private rooms", "private"),
        ("Encrypted rooms", "encrypted"),
    ] {
        out = out.fill(w::action(caption, move || {
            write(kind, value.into());
            remember("rooms.kind", value);
            write(selection, Vec::new());
        }));
    }
    for (caption, deleting) in [
        ("Delete selected rooms", true),
        ("Block selected rooms", false),
    ] {
        out = out.fill(operation(
            scope,
            caption,
            busy,
            message,
            Some("Apply this operation to every selected room? Room deletion cannot be undone."),
            move || {
                let ids = selection.get();
                async move {
                    if ids.is_empty() {
                        return Err(HttpError::message("Select at least one room"));
                    }
                    let mut succeeded = 0;
                    let mut errors = Vec::new();
                    for id in ids {
                        let result = if deleting {
                            api::delete_room(&id, false).await
                        } else {
                            api::block_room(&id, true).await
                        };
                        match result {
                            Ok(()) => succeeded += 1,
                            Err(error) => errors.push(format!("{id}: {error}")),
                        }
                    }
                    let data = api::get_rooms(
                        page.get(),
                        size.get(),
                        &order.get(),
                        if descending.get() { "DESC" } else { "ASC" },
                        &applied_search.get(),
                    )
                    .await?;
                    write(rows, data.data);
                    write(total, data.total);
                    write(selection, Vec::new());
                    if errors.is_empty() {
                        Ok(format!("Applied to {succeeded} rooms; list refreshed"))
                    } else {
                        Err(HttpError::message(format!(
                            "Applied to {succeeded} rooms; {} failed: {}",
                            errors.len(),
                            errors.join("; ")
                        )))
                    }
                }
            },
        ));
    }
    out.fill((
        w::action("Select all rooms on this page", move || {
            write(
                selection,
                visible.with(|rows| rows.iter().map(|r| r.id.clone()).collect()),
            )
        }),
        w::action("Clear room selection", move || write(selection, Vec::new())),
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
            visible,
            |r| r.id.clone(),
            move |row, _| {
                view().class("record").fill((
                    check()
                        .label(row.map(|r| format!("Select room {}", r.id)))
                        .checked(row.map(move |r| selection.with(|ids| ids.contains(&r.id))))
                        .on(checked_changed, move |event| {
                            if let Event::CheckedChanged(checked) = event {
                                let id = row.get().id;
                                selection.revise(|ids| {
                                    ids.retain(|v| v != &id);
                                    if checked {
                                        ids.push(id);
                                    }
                                });
                            }
                        }),
                    text().content(row.map(move |r| {
                        if !show_metadata.get() {
                            return format!(
                                "{} · {}",
                                r.room.name.as_deref().unwrap_or("Unnamed room"),
                                r.id
                            );
                        }
                        format!(
                            "{} · {} · {} members · {}{}",
                            r.room.name.as_deref().unwrap_or("Unnamed room"),
                            r.id,
                            r.members,
                            if r.is_encrypted { "encrypted " } else { "" },
                            if r.room.is_blocked_effective() {
                                "blocked"
                            } else {
                                ""
                            }
                        )
                    })),
                    w::action("Room details", move || {
                        crate::ui::navigate(&room_path(&row.get().id))
                    }),
                ))
            },
        ),
        create_panel(scope),
    ))
}
fn create_panel(scope: &glory::ui::Scope) -> glory::ui::View {
    let (name, name_input) = w::field(scope, "Room name");
    let (topic, topic_input) = w::field(scope, "Room topic");
    let public = scope.cage(true);
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    view().class("panel").fill((
        w::label("Create room"),
        name_input,
        topic_input,
        toggle(scope, "Public room", public),
        w::status(message),
        operation(
            scope,
            "Create room",
            busy,
            message,
            None,
            move || async move {
                let name = required(name.get(), "Room name")?;
                let id = api::create_room(&name, &topic.get(), public.get()).await?;
                crate::ui::navigate(&room_path(&id));
                Ok(format!("Created {id}"))
            },
        ),
    ))
}
fn room_summary(data: &RoomRecord) -> String {
    let r = &data.room;
    format!(
        "{}\n{}\nAlias: {}\nMembers: {} · Version: {} · Creator: {}\nEncrypted: {} · Federatable: {} · Blocked: {}\nJoin rules: {} · History: {} · Guest access: {}\nState events: {}",
        data.id,
        r.name.as_deref().unwrap_or("Unnamed room"),
        data.alias.as_deref().unwrap_or(""),
        data.members,
        r.version.as_deref().unwrap_or(""),
        r.creator.as_deref().unwrap_or(""),
        data.is_encrypted,
        r.federatable,
        r.is_blocked_effective(),
        r.join_rules.as_deref().unwrap_or(""),
        r.history_visibility.as_deref().unwrap_or(""),
        r.guest_access.as_deref().unwrap_or(""),
        r.state_events
    )
}
fn detail(scope: &glory::ui::Scope, id: String) -> glory::ui::View {
    let info = scope.cage("Loading room…".to_string());
    let message = scope.cage(String::new());
    let busy = scope.cage(false);
    let loaded = scope.cage(false);
    let (name, name_input) = w::field(scope, "Room name");
    let (topic, topic_input) = w::field(scope, "Room topic");
    let public = scope.cage(false);
    let load = {
        let id = id.clone();
        move || {
            let id = id.clone();
            async move {
                let data = api::get_room(&id).await?;
                write(info, room_summary(&data));
                write(name, data.room.name.unwrap_or_default());
                write(topic, data.room.topic.unwrap_or_default());
                write(loaded, true);
                Ok::<String, HttpError>("Room loaded".into())
            }
        }
    };
    let first = load();
    w::submit(scope, busy, message, first);
    let mut out = w::page(&format!("Room {id}")).fill((
        w::status(info),
        w::status(message),
        operation(scope, "Refresh room", busy, message, None, load.clone()),
        name_input,
        operation(scope, "Save room name", busy, message, None, {
            let id = id.clone();
            let reload = load.clone();
            move || {
                let id = id.clone();
                let reload = reload.clone();
                async move {
                    if !loaded.get() {
                        return Err(HttpError::message("Load the room before editing"));
                    }
                    api::set_room_name(&id, &name.get()).await?;
                    reload().await?;
                    Ok("Room name saved".into())
                }
            }
        }),
        topic_input.multiline(true),
        operation(scope, "Save room topic", busy, message, None, {
            let id = id.clone();
            let reload = load.clone();
            move || {
                let id = id.clone();
                let reload = reload.clone();
                async move {
                    api::set_room_topic(&id, &topic.get()).await?;
                    reload().await?;
                    Ok("Room topic saved".into())
                }
            }
        }),
        toggle(scope, "Published in room directory", public),
        operation(scope, "Load directory visibility", busy, message, None, {
            let id = id.clone();
            move || {
                let id = id.clone();
                async move {
                    write(public, api::get_room_directory_visibility(&id).await?);
                    Ok("Directory visibility loaded".into())
                }
            }
        }),
        operation(scope, "Save directory visibility", busy, message, None, {
            let id = id.clone();
            let reload = load.clone();
            move || {
                let id = id.clone();
                let reload = reload.clone();
                async move {
                    api::set_room_directory_visibility(&id, public.get()).await?;
                    reload().await?;
                    Ok("Directory visibility saved".into())
                }
            }
        }),
    ));
    let (join, join_input) = w::field(
        scope,
        "Join rule: public, invite, knock, restricted, knock_restricted",
    );
    let (history, history_input) = w::field(
        scope,
        "History visibility: world_readable, shared, invited, joined",
    );
    out = out.fill((
        join_input,
        operation(scope, "Save join rule", busy, message, None, {
            let id = id.clone();
            let reload = load.clone();
            move || {
                let id = id.clone();
                let reload = reload.clone();
                async move {
                    let value = join.get();
                    if ![
                        "public",
                        "invite",
                        "knock",
                        "restricted",
                        "knock_restricted",
                    ]
                    .contains(&value.as_str())
                    {
                        return Err(HttpError::message("Choose a supported join rule"));
                    }
                    api::set_room_join_rules(&id, &value).await?;
                    reload().await?;
                    Ok("Join rule saved".into())
                }
            }
        }),
        history_input,
        operation(scope, "Save history visibility", busy, message, None, {
            let id = id.clone();
            let reload = load.clone();
            move || {
                let id = id.clone();
                let reload = reload.clone();
                async move {
                    let value = history.get();
                    if !["world_readable", "shared", "invited", "joined"].contains(&value.as_str())
                    {
                        return Err(HttpError::message("Choose a supported history visibility"));
                    }
                    api::set_room_history_visibility(&id, &value).await?;
                    reload().await?;
                    Ok("History visibility saved".into())
                }
            }
        }),
    ));
    out.fill((
        aliases(scope, id.clone(), info),
        members(scope, id.clone()),
        events(scope, id.clone()),
        hierarchy(scope, id.clone()),
        danger(scope, id, info),
    ))
}
fn aliases(scope: &glory::ui::Scope, id: String, info: Cage<String>) -> glory::ui::View {
    let (alias, alias_input) = w::field(scope, "Room alias (#name:server)");
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    view().class("panel").fill((
        w::label("Room aliases"),
        alias_input,
        w::status(message),
        operation(scope, "Add alias", busy, message, None, {
            let id = id.clone();
            move || {
                let id = id.clone();
                async move {
                    let value = valid_alias(alias.get())?;
                    api::put_room_alias(&value, &id).await?;
                    write(info, room_summary(&api::get_room(&id).await?));
                    Ok("Alias added".into())
                }
            }
        }),
        operation(scope, "Set canonical alias", busy, message, None, {
            let id = id.clone();
            move || {
                let id = id.clone();
                async move {
                    let value = valid_alias(alias.get())?;
                    api::set_room_state(
                        &id,
                        "m.room.canonical_alias",
                        "",
                        serde_json::json!({"alias":value}),
                    )
                    .await?;
                    write(info, room_summary(&api::get_room(&id).await?));
                    Ok("Canonical alias saved".into())
                }
            }
        }),
        operation(
            scope,
            "Remove alias",
            busy,
            message,
            Some("Remove this room alias?"),
            move || {
                let id = id.clone();
                async move {
                    let value = valid_alias(alias.get())?;
                    api::delete_room_alias(&value).await?;
                    write(info, room_summary(&api::get_room(&id).await?));
                    Ok("Alias removed".into())
                }
            },
        ),
    ))
}
fn valid_alias(alias: String) -> Result<String, HttpError> {
    let alias = required(alias, "Alias")?;
    if !alias.starts_with('#') || !alias.contains(':') || alias.chars().any(char::is_whitespace) {
        return Err(HttpError::message("Use #alias:server"));
    }
    Ok(alias)
}
fn members(scope: &glory::ui::Scope, id: String) -> glory::ui::View {
    let page = scope.cage(1u64);
    let total = scope.cage(0u64);
    let rows = scope.cage(Vec::<String>::new());
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let (user, user_input) = w::field(scope, "Member user ID (@user:server)");
    let (reason, reason_input) = w::field(scope, "Reason");
    let load = {
        let id = id.clone();
        move || {
            let id = id.clone();
            async move {
                let result = api::get_room_members(&id, page.get(), 25).await?;
                write(total, result.total);
                write(rows, result.data);
                Ok(format!("{} members · page {}", result.total, page.get()))
            }
        }
    };
    let prev = load.clone();
    let next = load.clone();
    let ps = scope.clone();
    let ns = scope.clone();
    let mut out = view().class("panel").fill((
        w::label("Members and invitations"),
        w::status(message),
        operation(scope, "Load / refresh members", busy, message, None, load),
        w::action("Previous member page", move || {
            if !busy.get() && page.get() > 1 {
                page.revise(|p| *p -= 1);
                w::submit(&ps, busy, message, prev());
            }
        }),
        w::action("Next member page", move || {
            if !busy.get() && page.get() * 25 < total.get() {
                page.revise(|p| *p += 1);
                w::submit(&ns, busy, message, next());
            }
        }),
        each(
            rows,
            |v| v.clone(),
            move |row, _| {
                view().class("record").fill((
                    text().content(row),
                    w::action("Select member", move || write(user, row.get())),
                    w::action("View known user", move || {
                        crate::ui::navigate(&format!(
                            "/users/known/{}",
                            urlencoding::encode(&row.get())
                        ))
                    }),
                ))
            },
        ),
        user_input,
        reason_input,
    ));
    for (title, kind) in [
        ("Invite member", 0),
        ("Kick member", 1),
        ("Ban member", 2),
        ("Unban member", 3),
        ("Make room administrator", 4),
    ] {
        let id = id.clone();
        out = out.fill(operation(
            scope,
            title,
            busy,
            message,
            Some("Apply this membership or privilege change to the specified user?"),
            move || {
                let id = id.clone();
                async move {
                    let user = required(user.get(), "User ID")?;
                    if !crate::utils::mxid::is_mxid(&user) {
                        return Err(HttpError::message(
                            "Use a complete Matrix user ID (@user:server)",
                        ));
                    }
                    match kind {
                        0 => api::invite_user(&id, &user).await?,
                        1 => api::kick_user(&id, &user, &reason.get()).await?,
                        2 => api::ban_user(&id, &user, &reason.get()).await?,
                        3 => api::unban_user(&id, &user).await?,
                        _ => api::make_room_admin(&id, &user).await?,
                    }
                    let fresh = api::get_room_members(&id, page.get(), 25).await?;
                    write(total, fresh.total);
                    write(rows, fresh.data);
                    Ok("Member operation completed; members refreshed.".into())
                }
            },
        ));
    }
    out
}
fn events(scope: &glory::ui::Scope, id: String) -> glory::ui::View {
    let states = scope.cage(Vec::<RoomState>::new());
    let messages = scope.cage(Vec::<RoomMessage>::new());
    let (visible_states, state_pages) = collection_window(scope, states, "state events");
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let (event, event_input) = w::field(scope, "Event ID");
    let (reason, reason_input) = w::field(scope, "Redaction reason");
    let fetched = scope.cage(String::new());
    view()
        .class("panel")
        .fill((
            w::label("State events"),
            operation(scope, "Load / refresh state", busy, message, None, {
                let id = id.clone();
                move || {
                    let id = id.clone();
                    async move {
                        let result = api::get_room_state(&id).await?;
                        let count = result.len();
                        write(states, result);
                        Ok(format!("{count} state events loaded"))
                    }
                }
            }),
            state_pages,
            each(
                visible_states,
                |s| s.event_id.clone(),
                move |row, _| {
                    view().class("record").fill((
                        text().content(row.map(|s| {
                            format!(
                                "{} · {} · {}\n{}",
                                s.event_type, s.event_id, s.sender, s.content
                            )
                        })),
                        w::action("Select state event", move || {
                            write(event, row.get().event_id)
                        }),
                    ))
                },
            ),
        ))
        .fill((
            w::label("Recent messages (latest 100)"),
            operation(scope, "Load / refresh messages", busy, message, None, {
                let id = id.clone();
                move || {
                    let id = id.clone();
                    async move {
                        let result = api::get_room_messages(&id, 100).await?;
                        let count = result.len();
                        write(messages, result);
                        Ok(format!("{count} messages loaded"))
                    }
                }
            }),
            each(
                messages,
                |m| m.event_id.clone(),
                move |row, _| {
                    view().class("record").fill((
                        text().content(row.map(|m| {
                            format!(
                                "{} · {} · {}\n{}",
                                m.sender,
                                m.event_id,
                                m.origin_server_ts,
                                m.content.body.as_deref().unwrap_or("Non-text event")
                            )
                        })),
                        w::action("Select message", move || write(event, row.get().event_id)),
                    ))
                },
            ),
            event_input,
            reason_input,
            operation(
                scope,
                "Fetch event",
                busy,
                message,
                None,
                move || async move {
                    let event = required(event.get(), "Event ID")?;
                    write(
                        fetched,
                        serde_json::to_string_pretty(&api::fetch_event(&event).await?)
                            .map_err(|e| HttpError::message(e.to_string()))?,
                    );
                    Ok("Event loaded".into())
                },
            ),
            w::status(fetched),
            operation(
                scope,
                "Redact event",
                busy,
                message,
                Some("Permanently redact the selected event?"),
                move || {
                    let id = id.clone();
                    async move {
                        let event = required(event.get(), "Event ID")?;
                        let txn = crate::utils::password::generate_device_id();
                        api::redact_event(&id, &event, &txn, &reason.get()).await?;
                        write(messages, api::get_room_messages(&id, 100).await?);
                        write(states, api::get_room_state(&id).await?);
                        Ok("Event redacted; messages and state refreshed".into())
                    }
                },
            ),
            w::status(message),
        ))
}
fn hierarchy(scope: &glory::ui::Scope, id: String) -> glory::ui::View {
    let rows = scope.cage(Vec::<HierarchyRoom>::new());
    let extremities = scope.cage(Vec::<ForwardExtremity>::new());
    let (visible_hierarchy, hierarchy_pages) = collection_window(scope, rows, "hierarchy rooms");
    let (visible_extremities, extremity_pages) =
        collection_window(scope, extremities, "extremities");
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    view().class("panel").fill((
        w::label("Room hierarchy"),
        operation(scope, "Load / refresh hierarchy", busy, message, None, {
            let id = id.clone();
            move || {
                let id = id.clone();
                async move {
                    let result = api::get_room_hierarchy(&id).await?;
                    let count = result.len();
                    write(rows, result);
                    Ok(format!("{count} hierarchy rooms"))
                }
            }
        }),
        hierarchy_pages,
        each(
            visible_hierarchy,
            |r| r.room_id.clone(),
            |row, _| {
                view().class("record").fill((
                    text().content(row.map(|r| {
                        format!(
                            "{} · {} · {} members\n{}",
                            r.name.as_deref().unwrap_or("Unnamed room"),
                            r.room_id,
                            r.num_joined_members,
                            r.topic.as_deref().unwrap_or("")
                        )
                    })),
                    w::action("Open hierarchy room", move || {
                        crate::ui::navigate(&room_path(&row.get().room_id))
                    }),
                ))
            },
        ),
        w::label("Forward extremities"),
        operation(
            scope,
            "Load / refresh extremities",
            busy,
            message,
            None,
            move || {
                let id = id.clone();
                async move {
                    let result = api::get_forward_extremities(&id).await?;
                    write(extremities, result.results);
                    Ok(format!("{} forward extremities", result.count))
                }
            },
        ),
        extremity_pages,
        each(
            visible_extremities,
            |e| e.event_id.clone(),
            |row, _| {
                text().content(row.map(|e| {
                    format!(
                        "{} · depth {} · state group {:?} · received {}",
                        e.event_id, e.depth, e.state_group, e.received_ts
                    )
                }))
            },
        ),
        w::status(message),
    ))
}
fn danger(scope: &glory::ui::Scope, id: String, info: Cage<String>) -> glory::ui::View {
    let context = crate::ui::context::get(scope);
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let block = scope.cage(true);
    let (timestamp, timestamp_input) =
        w::field(scope, "Purge history before Unix timestamp in milliseconds");
    let mut out = view()
        .class("panel danger")
        .fill((w::label("Dangerous room operations"), w::status(message)));
    for (title, blocked) in [("Block room", true), ("Unblock room", false)] {
        let id = id.clone();
        out = out.fill(operation(
            scope,
            title,
            busy,
            message,
            Some("Change room blocking? Blocking prevents local users joining this room."),
            move || {
                let id = id.clone();
                async move {
                    api::block_room(&id, blocked).await?;
                    write(info, room_summary(&api::get_room(&id).await?));
                    Ok(if blocked {
                        "Room blocked"
                    } else {
                        "Room unblocked"
                    }
                    .into())
                }
            },
        ));
    }
    out.fill((
        timestamp_input,
        operation(
            scope,
            "Purge room history",
            busy,
            message,
            Some("Permanently purge room history before this timestamp?"),
            {
                let id = id.clone();
                move || {
                    let id = id.clone();
                    async move {
                        let timestamp = timestamp.get().parse::<u64>().map_err(|_| {
                            HttpError::message("Enter a positive Unix timestamp in milliseconds")
                        })?;
                        if timestamp == 0 {
                            return Err(HttpError::message("Timestamp must be greater than zero"));
                        }
                        api::purge_history(&id, timestamp).await?;
                        Ok("History purge accepted".into())
                    }
                }
            },
        ),
        toggle(scope, "Block future joins when deleting", block),
        operation(
            scope,
            "Delete room",
            busy,
            message,
            Some("Delete this room and remove its local users? This cannot be undone."),
            move || {
                let id = id.clone();
                let context = context.clone();
                async move {
                    api::delete_room(&id, block.get()).await?;
                    if let Some(context) = context {
                        context.notify("system_info", format!("Room {id} deleted"), false);
                    }
                    crate::ui::navigate("/rooms");
                    Ok("Room deletion accepted".into())
                }
            },
        ),
    ))
}
