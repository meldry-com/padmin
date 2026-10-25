//! Dedicated Pasion administration workflows, backed by the Pasion admin API.
use super::users::{collection_window, remember, stored};
use crate::ui::widgets::{action, confirm, field, label, load_quiet, page, spawn, status, submit};
use crate::{api::pasion::*, types::*, utils::error::HttpError};
use glory::{core::Cage, ui::*};
use std::result::Result;
use std::{future::Future, rc::Rc};

mod account;
mod provider;

pub fn render(scope: &Scope, route: &crate::router::Route) -> View {
    match route.path.as_str() {
        "/pasion/accounts" => accounts(scope),
        "/pasion/accounts/:user_id" => {
            account::render(scope, route.params.first().cloned().unwrap_or_default())
        }
        "/pasion/audit-log" => audit(scope),
        "/pasion/oauth2-sessions" => oauth(scope),
        "/pasion/personal-sessions" => personal(scope),
        "/pasion/connector-health" => health(scope),
        "/pasion/upstream-providers" => provider::render(scope),
        "/pasion/upstream-links" => links(scope),
        "/pasion/notification-channels" => channels(scope),
        "/pasion/notification-templates" => templates(scope),
        _ => page("Pasion").fill(label("Unknown Pasion page")),
    }
}

fn check_field(scope: &Scope, title: &str, initial: bool) -> (Cage<bool>, Check) {
    let value = scope.cage(initial);
    (
        value,
        check()
            .label(crate::ui::widgets::localized(scope, title.into()))
            .checked(value)
            .on(events::checked_changed, move |event| {
                if let Event::CheckedChanged(checked) = event {
                    value.revise(|slot| *slot = checked);
                }
            }),
    )
}

fn load<T: Clone + 'static>(
    scope: &Scope,
    busy: Cage<bool>,
    message: Cage<String>,
    data: Cage<Vec<T>>,
    future: impl Future<Output = Result<Vec<T>, HttpError>> + 'static,
) {
    if busy.get() {
        return;
    }
    busy.revise(|slot| *slot = true);
    message.revise(|slot| *slot = "Loading…".into());
    spawn(scope, async move {
        match future.await {
            Ok(rows) => {
                let n = rows.len();
                let _ = data.try_revise(|slot| *slot = rows);
                let _ = message.try_revise(|slot| {
                    *slot = if n == 0 {
                        "No results".into()
                    } else {
                        format!("{n} results")
                    }
                });
            }
            Err(error) => {
                let _ = message.try_revise(|slot| *slot = error.to_string());
            }
        }
        let _ = busy.try_revise(|slot| *slot = false);
    });
}

fn optional(value: String) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.trim().to_string())
}
fn nullable(value: String) -> serde_json::Value {
    optional(value)
        .map(Into::into)
        .unwrap_or(serde_json::Value::Null)
}
fn account_path(id: &str) -> String {
    format!("/pasion/accounts/{}", urlencoding::encode(id))
}
fn detail(title: &str, value: impl Into<String>) -> crate::ui::widgets::LocalText {
    label(format!("{title}: {}", value.into()))
}
fn required(value: String, name: &str) -> Result<String, HttpError> {
    optional(value).ok_or_else(|| HttpError::message(format!("{name} is required")))
}

fn accounts(scope: &Scope) -> View {
    let data = scope.cage(Vec::<PasionUser>::new());
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let (search, search_input) = field(scope, "Search username");
    let (state, _state_input) = field(scope, "Status: blank, active, locked, deactivated");
    let (admin, _admin_input) = field(scope, "Administrator: blank, true, false");
    search.revise(|v| *v = stored("pasion.accounts.search", ""));
    state.revise(|v| *v = stored("pasion.accounts.state", ""));
    admin.revise(|v| *v = stored("pasion.accounts.admin", ""));
    let saved_cursor =
        serde_json::from_str::<Option<String>>(&stored("pasion.accounts.cursor", "null"))
            .unwrap_or_default();
    let saved_history =
        serde_json::from_str::<Vec<Option<String>>>(&stored("pasion.accounts.history", "[]"))
            .unwrap_or_default();
    let cursor = scope.cage(saved_cursor.clone());
    let history = scope.cage(saved_history.clone());
    let next = scope.cage(None::<String>);
    let applied_filter = scope.cage(PasionUserFilter::default());
    let apply_pending = scope.cage(true);
    let accepted_page = scope.cage((
        None::<String>,
        Vec::<Option<String>>::new(),
        PasionUserFilter::default(),
    ));
    let total = scope.cage(String::new());
    let s = scope.clone();
    let refresh: Rc<dyn Fn()> = Rc::new(move || {
        let rollback = || {
            let (old_cursor, old_history, old_filter) = accepted_page.get();
            cursor.revise(|slot| *slot = old_cursor);
            history.revise(|slot| *slot = old_history);
            applied_filter.revise(|slot| *slot = old_filter);
        };
        let f = if apply_pending.get() {
            PasionUserFilter {
                search: optional(search.get()),
                status: optional(state.get()),
                admin: match admin.get().as_str() {
                    "true" => Some(true),
                    "false" => Some(false),
                    "" => None,
                    _ => {
                        message.revise(|slot| {
                            *slot = "Administrator must be blank, true or false".into()
                        });
                        rollback();
                        return;
                    }
                },
            }
        } else {
            applied_filter.get()
        };
        if f.status
            .as_deref()
            .is_some_and(|v| !["active", "locked", "deactivated"].contains(&v))
        {
            message.revise(|slot| *slot = "Unknown account status".into());
            rollback();
            return;
        }
        applied_filter.revise(|slot| *slot = f.clone());
        apply_pending.revise(|slot| *slot = false);
        let after = cursor.get();
        let proposed_page = (after.clone(), history.get(), f.clone());
        load(&s, busy, message, data, async move {
            let response = match pasion_get_users(&f, after.as_deref(), 25).await {
                Ok(response) => response,
                Err(error) => {
                    if let Ok((previous_cursor, previous_history, previous_filter)) =
                        accepted_page.try_get()
                    {
                        let _ = cursor.try_revise(|slot| *slot = previous_cursor);
                        let _ = history.try_revise(|slot| *slot = previous_history);
                        let _ = applied_filter.try_revise(|slot| *slot = previous_filter);
                    }
                    return Err(error);
                }
            };
            remember("pasion.accounts.search", f.search.as_deref().unwrap_or(""));
            remember("pasion.accounts.state", f.status.as_deref().unwrap_or(""));
            remember(
                "pasion.accounts.admin",
                f.admin
                    .map(|v| if v { "true" } else { "false" })
                    .unwrap_or(""),
            );
            remember(
                "pasion.accounts.cursor",
                &serde_json::to_string(&proposed_page.0).unwrap_or_default(),
            );
            remember(
                "pasion.accounts.history",
                &serde_json::to_string(&proposed_page.1).unwrap_or_default(),
            );
            let _ = accepted_page.try_revise(|slot| *slot = proposed_page);
            let _ = total.try_revise(|slot| {
                *slot = response
                    .count
                    .map(|n| format!("Total matching accounts: {n}"))
                    .unwrap_or_default()
            });
            let _ = next.try_revise(|slot| *slot = response.next_cursor);
            Ok(response.data)
        });
    });
    refresh();
    let apply = refresh.clone();
    let forward = refresh.clone();
    let back = refresh.clone();
    let (username, username_input) = field(scope, "New account username");
    let (password, password_input) = field(scope, "Initial password (optional)");
    let (skip, skip_input) = check_field(scope, "Skip password policy", false);
    let (make_admin, admin_check) = check_field(scope, "Grant administrator", false);
    let writing = scope.cage(false);
    let s = scope.clone();
    let create_open = scope.cage(false);
    let table_data = data.map(|rows| vec![rows.clone()]);
    page("Local Accounts")
        .class("page list-page accounts-list")
        .fill((
            label(
                "Accounts managed by Pasion: roles, lock and deactivation, passwords and sessions",
            )
            .class("page-description"),
            view().class("page-actions").fill(
                action("Create account", move || create_open.revise(|v| *v = !*v))
                    .class("btn-primary"),
            ),
            view().class("list-toolbar").fill((
                search_input,
                sapid::native_select("account-status")
                    .unstyled()
                    .name("Account status")
                    .value(state.map(|v| {
                        Some(if v.is_empty() {
                            "all".into()
                        } else {
                            v.clone()
                        })
                    }))
                    .options([
                        sapid::select_option("all", "All statuses"),
                        sapid::select_option("active", "Active"),
                        sapid::select_option("locked", "Locked"),
                        sapid::select_option("deactivated", "Deactivated"),
                    ])
                    .on_value_change(move |v| {
                        state.revise(|slot| *slot = v.filter(|v| v != "all").unwrap_or_default())
                    }),
                sapid::native_select("account-admin")
                    .unstyled()
                    .name("Account role")
                    .value(admin.map(|v| {
                        Some(if v.is_empty() {
                            "all".into()
                        } else {
                            v.clone()
                        })
                    }))
                    .options([
                        sapid::select_option("all", "All roles"),
                        sapid::select_option("true", "Administrator"),
                        sapid::select_option("false", "Regular"),
                    ])
                    .on_value_change(move |v| {
                        admin.revise(|slot| *slot = v.filter(|v| v != "all").unwrap_or_default())
                    }),
                action("Search / apply filters", move || {
                    if busy.get() {
                        return;
                    }
                    cursor.revise(|slot| *slot = None);
                    history.revise(|slot| *slot = Vec::new());
                    apply_pending.revise(|slot| *slot = true);
                    apply();
                })
                .enabled(busy.map(|b| !b)),
            )),
            view().class("table-container").fill((
                each(
                    table_data,
                    |rows| serde_json::to_string(rows).unwrap_or_default(),
                    move |values, _| {
                        widget(move |scope| {
                            let mut head = sapid::table_row("head");
                            for (key, title) in [
                                ("username", "Username"),
                                ("name", "Display name"),
                                ("status", "Status"),
                                ("created", "Created"),
                                ("actions", ""),
                            ] {
                                head = head.cell(sapid::table_header_cell(key).text(title));
                            }
                            let mut body = sapid::table_body();
                            for (index, row) in values.get().into_iter().enumerate() {
                                let id = row.id.clone();
                                body = body.row(
                                    sapid::table_row(format!("account-{index}"))
                                        .class("record")
                                        .cell(
                                            sapid::table_cell("username")
                                                .text(row.username.clone()),
                                        )
                                        .cell(sapid::table_cell("name").text(
                                            row.display_name.clone().unwrap_or_else(|| "—".into()),
                                        ))
                                        .cell(sapid::table_cell("status").text(
                                            if row.deactivated_at.is_some() {
                                                "Deactivated"
                                            } else if row.locked_at.is_some() {
                                                "Locked"
                                            } else {
                                                "Active"
                                            },
                                        ))
                                        .cell(
                                            sapid::table_cell("created")
                                                .text(row.created_at.to_string()),
                                        )
                                        .cell(sapid::table_cell("actions").content(move || {
                                            let id = id.clone();
                                            action("Account details", move || {
                                                crate::ui::navigate(&account_path(&id))
                                            })
                                        })),
                                );
                            }
                            let mut table = sapid::table("pasion-accounts")
                                .unstyled()
                                .class("application-table")
                                .head(sapid::table_head().row(head))
                                .body(body);
                            table.build(scope)
                        })
                    },
                ),
                when(data.map(|rows| rows.is_empty()), || {
                    view()
                        .class("table-empty-compact")
                        .fill(label("No accounts found"))
                }),
            )),
            status(message),
            text().content(total),
            view().class("list-pagination").fill((
                action("Previous", move || {
                    if busy.get() {
                        return;
                    }
                    let mut h = history.get();
                    if let Some(previous) = h.pop() {
                        history.revise(|slot| *slot = h);
                        cursor.revise(|slot| *slot = previous);
                        back();
                    }
                })
                .enabled(history.map(|h| !h.is_empty())),
                action("Next", move || {
                    if busy.get() {
                        return;
                    }
                    if let Some(n) = next.get() {
                        let mut h = history.get();
                        h.push(cursor.get());
                        history.revise(|slot| *slot = h);
                        cursor.revise(|slot| *slot = Some(n));
                        forward();
                    }
                })
                .enabled(next.map(|n| n.is_some())),
            )),
            crate::ui::widgets::modal(
                "create-account",
                "Create account",
                create_open,
                view().name("Create account").class("operation-form").fill((
                    label("Create account"),
                    username_input,
                    password_input.purpose(Purpose::Password),
                    skip_input,
                    admin_check,
                    action("Create account", move || {
                        let name = username.get();
                        let pw = password.get();
                        let skip = skip.get();
                        let admin = make_admin.get();
                        submit(&s, writing, message, async move {
                            let name = required(name, "Username")?;
                            let u = pasion_create_user(&name).await?;
                            let mut errors = Vec::new();
                            if !pw.is_empty() {
                                if let Err(e) = pasion_set_password(&u.id, &pw, skip).await {
                                    errors.push(format!("password: {e}"));
                                }
                            }
                            if admin {
                                if let Err(e) =
                                    pasion_update_user(&u.id, serde_json::json!({"admin":true}))
                                        .await
                                {
                                    errors.push(format!("admin: {e}"));
                                }
                            }
                            let _ = password.try_revise(|slot| *slot = String::new());
                            let _ = username.try_revise(|slot| *slot = String::new());
                            // Keep the created account visible even when a follow-up failed.
                            let _ = data.try_revise(|rows| {
                                rows.insert(0, u.clone());
                                rows.truncate(25);
                            });
                            Ok(if errors.is_empty() {
                                crate::ui::navigate(&account_path(&u.id));
                                format!("Account {} created", u.username)
                            } else {
                                format!(
                                    "Account {} created; follow-up failed: {}",
                                    u.username,
                                    errors.join("; ")
                                )
                            })
                        });
                    })
                    .enabled(writing.map(|b| !b)),
                )),
            ),
        ))
}

fn audit(scope: &Scope) -> View {
    let data = scope.cage(Vec::<PasionAuditEntry>::new());
    let (filter, filter_input) = field(scope, "Filter by operation");
    let filtered = scope.cage(Vec::<PasionAuditEntry>::new());
    scope.owner().effect(move || {
        let filter = filter.get();
        let matching = data.with(|rows| {
            rows.iter()
                .filter(|row| row.operation.contains(&filter))
                .cloned()
                .collect()
        });
        filtered.revise(|rows| *rows = matching);
    });
    let (visible, pagination) = collection_window(scope, filtered, "records");
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let (limit, input) = field(scope, "Maximum records (1–1000)");
    limit.revise(|slot| *slot = "100".into());
    let s = scope.clone();
    let refresh: Rc<dyn Fn()> = Rc::new(move || {
        let n = limit.get().parse::<u64>();
        match n {
            Ok(n) if (1..=1000).contains(&n) => load(&s, busy, message, data, async move {
                Ok(pasion_get_audit_feed(n).await?.data)
            }),
            _ => message.revise(|slot| *slot = "Limit must be 1–1000".into()),
        }
    });
    refresh();
    page("Pasion audit log").class("page pasion-page").fill((
        input,
        filter_input,
        action("Refresh", move || refresh()).enabled(busy.map(|b| !b)),
        status(message),
        pagination,
        each(
            visible,
            |r| r.id.clone(),
            move |r, _| {
                view().class("record").fill((
                    text().content(r.map(|r| {
                        format!(
                            "{} · {} · actor {}",
                            r.created_at,
                            r.operation,
                            r.admin_user_id.as_deref().unwrap_or("system")
                        )
                    })),
                    text().content(r.map(|r| {
                        format!(
                            "{} {}",
                            r.resource_type.as_deref().unwrap_or(""),
                            r.resource_id.as_deref().unwrap_or("")
                        )
                    })),
                    text().content(r.map(|r| {
                        r.details
                            .as_ref()
                            .map(ToString::to_string)
                            .unwrap_or_default()
                    })),
                ))
            },
        ),
    ))
}

fn health(scope: &Scope) -> View {
    let data = scope.cage(Vec::<PasionConnectorHealth>::new());
    let (visible, pagination) = collection_window(scope, data, "records");
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let s = scope.clone();
    let refresh = Rc::new(move || load(&s, busy, message, data, pasion_get_connector_health()));
    refresh();
    page("Connector health").class("page pasion-page").fill((
        action("Refresh", move || refresh()).enabled(busy.map(|b| !b)),
        status(message),
        pagination,
        each(
            visible,
            |r| format!("{}:{}", r.provider, r.homeserver),
            move |r, _| {
                text().content(r.map(|r| {
                    format!(
                        "{} → {} · {}{}",
                        r.provider,
                        r.homeserver,
                        r.status,
                        r.error
                            .as_ref()
                            .map(|e| format!(" · {e}"))
                            .unwrap_or_default()
                    )
                }))
            },
        ),
    ))
}

fn channels(scope: &Scope) -> View {
    let data = scope.cage(Vec::<PasionNotificationChannel>::new());
    let (visible, pagination) = collection_window(scope, data, "records");
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let s = scope.clone();
    let refresh =
        Rc::new(move || load(&s, busy, message, data, pasion_get_notification_channels()));
    refresh();
    page("Notification channels").class("page pasion-page").fill((label("Channel configuration is managed by the server. This API exposes configuration status only."),action("Refresh",move ||refresh()).enabled(busy.map(|b|!b)),status(message),pagination,each(visible,|r|r.channel.clone(),move |r,_|text().content(r.map(|r|format!("{} · {}",r.channel,if r.configured{"configured"}else{"not configured"}))))))
}

fn templates(scope: &Scope) -> View {
    let data = scope.cage(Vec::<PasionNotificationTemplate>::new());
    let (visible, pagination) = collection_window(scope, data, "records");
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let s = scope.clone();
    let refresh =
        Rc::new(move || load(&s, busy, message, data, pasion_get_notification_templates()));
    refresh();
    let (key, key_input) = field(scope, "Template key");
    let (locale, locale_input) = field(scope, "Locale");
    locale.revise(|slot| *slot = "en".into());
    let (channel, channel_input) = field(scope, "Channel");
    channel.revise(|slot| *slot = "email".into());
    let (subject, subject_input) = field(scope, "Subject template");
    let (body, body_input) = field(scope, "Body template");
    let writing = scope.cage(false);
    let s = scope.clone();
    page("Notification templates").class("page pasion-page").fill((action("Refresh",move ||refresh()),status(message),pagination,each(visible,|r|r.key.clone(),move |r,_|{let name=r.get().key;view().fill((text().content(r.map(|r|format!("{} · {}",r.key,r.description))),action("Select template",move||key.revise(|slot| *slot = name.clone()))))}),view().name("Publish template").class("panel template-editor").fill((label("Publish template"),key_input,locale_input,channel_input,subject_input,body_input.multiline(true),action("Publish template",move||{
        let k=key.get();let l=locale.get();let c=channel.get();let sub=subject.get();let b=body.get();if !confirm("Publish this notification template?"){return;}
        submit(&s,writing,message,async move{let k=required(k,"Template key")?;let l=required(l,"Locale")?;let c=required(c,"Channel")?;required(b.clone(),"Body template")?;let mut payload=serde_json::json!({"template_key":k,"locale":l,"channel":c,"body_template":b});if !sub.is_empty(){payload["subject_template"]=sub.into();}pasion_publish_template(payload).await?;Ok("Template published".into())});
    }).enabled(writing.map(|b|!b))))))
}

fn oauth(scope: &Scope) -> View {
    let data = scope.cage(Vec::<PasionOAuth2Session>::new());
    let (visible, pagination) = collection_window(scope, data, "records");
    let busy = scope.cage(false);
    let writing = scope.cage(false);
    let message = scope.cage(String::new());
    let (user, input) = field(scope, "Filter by user ID");
    let s = scope.clone();
    let refresh = Rc::new(move || {
        let uid = optional(user.get());
        load(&s, busy, message, data, async move {
            pasion_get_oauth2_sessions(uid.as_deref()).await
        })
    });
    refresh();
    let s = scope.clone();
    page("OAuth2 sessions").class("page pasion-page").fill((
        input,
        action("Apply filter / refresh", move || refresh()).enabled(busy.map(|b| !b)),
        status(message),
        pagination,
        each(
            visible,
            |r| r.id.clone(),
            move |r, _| {
                let s = s.clone();
                let ds = s.clone();
                let id = r.get().id;
                let did = id.clone();
                view().class("record").fill((
                    text().content(r.map(|r| {
                        format!(
                            "{} · user {} · client {} · {}",
                            r.id,
                            r.user_id.as_deref().unwrap_or("—"),
                            r.client_id,
                            if r.finished_at.is_some() {
                                "finished"
                            } else {
                                "active"
                            }
                        )
                    })),
                    text().content(r.map(|r| {
                        format!(
                            "Scope {} · created {} · last active {} · IP {}",
                            r.scope,
                            r.created_at,
                            r.last_active_at.as_deref().unwrap_or("—"),
                            r.last_active_ip.as_deref().unwrap_or("—")
                        )
                    })),
                    action("Load details", move || {
                        let id = did.clone();
                        submit(&ds, writing, message, async move {
                            let session = pasion_get_oauth2_session(&id).await?;
                            let _ = data.try_revise(|rows| {
                                if let Some(row) = rows.iter_mut().find(|r| r.id == id) {
                                    *row = session;
                                }
                            });
                            Ok("Session details loaded".into())
                        });
                    }),
                    action("End session", move || {
                        if !confirm("End this OAuth2 session and revoke its tokens?") {
                            return;
                        }
                        let id = id.clone();
                        submit(&s, writing, message, async move {
                            pasion_finish_oauth2_session(&id).await?;
                            let uid = optional(user.get());
                            let rows = pasion_get_oauth2_sessions(uid.as_deref()).await?;
                            let _ = data.try_revise(|slot| *slot = rows);
                            Ok("Session ended".into())
                        });
                    })
                    .enabled(r.map(|r| r.finished_at.is_none())),
                ))
            },
        ),
    ))
}

fn personal(scope: &Scope) -> View {
    let data = scope.cage(Vec::<PasionPersonalSession>::new());
    let (visible, pagination) = collection_window(scope, data, "records");
    let busy = scope.cage(false);
    let writing = scope.cage(false);
    let message = scope.cage(String::new());
    let token = scope.cage(String::new());
    let s = scope.clone();
    let refresh = Rc::new(move || load(&s, busy, message, data, pasion_get_personal_sessions()));
    refresh();
    let (name, name_input) = field(scope, "Token name");
    let (user, user_input) = field(scope, "Actor user ID");
    let (scopes, scope_input) = field(scope, "Scopes");
    let s = scope.clone();
    let rows_scope = s.clone();
    let copy_scope = s.clone();
    page("Personal sessions / API tokens")
        .class("page pasion-page")
        .fill((
            action("Refresh", move || refresh()),
            status(message),
            view()
                .name("One-time token")
                .class(token.map(|value| {
                    if value.is_empty() {
                        "bulk-hidden".into()
                    } else {
                        "panel token-panel".into()
                    }
                }))
                .fill((
                    label("Copy this token now. It is shown only after creation or regeneration."),
                    text().content(token).selectable(true),
                    action("Copy token", move || {
                        copy(&copy_scope, token.get(), message)
                    }),
                    action("Dismiss token", move || {
                        token.revise(|slot| *slot = String::new())
                    }),
                )),
            view()
                .name("Create personal session")
                .class("panel token-create-panel")
                .fill((
                    name_input,
                    user_input,
                    scope_input,
                    action("Create token", move || {
                        let n = name.get();
                        let u = user.get();
                        let sc = scopes.get();
                        submit(&s, writing, message, async move {
                            let session =
                                pasion_create_personal_session(CreatePersonalSessionRequest {
                                    human_name: required(n, "Token name")?,
                                    actor_user_id: required(u, "Actor user ID")?,
                                    scope: required(sc, "Scopes")?,
                                    expires_in: None,
                                })
                                .await?;
                            let _ =
                                token.try_revise(|slot| *slot = session.token.unwrap_or_default());
                            let loaded = pasion_get_personal_sessions().await?;
                            let _ = data.try_revise(|slot| *slot = loaded);
                            Ok("Personal session created".into())
                        });
                    })
                    .enabled(writing.map(|b| !b)),
                )),
            pagination,
            each(
                visible,
                |r| r.id.clone(),
                move |r, _| {
                    let id = r.get().id;
                    let reg = id.clone();
                    let det = id.clone();
                    let s = rows_scope.clone();
                    let rs = s.clone();
                    let ds = s.clone();
                    view().class("record").fill((
                        text().content(r.map(|r| {
                            format!(
                                "{} · {} · user {} · {}",
                                r.id,
                                r.name.as_deref().unwrap_or("unnamed"),
                                r.user_id.as_deref().unwrap_or("—"),
                                if r.revoked_at.is_some() {
                                    "revoked"
                                } else {
                                    "active"
                                }
                            )
                        })),
                        text().content(r.map(|r| {
                            format!(
                                "Scope {} · created {} · last active {}",
                                r.scope,
                                r.created_at,
                                r.last_active_at.as_deref().unwrap_or("—")
                            )
                        })),
                        action("Details", move || {
                            let id = det.clone();
                            submit(&ds, writing, message, async move {
                                let mut session = pasion_get_personal_session(&id).await?;
                                session.token = None;
                                let _ = data.try_revise(|rows| {
                                    if let Some(row) = rows.iter_mut().find(|r| r.id == id) {
                                        *row = session;
                                    }
                                });
                                Ok("Details loaded".into())
                            });
                        }),
                        action("Regenerate token", move || {
                            if !confirm(
                                "Regenerate this token? The previous token will stop working.",
                            ) {
                                return;
                            }
                            let id = reg.clone();
                            submit(&rs, writing, message, async move {
                                let session = pasion_regenerate_personal_session(&id).await?;
                                let _ = token
                                    .try_revise(|slot| *slot = session.token.unwrap_or_default());
                                let loaded = pasion_get_personal_sessions().await?;
                                let _ = data.try_revise(|slot| *slot = loaded);
                                Ok("Token regenerated".into())
                            });
                        })
                        .enabled(r.map(|r| r.revoked_at.is_none())),
                        action("Revoke", move || {
                            if !confirm("Permanently revoke this personal session?") {
                                return;
                            }
                            let id = id.clone();
                            submit(&s, writing, message, async move {
                                pasion_revoke_personal_session(&id).await?;
                                let _ = token.try_revise(|slot| *slot = String::new());
                                let loaded = pasion_get_personal_sessions().await?;
                                let _ = data.try_revise(|slot| *slot = loaded);
                                Ok("Session revoked".into())
                            });
                        })
                        .enabled(r.map(|r| r.revoked_at.is_none())),
                    ))
                },
            ),
        ))
}

fn copy(scope: &Scope, value: String, message: Cage<String>) {
    #[cfg(target_arch = "wasm32")]
    spawn(scope, async move {
        let result = if let Some(window) = web_sys::window() {
            wasm_bindgen_futures::JsFuture::from(window.navigator().clipboard().write_text(&value))
                .await
                .map(|_| ())
                .map_err(|_| "Clipboard permission denied")
        } else {
            Err("Browser unavailable")
        };
        let _ = message.try_revise(|slot| {
            *slot = match result {
                Ok(()) => "Copied".into(),
                Err(e) => e.into(),
            }
        });
    });
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (scope, value);
        message.revise(|slot| *slot = "Clipboard requires the browser".into());
    }
}

fn links(scope: &Scope) -> View {
    let data = scope.cage(Vec::<PasionUpstreamLink>::new());
    let (visible, pagination) = collection_window(scope, data, "records");
    let busy = scope.cage(false);
    let writing = scope.cage(false);
    let message = scope.cage(String::new());
    let (user, user_input) = field(scope, "Filter by user ID");
    let (provider, provider_input) = field(scope, "Filter by provider ID");
    let s = scope.clone();
    let refresh = Rc::new(move || {
        let u = optional(user.get());
        let p = optional(provider.get());
        load(&s, busy, message, data, async move {
            pasion_get_upstream_links(u.as_deref(), p.as_deref()).await
        })
    });
    refresh();
    let (cu, cu_input) = field(scope, "Account user ID");
    let (cp, cp_input) = field(scope, "Provider ID");
    let (subject, subject_input) = field(scope, "Provider subject");
    let (name, name_input) = field(scope, "Human account name (optional)");
    let s = scope.clone();
    let rows_scope = s.clone();
    page("Upstream account links").class("page pasion-page").fill((view().fill((user_input,provider_input,action("Apply filters / refresh",move||refresh()).enabled(busy.map(|b|!b)))),status(message),view().name("Create upstream link").class("panel link-create-panel").fill((cu_input,cp_input,subject_input,name_input,action("Create link",move||{let u=cu.get();let p=cp.get();let sub=subject.get();let n=optional(name.get());submit(&s,writing,message,async move{let mut body=serde_json::json!({"user_id":required(u,"User ID")?,"provider_id":required(p,"Provider ID")?,"subject":required(sub,"Subject")?});if let Some(n)=n{body["human_account_name"]=n.into();}let link=pasion_create_upstream_link(body).await?;let _=data.try_revise(|rows|rows.insert(0,link));Ok("Link created".into())});}).enabled(writing.map(|b|!b)))),pagination,each(visible,|r|r.id.clone(),move|r,_|{let id=r.get().id;let s=rows_scope.clone();view().class("record").fill((text().content(r.map(|r|format!("{} · user {} · provider {} · subject {} · {} · {}",r.id,r.user_id.as_deref().unwrap_or("—"),r.provider_id,r.subject,r.human_account_name.as_deref().unwrap_or("—"),r.created_at))),action("Delete link",move||{if !confirm("Delete this upstream identity link?"){return;}let id=id.clone();submit(&s,writing,message,async move{pasion_delete_upstream_link(&id).await?;let _=data.try_revise(|rows|rows.retain(|r|r.id!=id));Ok("Link deleted".into())});})))})))
}
