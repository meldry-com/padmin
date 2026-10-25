//! Dedicated operational workflows backed by the existing typed Matrix/Palpo APIs.
use crate::ui::widgets::*;
use crate::utils::{date, error::HttpError, storage};
use crate::{api, router::Route, types::*};
use glory::prelude::*;
use glory::ui::{Scope, View};
use std::rc::Rc;
use std::result::Result;

fn required(value: &str, name: &str) -> Result<String, HttpError> {
    if value.trim().is_empty() {
        Err(HttpError::message(format!("{name} is required")))
    } else {
        Ok(value.trim().to_owned())
    }
}
fn number(value: &str, name: &str) -> Result<u64, HttpError> {
    value
        .trim()
        .parse()
        .map_err(|_| HttpError::message(format!("{name} must be a non-negative integer")))
}
fn optional_number(value: &str, name: &str) -> Result<Option<u64>, HttpError> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        number(value, name).map(Some)
    }
}
fn toggle(scope: &Scope, caption: &str, initial: bool) -> (Cage<bool>, glory::ui::Check) {
    let value = scope.cage(initial);
    (
        value,
        check()
            .name(localized(scope, caption.to_owned()))
            .checked(value)
            .on(checked_changed, move |event| {
                if let glory::render::Event::CheckedChanged(checked) = event {
                    value.revise(|stored_value| *stored_value = checked);
                }
            }),
    )
}
#[derive(Clone)]
struct Row {
    id: String,
    summary: String,
    target: String,
}
fn rows(data: Cage<Vec<Row>>) -> impl glory::ui::Widget {
    each(
        data,
        |r| r.id.clone(),
        move |row, _| {
            view()
                .class("record")
                .fill(text().content(row.map(|r| r.summary.clone())))
                .fill(action("Open details", move || {
                    let target = row.get().target;
                    crate::ui::navigate(&target);
                }))
        },
    )
}

pub fn render(scope: &Scope, route: &Route) -> View {
    match route.path.as_str() {
        "/media" => media(scope),
        "/reports" => listing(scope, "Event reports", 1, None),
        "/reports/:report_id" => report(scope, route.params.first().cloned().unwrap_or_default()),
        "/destinations" => listing(scope, "Federation destinations", 2, None),
        "/destinations/:destination_id" => {
            destination(scope, route.params.first().cloned().unwrap_or_default())
        }
        "/registration-tokens" => tokens(scope),
        "/auth-status" => diagnostics(scope),
        "/server-status" => server_status(scope),
        "/server-actions" => server_actions(scope),
        "/server-notices" => notices(scope),
        "/server-notifications" => notifications(scope),
        "/settings/notifications" => preferences(scope),
        _ => page("Operations").fill(label("Unknown operation")),
    }
}

fn listing(scope: &Scope, title: &str, kind: u8, refresh: Option<Cage<u64>>) -> View {
    let state_key = format!("operations-list-{kind}");
    let saved = storage::get_item(&state_key)
        .and_then(|v| serde_json::from_str::<serde_json::Value>(&v).ok())
        .unwrap_or_default();
    let data = scope.cage(Vec::<Row>::new());
    let message = scope.cage(String::new());
    let busy = scope.cage(false);
    let current = scope.cage(saved["page"].as_u64().unwrap_or(1).clamp(1, u64::MAX / 25));
    let total = scope.cage(0u64);
    let descending = scope.cage(saved["descending"].as_bool().unwrap_or(true));
    let (search, search_input) = field(scope, "Search user ID or display name");
    let (sort, sort_input) = field(scope, "Sort field");
    search.revise(|value| *value = saved["search"].as_str().unwrap_or_default().to_owned());
    sort.revise(|stored_value| {
        *stored_value = saved["sort"]
            .as_str()
            .unwrap_or(match kind {
                1 => "received_ts",
                2 => "destination",
                _ => "media_length",
            })
            .into()
    });
    let s = scope.clone();
    let context = crate::ui::context::get(scope);
    let reload: Rc<dyn Fn()> = Rc::new(move || {
        let context = context.clone();
        let state_key = state_key.clone();
        let index = current.get();
        let query = search.get();
        let order_by = sort.get();
        let direction = if descending.get() { "DESC" } else { "ASC" };
        submit(&s, busy, message, async move {
            let valid = match kind {
                1 => ["received_ts"].contains(&order_by.as_str()),
                2 => [
                    "destination",
                    "retry_last_ts",
                    "retry_interval",
                    "failure_ts",
                    "last_successful_stream_ordering",
                ]
                .contains(&order_by.as_str()),
                _ => ["user_id", "displayname", "media_count", "media_length"]
                    .contains(&order_by.as_str()),
            };
            if !valid {
                return Err(HttpError::message("Unsupported sort field"));
            }
            storage::set_item(&state_key, &serde_json::json!({"page":index,"search":query,"sort":order_by,"descending":direction == "DESC"}).to_string());
            let (items, count) = match kind {
                1 => {
                    let result = api::reports::get_reports(index, 25, &order_by, direction).await?;
                    (
                        result
                            .data
                            .into_iter()
                            .map(|r| Row {
                                id: r.id.to_string(),
                                summary: format!(
                                    "#{} · {} · Room {} · Reporter {} · {} · {}",
                                    r.id,
                                    r.status,
                                    r.room_id,
                                    r.user_id,
                                    date::format_timestamp(r.received_ts),
                                    r.reason.unwrap_or_default()
                                ),
                                target: format!("/reports/{}", r.id),
                            })
                            .collect(),
                        result.total,
                    )
                }
                2 => {
                    let result =
                        api::destinations::get_destinations(index, 25, &order_by, direction)
                            .await?;
                    (
                        result
                            .data
                            .into_iter()
                            .map(|r| Row {
                                id: r.id.clone(),
                                summary: format!(
                                    "{} · Retry interval {} ms · Last attempt {} · Failure {}",
                                    r.id,
                                    r.destination.retry_interval,
                                    date::format_timestamp(r.destination.retry_last_ts),
                                    r.destination
                                        .failure_ts
                                        .map(date::format_timestamp)
                                        .unwrap_or_default()
                                ),
                                target: format!("/destinations/{}", urlencoding::encode(&r.id)),
                            })
                            .collect(),
                        result.total,
                    )
                }
                _ => {
                    let result = api::media::get_user_media_statistics(
                        index, 25, &order_by, direction, &query,
                    )
                    .await?;
                    (
                        result
                            .data
                            .into_iter()
                            .map(|r| Row {
                                id: r.id.clone(),
                                summary: format!(
                                    "{} · {} · {} files · {}",
                                    r.id,
                                    r.statistic.displayname.unwrap_or_default(),
                                    r.statistic.media_count,
                                    date::format_bytes(r.statistic.media_length)
                                ),
                                target: format!("/users/{}", urlencoding::encode(&r.id)),
                            })
                            .collect(),
                        result.total,
                    )
                }
            };
            if kind == 1 && index == 1 && count > 0 {
                if let Some(context) = &context {
                    context.notify(
                        "report_filed",
                        format!("{count} event report(s) pending review"),
                        false,
                    );
                }
            }
            data.revise(|stored_value| *stored_value = items);
            total.revise(|stored_value| *stored_value = count);
            Ok(if count == 0 {
                "No matching records".into()
            } else {
                format!("{count} records · Page {index}")
            })
        });
    });
    reload();
    if let Some(refresh) = refresh {
        let reload = reload.clone();
        let consumed = std::cell::Cell::new(0);
        scope.owner().effect(move || {
            let requested = refresh.get();
            let loading = busy.get();
            if requested != consumed.get() && !loading {
                consumed.set(requested);
                glory::core::untracked(|| reload());
            }
        });
    }
    let mut root = page(title);
    if kind == 0 {
        root = root
            .fill(label("Search user ID or display name"))
            .fill(search_input);
    }
    let search_reload = reload.clone();
    let order_reload = reload.clone();
    let previous = reload.clone();
    let next = reload.clone();
    root.fill(label("Sort field")).fill(sort_input)
        .fill(label(match kind {1=>"Sort: received_ts",2=>"Sort: destination, retry_last_ts, retry_interval, failure_ts, last_successful_stream_ordering",_=>"Sort: user_id, displayname, media_count, media_length"}))
        .fill(action("Apply filters", move || { if !busy.get() {current.revise(|stored_value| *stored_value = 1); search_reload();} }))
        .fill(action("Reverse sort", move || { if !busy.get() { descending.revise(|stored_value| *stored_value = !*stored_value); current.revise(|stored_value| *stored_value = 1);order_reload();} }))
        .fill(action("Refresh / Retry", move || reload()))
        .fill(status(message)).fill(rows(data))
        .fill(action("Previous page", move || { if !busy.get() && current.get()>1 {current.revise(|stored_value| *stored_value -= 1);previous();} }))
        .fill(action("Next page", move || { if !busy.get() && current.get().saturating_mul(25)<total.get() {current.revise(|stored_value| *stored_value += 1);next();} }))
}

fn media(scope: &Scope) -> View {
    let refresh = scope.cage(0u64);
    let mut root = listing(scope, "Media", 0, Some(refresh));
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let (before, before_input) = field(scope, "Before UTC timestamp (milliseconds)");
    let (size, size_input) = field(scope, "Minimum size in bytes");
    size.revise(|stored_value| *stored_value = "0".into());
    let (keep, keep_input) = toggle(scope, "Keep profile media", false);
    let s = scope.clone();
    let s2 = scope.clone();
    root = root
        .fill(label("Bulk media cleanup"))
        .fill(label("This server bulk cleanup includes profile media. Preserving profile media is unavailable; selecting it blocks submission."))
        .fill(label("Before UTC timestamp (milliseconds)"))
        .fill(before_input)
        .fill(label("Minimum size in bytes"))
        .fill(size_input)
        .fill(keep_input)
        .fill(action("Delete local media", move || {
            if !confirm("Permanently delete matching local media?") {
                return;
            }
            let b = before.get();
            let z = size.get();
            let k = keep.get();
            submit(&s, busy, message, async move {
                let ts = number(&b, "Before timestamp")?;
                if ts == 0 {
                    return Err(HttpError::message("Before timestamp must be positive"));
                }
                let result =
                    api::media::delete_local_media(ts, number(&z, "Minimum size")?, k).await?;
                refresh.revise(|v| *v += 1);
                Ok(format!("Deleted {} media files", result.total))
            });
        }))
        .fill(action("Purge remote media cache", move || {
            if !confirm("Purge cached remote media before this timestamp?") {
                return;
            }
            let b = before.get();
            submit(&s2, busy, message, async move {
                let ts = number(&b, "Before timestamp")?;
                if ts == 0 {
                    return Err(HttpError::message("Before timestamp must be positive"));
                }
                let result = api::media::purge_remote_media(ts).await?;
                refresh.revise(|v| *v += 1);
                Ok(format!("Purged {} files", result.total))
            });
        }));
    let (id, id_input) = field(scope, "Media ID");
    root = root
        .fill(label("Manage individual media"))
        .fill(label("Media ID"))
        .fill(id_input);
    for (caption, kind) in [
        ("Delete media", 0),
        ("Quarantine media", 1),
        ("Protect media", 2),
    ] {
        let s = scope.clone();
        root = root.fill(action(caption, move || {
            if !confirm(&format!("{caption}?")) {
                return;
            }
            let value = id.get();
            submit(&s, busy, message, async move {
                let id = required(&value, "Media ID")?;
                if id.contains('/') || id.contains('?') || id.contains('#') {
                    return Err(HttpError::message(
                        "Enter a media ID, without URL separators",
                    ));
                }
                match kind {
                    0 => api::media::delete_media(&id).await?,
                    1 => api::media::quarantine_media(&id).await?,
                    _ => api::media::protect_media(&id).await?,
                };
                refresh.revise(|v| *v += 1);
                Ok(format!("{caption} completed"))
            });
        }));
    }
    root.fill(status(message))
}

fn report_detail(r: EventReport) -> String {
    format!(
        "Report #{}\nStatus: {}\nRoom: {}\nEvent: {}\nReporter: {}\nSender: {}\nReceived: {}\nReason: {}\nScore: {:?}\nEvent: {}",
        r.id,
        r.status,
        r.room_id,
        r.event_id,
        r.user_id,
        r.sender.unwrap_or_default(),
        date::format_timestamp(r.received_ts),
        r.reason.unwrap_or_default(),
        r.score,
        r.event_json.map(|v| v.to_string()).unwrap_or_default()
    )
}

fn report(scope: &Scope, id: String) -> View {
    let record = scope.cage(None::<EventReport>);
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let detail = scope.cage(String::new());
    let s = scope.clone();
    let read_id = id.clone();
    let load: Rc<dyn Fn()> = Rc::new(move || {
        let id = read_id.clone();
        submit(&s, busy, message, async move {
            let r = api::reports::get_report(number(&id, "Report ID")?).await?;
            record.revise(|value| *value = Some(r.clone()));
            detail.revise(|stored_value| *stored_value = report_detail(r));
            Ok("Report loaded".into())
        });
    });
    load();
    let mut root = page("Report details")
        .fill(status(detail))
        .fill(action("Refresh / Retry", move || load()));
    for value in ["new", "in_review", "resolved"] {
        let id = id.clone();
        let s = scope.clone();
        root = root.fill(action(&format!("Mark {value}"), move || {
            let id = id.clone();
            submit(&s, busy, message, async move {
                let r =
                    api::reports::update_report_status(number(&id, "Report ID")?, value).await?;
                let status = r.status.clone();
                record.revise(|value| *value = Some(r.clone()));
                detail.revise(|stored_value| *stored_value = report_detail(r));
                Ok(format!("Status changed to {status}"))
            });
        }));
    }
    for (caption, kind) in [
        ("Redact reported event", 0),
        ("Ban reported sender", 1),
        ("Block reported room", 2),
    ] {
        let owner = scope.clone();
        root = root.fill(
            action(caption, move || {
                if busy.get() {
                    return;
                }
                let Some(report) = record.get() else {
                    return;
                };
                if !confirm(&format!(
                    "{caption}? This changes the reported room or event."
                )) {
                    return;
                }
                submit(&owner, busy, message, async move {
                    let room = required(&report.room_id, "Reported room")?;
                    match kind {
                        0 => {
                            let event = required(&report.event_id, "Reported event")?;
                            let transaction = format!(
                                "report-{}-{}",
                                report.id,
                                chrono::Utc::now().timestamp_millis()
                            );
                            api::rooms::redact_event(
                                &room,
                                &event,
                                &transaction,
                                "Removed by admin",
                            )
                            .await?;
                        }
                        1 => {
                            let sender = required(
                                report.sender.as_deref().unwrap_or_default(),
                                "Reported sender",
                            )?;
                            api::rooms::ban_user(
                                &room,
                                &sender,
                                "Banned by admin following report",
                            )
                            .await?;
                        }
                        _ => api::rooms::block_room(&room, true).await?,
                    }
                    Ok(format!("{caption} completed"))
                });
            })
            .enabled(record.map(move |r| {
                r.as_ref().is_some_and(|r| {
                    !r.room_id.is_empty()
                        && match kind {
                            0 => !r.event_id.is_empty(),
                            1 => r.sender.as_ref().is_some_and(|v| !v.is_empty()),
                            _ => true,
                        }
                })
            })),
        );
    }
    for (caption, kind) in [
        ("Open reporter", 0),
        ("Open reported room", 1),
        ("Open reported sender", 2),
    ] {
        let label = localized(scope, caption.to_owned());
        root = root.fill(when(
            record.map(move |r| {
                r.as_ref().is_some_and(|r| match kind {
                    0 => !r.user_id.is_empty(),
                    1 => !r.room_id.is_empty(),
                    _ => r.sender.as_ref().is_some_and(|v| !v.is_empty()),
                })
            }),
            move || {
                link().label(label.clone()).target(record.map(move |r| {
                    let Some(report) = r else {
                        return "/".into();
                    };
                    let (prefix, value) = match kind {
                        0 => ("/users/", report.user_id.as_str()),
                        1 => ("/rooms/", report.room_id.as_str()),
                        _ => ("/users/", report.sender.as_deref().unwrap_or_default()),
                    };
                    crate::utils::config::app_path(&format!(
                        "{prefix}{}",
                        urlencoding::encode(value)
                    ))
                }))
            },
        ));
    }
    let s = scope.clone();
    root.fill(action("Delete report", move || {
        if !confirm("Permanently delete this report?") {
            return;
        }
        let id = id.clone();
        submit(&s, busy, message, async move {
            api::reports::delete_report(number(&id, "Report ID")?).await?;
            crate::ui::navigate("/reports");
            Ok("Report deleted".into())
        });
    }))
    .fill(action("Back to reports", || {
        crate::ui::navigate("/reports")
    }))
    .fill(status(message))
}
fn destination(scope: &Scope, id: String) -> View {
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let detail = scope.cage(String::new());
    let s = scope.clone();
    let target = id.clone();
    let load: Rc<dyn Fn()> = Rc::new(move || {
        let id = target.clone();
        submit(&s, busy, message, async move {
            let r = api::destinations::get_destination(&id).await?.destination;
            detail.revise(|stored_value| *stored_value = format!("{}\nRetry interval: {} ms\nLast retry: {}\nFailure: {}\nLast successful stream ordering: {:?}",r.destination,r.retry_interval,date::format_timestamp(r.retry_last_ts),r.failure_ts.map(date::format_timestamp).unwrap_or_default(),r.last_successful_stream_ordering));
            Ok("Destination loaded".into())
        });
    });
    load();
    let s = scope.clone();
    page("Federation destination")
        .fill(status(detail))
        .fill(action("Refresh / Retry", move || load()))
        .fill(action("Reset connection", move || {
            if !confirm("Reset this federation connection?") {
                return;
            }
            let id = id.clone();
            submit(&s, busy, message, async move {
                api::destinations::reset_destination_connection(&id).await?;
                let r = api::destinations::get_destination(&id).await?.destination;
                detail.revise(|v| *v = format!("{}\nRetry interval: {} ms\nLast retry: {}\nFailure: {}\nLast successful stream ordering: {:?}",r.destination,r.retry_interval,date::format_timestamp(r.retry_last_ts),r.failure_ts.map(date::format_timestamp).unwrap_or_default(),r.last_successful_stream_ordering));
                Ok("Connection reset".into())
            });
        }))
        .fill(action("Back to destinations", || {
            crate::ui::navigate("/destinations")
        }))
        .fill(status(message))
}

fn tokens(scope: &Scope) -> View {
    let data = scope.cage(Vec::<RegistrationTokenRecord>::new());
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let (query, query_input) = field(scope, "Search registration tokens");
    let mode = scope.cage(0u8);
    let token_page = scope.cage(0usize);
    scope.owner().effect(move || {
        let _ = query.get();
        let _ = mode.get();
        glory::core::untracked(|| token_page.revise(|v| *v = 0));
    });
    let s = scope.clone();
    let reload: Rc<dyn Fn()> = Rc::new(move || {
        submit(&s, busy, message, async move {
            let items = api::registration_tokens::get_registration_tokens().await?;
            let n = items.len();
            data.revise(|stored_value| *stored_value = items);
            Ok(if n == 0 {
                "No registration tokens".into()
            } else {
                format!("{n} registration tokens")
            })
        })
    });
    reload();
    let filtered = data.map(move |items| {
        let q = query.get();
        let mode = mode.get();
        let now = chrono::Utc::now().timestamp_millis().max(0) as u64;
        let page = token_page.get();
        items
            .iter()
            .filter(|r| {
                let expired = r.token.expiry_time.is_some_and(|v| v > 0 && v <= now)
                    || r.token.uses_allowed.is_some_and(|limit| {
                        r.token.pending.saturating_add(r.token.completed) >= limit
                    });
                r.id.contains(&q)
                    && (mode == 0 || (mode == 1 && !expired) || (mode == 2 && expired))
            })
            .skip(page.saturating_mul(25))
            .take(26)
            .cloned()
            .collect::<Vec<_>>()
    });
    let has_more = scope.bond(move || filtered.with(|items| items.len() > 25));
    let visible = scope
        .bond(move || filtered.with(|items| items.iter().take(25).cloned().collect::<Vec<_>>()));
    let s = scope.clone();

    let table = each(
        visible,
        |r| r.id.clone(),
        move |row, _| {
            let s1 = s.clone();
            let s2 = s.clone();

            view()
                .class("record")
                .fill(text().content(row.map(|r| {
                    format!(
                        "{} · Uses allowed {} · Pending {} · Completed {} · Expiry {}",
                        r.id,
                        r.token
                            .uses_allowed
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| "unlimited".into()),
                        r.token.pending,
                        r.token.completed,
                        r.token
                            .expiry_time
                            .map(date::format_timestamp)
                            .unwrap_or_else(|| "never".into())
                    )
                })))
                .fill(action("Copy token", move || {
                    let token = row.get().id;
                    submit(&s1, busy, message, async move {
                        copy(&token).await?;
                        Ok("Token copied".into())
                    });
                }))
                .fill(action("Delete token", move || {
                    if !confirm("Delete this registration token?") {
                        return;
                    }
                    let id = row.get().id;

                    submit(&s2, busy, message, async move {
                        api::registration_tokens::delete_registration_token(&id).await?;
                        data.revise(|items| items.retain(|v| v.id != id));

                        Ok("Token deleted".into())
                    });
                }))
        },
    );
    let (token, token_input) = field(scope, "Token (blank to generate)");
    let (uses, uses_input) = field(scope, "Allowed uses (blank for unlimited)");
    let (expiry, expiry_input) = field(scope, "Expiry date UTC (YYYY-MM-DD; blank for none)");
    let (length, length_input) = field(scope, "Generated token length (1–64)");
    length.revise(|stored_value| *stored_value = "16".into());
    let s = scope.clone();
    page("Registration tokens").fill(label("Search registration tokens")).fill(query_input).fill(action("All tokens",move ||mode.revise(|stored_value| *stored_value = 0))).fill(action("Active tokens",move ||mode.revise(|stored_value| *stored_value = 1))).fill(action("Expired tokens",move ||mode.revise(|stored_value| *stored_value = 2))).fill(action("Refresh / Retry",move ||reload())).fill(status(message)).fill(table)
        .fill(action("Previous token page",move ||token_page.revise(|v|*v=v.saturating_sub(1))))
        .fill(action("Next token page",move ||{if has_more.get(){token_page.revise(|v|*v=v.saturating_add(1));}}))
        .fill(label("Create registration token")).fill(label("Token (blank to generate)")).fill(token_input).fill(label("Allowed uses (blank for unlimited)")).fill(uses_input).fill(label("Expiry date UTC (YYYY-MM-DD; blank for none)")).fill(expiry_input).fill(label("Generated token length (1–64)")).fill(length_input)
        .fill(action("Create token",move ||{let token=token.get();let uses=uses.get();let expiry=expiry.get();let length=length.get();submit(&s,busy,message,async move{
            let uses=optional_number(&uses,"Allowed uses")?;
            let expiry=if expiry.trim().is_empty(){None}else{let date=chrono::NaiveDate::parse_from_str(&expiry,"%Y-%m-%d").map_err(|_|HttpError::message("Expiry must be YYYY-MM-DD"))?;let ts=date.and_hms_opt(23,59,59).unwrap().and_utc().timestamp_millis();if ts<=chrono::Utc::now().timestamp_millis(){return Err(HttpError::message("Expiry must be in the future"));}Some(ts as u64)};
            let length=if token.is_empty(){let length=number(&length,"Length")?;if !(1..=64).contains(&length){return Err(HttpError::message("Length must be 1–64"));}Some(length)}else{if !token.chars().all(|c|c.is_ascii_alphanumeric()||"._~-".contains(c)){return Err(HttpError::message("Token may contain only letters, digits, dot, underscore, tilde and dash"));}None};
            let result=api::registration_tokens::create_registration_token(if token.is_empty(){None}else{Some(token.as_str())},uses,expiry,length).await?;data.revise(|items|items.push(result));Ok("Registration token created".into())
        });}))
}

fn diagnostics(scope: &Scope) -> View {
    let data = scope.cage(Vec::<api::diagnostics::Diagnostic>::new());
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let s = scope.clone();
    let reload: Rc<dyn Fn()> = Rc::new(move || {
        submit(&s, busy, message, async move {
            let discovered = api::diagnostics::discover().await;
            data.revise(|stored_value| *stored_value = discovered);
            Ok("Discovery completed; individual endpoint outcomes are shown below".into())
        })
    });
    reload();
    let s = scope.clone();
    page("Authentication diagnostics")
        .fill(action("Refresh discovery", move || reload()))
        .fill(action("Probe login, registration and consent", move || {
            submit(&s, busy, message, async move {
                let checks = api::diagnostics::probe().await;
                data.revise(|items| {
                    items.retain(|v| {
                        !matches!(
                            v.name.as_str(),
                            "Matrix login"
                                | "Well-known discovery"
                                | "Consent endpoint"
                                | "Registration"
                        )
                    });
                    items.extend(checks);
                });
                Ok("Capability probes completed".into())
            })
        }))
        .fill(status(message))
        .fill(each(
            data,
            |r| r.name.clone(),
            move |r, _| {
                view().class("record").fill(
                    text().content(r.map(|r| format!("{}\n{}\n{}", r.name, r.endpoint, r.outcome))),
                )
            },
        ))
}
fn sidecar(scope: &Scope) -> (Cage<String>, LocalInput) {
    let (url, input) = field(scope, "Palpo admin sidecar URL");
    url.revise(|stored_value| {
        *stored_value = storage::get_item("palpo_admin_url").unwrap_or_default()
    });
    (url, input)
}
fn configured_url(value: String) -> Result<String, HttpError> {
    let value = required(&value, "Sidecar URL")?;
    if !(value.starts_with('/') || value.starts_with("http://") || value.starts_with("https://")) {
        return Err(HttpError::message(
            "Sidecar URL must be HTTP(S) or a same-origin path",
        ));
    }
    Ok(value.trim_end_matches('/').into())
}
fn server_status(scope: &Scope) -> View {
    let (url, url_input) = sidecar(scope);
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let detail = scope.cage(String::new());
    let help = scope.cage(Vec::<(usize, String, String)>::new());
    let s = scope.clone();
    let reload: Rc<dyn Fn()> = Rc::new(move || {
        let url = url.get();
        submit(&s, busy, message, async move {
            let url = configured_url(url)?;
            let status = api::palpo_admin::get_server_status(&url).await?;
            let process = api::palpo_admin::get_server_running_process(&url).await;
            let mut lines = vec![format!(
                "Host: {} · Healthy: {} · Maintenance: {}",
                status.host.unwrap_or_default(),
                status.ok,
                status.maintenance
            )];
            let mut links = Vec::new();
            for (index, c) in status.results.into_iter().enumerate() {
                if !c.ok {
                    if let Some(url) = c.help.as_ref().filter(|url| {
                        url.starts_with("https://")
                            || url.starts_with("http://")
                            || url.starts_with('/')
                    }) {
                        links.push((
                            index,
                            url.clone(),
                            c.label.clone().unwrap_or_else(|| "component".into()),
                        ));
                    }
                }
                lines.push(format!(
                    "{} · {} · {} · {} · {} · {}",
                    c.label.unwrap_or_default(),
                    c.category.unwrap_or_default(),
                    if c.ok { "OK" } else { "FAILED" },
                    c.reason.unwrap_or_default(),
                    c.url.unwrap_or_default(),
                    c.help.unwrap_or_default()
                ));
            }
            lines.push(match process {
                Ok(p) => format!(
                    "Current command: {} · Locked at: {} · Maintenance: {}",
                    p.command.unwrap_or_else(|| "idle".into()),
                    p.locked_at.unwrap_or_default(),
                    p.maintenance
                ),
                Err(e) => format!("Process/lock status failed: {e}"),
            });
            help.revise(|v| *v = links);
            detail.revise(|stored_value| *stored_value = lines.join("\n"));
            Ok("Server status refreshed".into())
        });
    });
    if !url.get().is_empty() {
        reload();
    }
    page("Server status")
        .fill(label("Palpo admin sidecar URL"))
        .fill(url_input)
        .fill(action(
            "Save sidecar configuration",
            move || match configured_url(url.get()) {
                Ok(value) => {
                    storage::set_item("palpo_admin_url", &value);
                    message.revise(|stored_value| *stored_value = "Sidecar URL saved".into());
                }
                Err(e) => message.revise(|stored_value| *stored_value = e.to_string()),
            },
        ))
        .fill(action("Refresh / Retry", move || reload()))
        .fill(status(message))
        .fill(status(detail))
        .fill(each(
            help,
            |entry| entry.0,
            move |entry, _| {
                link()
                    .target(entry.map(|entry| entry.1.clone()))
                    .label(entry.map(|entry| format!("Component help: {}", entry.2)))
            },
        ))
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct NoticeHistory {
    user_id: String,
    message: String,
    timestamp: String,
    event_id: String,
}
fn notice_history_add(history: Cage<Vec<NoticeHistory>>, user: &str, body: &str, event: &str) {
    history.revise(|items| {
        items.insert(
            0,
            NoticeHistory {
                user_id: user.into(),
                message: body.into(),
                event_id: event.into(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            },
        );
        items.truncate(50);
    });
    let serialized =
        history.with(|items| serde_json::to_string(items).expect("notice history serializes"));
    storage::set_item("server_notice_history", &serialized);
}
fn notices(scope: &Scope) -> View {
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let progress = scope.cage(String::new());
    let history = scope.cage(
        storage::get_item("server_notice_history")
            .and_then(|v| serde_json::from_str::<Vec<NoticeHistory>>(&v).ok())
            .unwrap_or_default()
            .into_iter()
            .take(50)
            .collect::<Vec<_>>(),
    );
    let (user, user_input) = field(scope, "Target user Matrix ID");
    let (body, body_input) = field(scope, "Notice message");
    let s = scope.clone();
    let broadcast_owner = scope.clone();
    page("Send server notice")
        .fill(label("Target user Matrix ID"))
        .fill(user_input)
        .fill(label("Notice message"))
        .fill(body_input.multiline(true))
        .fill(action("Send notice", move || {
            let recipient = user.get(); let content = body.get();
            submit(&s, busy, message, async move {
                let recipient = required(&recipient, "Target user")?;
                if !recipient.starts_with('@') || !recipient.contains(':') {
                    return Err(HttpError::message("Enter a complete Matrix user ID (@user:server)"));
                }
                if content.trim().is_empty() { return Err(HttpError::message("Notice message is required")); }
                let event = api::users::send_server_notice(&recipient,&content).await?;
                notice_history_add(history,&recipient,&content,&event);
                body.revise(|v|v.clear());
                Ok(format!("Notice sent · Event {event}"))
            });
        }))
        .fill(action("Broadcast notice to all users", move || {
            if busy.get() { return; }
            let content = body.get();
            if content.trim().is_empty() { message.revise(|v|*v="Notice message is required".into()); return; }
            if !confirm("Send this server notice to every user? This cannot be undone.") { return; }
            submit(&broadcast_owner,busy,message,async move {
                use futures::stream::{self,StreamExt};
                let mut page = 1u64; let mut attempted=0usize; let mut sent=0usize; let mut failed=0usize;
                loop {
                    let result=api::users::get_users(page,500,"name","ASC","").await?;
                    let count=result.data.len();
                    if count==0 { break; }
                    let content_ref=&content;
                    let mut pending=stream::iter(result.data.into_iter()).map(|user| async move {
                        let result=api::users::send_server_notice(&user.id,content_ref).await;
                        (user.id,result)
                    }).buffer_unordered(8);
                    while let Some((user,result))=pending.next().await {
                        attempted+=1;
                        match result { Ok(event)=>{sent+=1;notice_history_add(history,&user,&content,&event);},Err(_)=>failed+=1 }
                        progress.revise(|v|*v=format!("Broadcast progress: {attempted} completed · {sent} sent · {failed} failed"));
                    }
                    if page.saturating_mul(500)>=result.total { break; }
                    page+=1;
                }
                if failed>0 { return Err(HttpError::message(format!("Broadcast: {sent} sent, {failed} failed out of {attempted}"))); }
                body.revise(|v|v.clear());
                Ok(format!("Broadcast sent to {sent} users"))
            });
        }))
        .fill(status(progress)).fill(status(message))
        .fill(label("Notice history"))
        .fill(action("Clear notice history",move || {
            if confirm("Clear all locally stored notice history?") {
                history.revise(|v|v.clear());storage::remove_item("server_notice_history");
                message.revise(|v|*v="Notice history cleared".into());
            }
        }))
        .fill(text().content(history.map(|items|if items.is_empty(){"No sent notices".to_owned()}else{format!("{} sent notices",items.len())})))
        .fill(each(history,|item|(item.event_id.clone(),item.user_id.clone(),item.timestamp.clone()),move |item,_|view().class("record").fill(text().content(item.map(|item|format!("{} · {} · {}\n{}",item.user_id,item.event_id,item.timestamp,item.message))))))
}

fn notifications(scope: &Scope) -> View {
    let (url, url_input) = sidecar(scope);
    let items = scope.cage(Vec::<ServerNotification>::new());
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let s = scope.clone();
    let reload: Rc<dyn Fn()> = Rc::new(move || {
        let url = url.get();
        submit(&s, busy, message, async move {
            let result = api::palpo_admin::get_server_notifications(&configured_url(url)?).await?;
            let n = result.notifications.len();
            items.revise(|stored_value| *stored_value = result.notifications);
            Ok(if n == 0 {
                "No server notifications".into()
            } else {
                format!("{n} notifications")
            })
        });
    });
    if !url.get().is_empty() {
        reload();
    }
    let s = scope.clone();
    page("Server notifications")
        .fill(local_notifications(scope))
        .fill(label("Palpo admin sidecar URL"))
        .fill(url_input)
        .fill(action("Refresh / Retry", move || reload()))
        .fill(action("Clear server notifications", move || {
            if !confirm("Clear all server notifications?") {
                return;
            }
            let url = url.get();
            submit(&s, busy, message, async move {
                api::palpo_admin::delete_server_notifications(&configured_url(url)?).await?;
                items.revise(|stored_value| *stored_value = Vec::new());
                Ok("Server notifications cleared".into())
            });
        }))
        .fill(status(message))
        .fill(each(
            items,
            |n| n.event_id.clone(),
            move |n, _| {
                view().class("record").fill(text().content(n.map(|n| {
                    format!(
                        "{} · {}\n{}",
                        n.event_id,
                        n.sent_at.clone().unwrap_or_default(),
                        n.output
                    )
                })))
            },
        ))
}

fn local_notifications(scope: &Scope) -> View {
    let Some(context) = crate::ui::context::get(scope) else {
        return view();
    };
    let entries = context.notifications;
    view()
        .class("panel")
        .fill(label("Application notifications"))
        .fill(text().content(entries.map(|items| {
            if items.is_empty() {
                "No application notifications".to_owned()
            } else {
                format!("{} application notifications", items.len())
            }
        })))
        .fill(action("Mark all notifications read", move || {
            entries.revise(|items| {
                for item in items {
                    item.read = true;
                }
            })
        }))
        .fill(action("Clear application notifications", move || {
            if confirm("Clear all application notifications?") {
                entries.revise(|items| items.clear());
            }
        }))
        .fill(each(
            entries,
            |item| item.id,
            move |item, _| {
                view()
                    .class("record")
                    .fill(text().content(item.map(|item| {
                        format!(
                            "{} · {} · {}",
                            item.kind,
                            if item.read { "Read" } else { "Unread" },
                            item.message
                        )
                    })))
                    .fill(action("Mark notification read", move || {
                        let id = item.get().id;
                        entries.revise(|items| {
                            if let Some(item) = items.iter_mut().find(|item| item.id == id) {
                                item.read = true;
                            }
                        });
                    }))
                    .fill(action("Dismiss notification", move || {
                        let id = item.get().id;
                        entries.revise(|items| items.retain(|item| item.id != id));
                    }))
            },
        ))
}

pub fn notification_enabled(kind: &str) -> bool {
    storage::get_item("notification_preferences")
        .and_then(|v| serde_json::from_str::<serde_json::Value>(&v).ok())
        .and_then(|v| v.get(kind).and_then(|v| v.as_bool()))
        .unwrap_or(true)
}
fn preferences(scope: &Scope) -> View {
    let context = crate::ui::context::get(scope);
    let message = scope.cage(String::new());
    let mut root = page("Notification preferences");
    for (key, caption) in [
        ("report_filed", "Report filed"),
        ("user_registered", "User registered"),
        ("federation_alert", "Federation alert"),
        ("job_failure", "Job failure"),
        ("system_info", "System information"),
    ] {
        let context = context.clone();
        let initial = context
            .as_ref()
            .map(|c| c.preferences.with(|p| p.get(key).copied().unwrap_or(true)))
            .unwrap_or_else(|| notification_enabled(key));
        let selected = scope.cage(initial);
        root = root
            .fill(
                check()
                    .name(localized(scope, caption.to_owned()))
                    .checked(selected)
                    .on(checked_changed, move |event| {
                        if let glory::render::Event::CheckedChanged(checked) = event {
                            selected.revise(|stored_value| *stored_value = checked);
                            if let Some(context) = &context {
                                context.preference(key, checked);
                            }
                            let mut prefs = storage::get_item("notification_preferences")
                                .and_then(|v| serde_json::from_str::<serde_json::Value>(&v).ok())
                                .unwrap_or_else(|| serde_json::json!({}));
                            prefs[key] = serde_json::json!(checked);
                            storage::set_item("notification_preferences", &prefs.to_string());
                            message.revise(|stored_value| {
                                *stored_value = "Notification preferences saved".into()
                            });
                        }
                    }),
            )
            .fill(label(caption));
    }
    root.fill(status(message))
}

fn server_actions(scope: &Scope) -> View {
    let (url, url_input) = sidecar(scope);
    let commands = scope.cage(Vec::<(String, ServerCommand)>::new());
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let locked = scope.cage(false);
    let (command, command_input) = field(scope, "Command name");
    let (arguments, args_input) = field(scope, "Additional command arguments");
    let original_arguments = scope.cage(None::<serde_json::Value>);
    let s = scope.clone();
    let load: Rc<dyn Fn()> = Rc::new(move || {
        let url = url.get();
        submit(&s, busy, message, async move {
            let url = configured_url(url)?;
            let process = api::palpo_admin::get_server_running_process(&url).await?;
            locked.revise(|stored_value| *stored_value = process.command.is_some());
            let mut catalog = api::palpo_admin::get_server_commands(&url)
                .await?
                .into_iter()
                .collect::<Vec<_>>();
            catalog.sort_by(|a, b| a.0.cmp(&b.0));
            let n = catalog.len();
            commands.revise(|stored_value| *stored_value = catalog);
            Ok(format!(
                "{n} commands · Running: {}",
                process.command.unwrap_or_else(|| "none".into())
            ))
        });
    });
    if !url.get().is_empty() {
        load();
    }
    let s = scope.clone();
    page("Server actions")
        .fill(label("Palpo admin sidecar URL"))
        .fill(url_input)
        .fill(action("Refresh commands / Retry", move || load()))
        .fill(status(message))
        .fill(each(
            commands,
            |c| c.0.clone(),
            move |entry, _| {
                view()
                    .class("record")
                    .fill(text().content(entry.map(|(key, c)| {
                        format!(
                            "{} ({key}) · {} · Requires lock: {}\nArguments: {}",
                            c.name,
                            c.description.clone().unwrap_or_default(),
                            c.with_lock,
                            c.args
                                .as_ref()
                                .map(ToString::to_string)
                                .unwrap_or_else(|| "none".into())
                        )
                    })))
                    .fill(action("Select command", move || {
                        command.revise(|stored_value| *stored_value = entry.get().0);
                        let selected_args = entry.get().1.additional_args;
                        let displayed = command_argument_text(&selected_args);
                        original_arguments.revise(|v| *v = selected_args);
                        arguments.revise(|v| *v = displayed);
                    }))
            },
        ))
        .fill(label("Command name"))
        .fill(command_input)
        .fill(label("Additional command arguments"))
        .fill(args_input)
        .fill(action("Run command", move || {
            if locked.get() {
                message.revise(|stored_value| {
                    *stored_value =
                        "A command is already running; refresh its status before trying again"
                            .into()
                });
                return;
            }
            let target = command.get();
            if !commands.with(|rows| rows.iter().any(|(name, _)| name == &target)) {
                message.revise(|stored_value| {
                    *stored_value = "Select a command from the server catalog".into()
                });
                return;
            }
            if !confirm(&format!(
                "Run server command {target}? This may change server data or availability."
            )) {
                return;
            }
            let url = url.get();
            let args = arguments.get();
            let original = original_arguments.get();
            submit(&s, busy, message, async move {
                let url = configured_url(url)?;
                api::palpo_admin::run_server_command(
                    &url,
                    &target,
                    edited_command_arguments(&args, &original),
                )
                .await?;
                let p = api::palpo_admin::get_server_running_process(&url).await?;
                locked.revise(|stored_value| *stored_value = p.command.is_some());
                Ok("Command accepted by server".into())
            });
        }))
        .fill(schedule_editor(scope, url, commands, false))
        .fill(schedule_editor(scope, url, commands, true))
}
#[derive(Clone)]
struct ScheduleRow {
    id: String,
    command: String,
    args: Option<serde_json::Value>,
    time: Option<String>,
    scheduled_at: Option<String>,
    is_recurring: bool,
}
fn schedule_editor(
    scope: &Scope,
    url: Cage<String>,
    catalog: Cage<Vec<(String, ServerCommand)>>,
    recurring: bool,
) -> View {
    let items = scope.cage(Vec::<ScheduleRow>::new());
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let editing = scope.cage(false);
    let preserved = scope.cage((None::<String>, false));
    let original_args = scope.cage(None::<serde_json::Value>);
    let (id, id_input) = field(scope, "Schedule ID (blank for new)");
    let (command, command_input) = field(scope, "Scheduled command");
    let (args, args_input) = field(scope, "Scheduled arguments");
    let (time, time_input) = field(
        scope,
        if recurring {
            "Recurring time / expression"
        } else {
            "Scheduled at (RFC3339 timestamp)"
        },
    );
    let s = scope.clone();
    let load: Rc<dyn Fn()> = Rc::new(move || {
        let url = url.get();
        submit(&s, busy, message, async move {
            let url = configured_url(url)?;
            let rows = if recurring {
                api::palpo_admin::get_recurring_commands(&url)
                    .await?
                    .into_iter()
                    .map(|v| ScheduleRow {
                        id: v.id,
                        command: v.command,
                        args: v.args,
                        time: v.time,
                        scheduled_at: v.scheduled_at,
                        is_recurring: false,
                    })
                    .collect()
            } else {
                api::palpo_admin::get_scheduled_commands(&url)
                    .await?
                    .into_iter()
                    .map(|v| ScheduleRow {
                        id: v.id,
                        command: v.command,
                        args: v.args,
                        time: v.scheduled_at,
                        scheduled_at: None,
                        is_recurring: v.is_recurring,
                    })
                    .collect::<Vec<_>>()
            };
            let n = rows.len();
            items.revise(|stored_value| *stored_value = rows);
            Ok(if n == 0 {
                "No scheduled commands".into()
            } else {
                format!("{n} commands")
            })
        });
    });
    if !url.get().is_empty() {
        load();
    }
    let s = scope.clone();
    let table = each(
        items,
        |r| r.id.clone(),
        move |entry, _| {
            let s = s.clone();
            view()
                .class("record")
                .fill(text().content(entry.map(|r| {
                    format!(
                        "{} · {} · {} · {}",
                        r.id,
                        r.command,
                        r.time.clone().unwrap_or_default(),
                        r.args.as_ref().map(ToString::to_string).unwrap_or_default()
                    )
                })))
                .fill(action("Edit schedule", move || {
                    let row = entry.get();
                    id.revise(|stored_value| *stored_value = row.id);
                    command.revise(|stored_value| *stored_value = row.command);
                    original_args.revise(|v| *v = row.args.clone());
                    args.revise(|stored_value| {
                        *stored_value = row
                            .args
                            .map(|v| match v {
                                serde_json::Value::String(s) => s,
                                v => v.to_string(),
                            })
                            .unwrap_or_default()
                    });
                    time.revise(|stored_value| *stored_value = row.time.unwrap_or_default());
                    preserved.revise(|stored_value| {
                        *stored_value = (row.scheduled_at, row.is_recurring)
                    });
                    editing.revise(|stored_value| *stored_value = true);
                }))
                .fill(action("Delete schedule", move || {
                    if !confirm("Delete this scheduled command?") {
                        return;
                    }
                    let target = entry.get().id;
                    let url = url.get();
                    submit(&s, busy, message, async move {
                        let url = configured_url(url)?;
                        if recurring {
                            api::palpo_admin::delete_recurring_command(&url, &target).await?;
                        } else {
                            api::palpo_admin::delete_scheduled_command(&url, &target).await?;
                        }
                        items.revise(|rows| rows.retain(|v| v.id != target));
                        Ok("Schedule deleted".into())
                    });
                }))
        },
    );
    let s = scope.clone();
    view()
        .class("panel")
        .fill(label(if recurring {
            "Recurring commands"
        } else {
            "One-time scheduled commands"
        }))
        .fill(action("Refresh schedules / Retry", move || load()))
        .fill(status(message))
        .fill(table)
        .fill(label("Schedule ID (blank for new)"))
        .fill(id_input.enabled(editing.map(|v| !*v)))
        .fill(label("Command name"))
        .fill(command_input)
        .fill(label("Scheduled arguments"))
        .fill(args_input)
        .fill(label("Execution time"))
        .fill(time_input)
        .fill(action("New schedule", move || {
            editing.revise(|stored_value| *stored_value = false);
            id.revise(|stored_value| *stored_value = String::new());
            command.revise(|stored_value| *stored_value = String::new());
            args.revise(|stored_value| *stored_value = String::new());
            time.revise(|stored_value| *stored_value = String::new());
            preserved.revise(|stored_value| *stored_value = (None, false));
            original_args.revise(|v| *v = None);
        }))
        .fill(action("Save schedule", move || {
            let target = command.get();
            if !catalog.with(|rows| rows.iter().any(|(key, _)| key == &target)) {
                message.revise(|stored_value| {
                    *stored_value = "Select a command from the loaded server catalog".into()
                });
                return;
            }
            if !confirm("Save this server command schedule?") {
                return;
            }
            let url = url.get();
            let id_cage = id;
            let id = id.get();
            let time = time.get();
            let args = args.get();
            let original = original_args.get();
            let edit = editing.get();
            let old = preserved.get();
            submit(&s, busy, message, async move {
                let url = configured_url(url)?;
                let time = required(&time, "Execution time")?;
                if !recurring {
                    chrono::DateTime::parse_from_rfc3339(&time).map_err(|_| {
                        HttpError::message(
                            "One-time execution time must be RFC3339, including a timezone",
                        )
                    })?;
                }
                let args = edited_command_arguments(&args, &original);
                let saved = if recurring {
                    let value = RecurringCommand {
                        id,
                        command: target,
                        args,
                        time: Some(time),
                        scheduled_at: old.0,
                    };
                    let v = if edit {
                        api::palpo_admin::update_recurring_command(&url, &value).await?
                    } else {
                        api::palpo_admin::create_recurring_command(&url, &value).await?
                    };
                    ScheduleRow {
                        id: v.id,
                        command: v.command,
                        args: v.args,
                        time: v.time,
                        scheduled_at: v.scheduled_at,
                        is_recurring: false,
                    }
                } else {
                    let value = ScheduledCommand {
                        id,
                        command: target,
                        args,
                        scheduled_at: Some(time),
                        is_recurring: old.1,
                    };
                    let v = if edit {
                        api::palpo_admin::update_scheduled_command(&url, &value).await?
                    } else {
                        api::palpo_admin::create_scheduled_command(&url, &value).await?
                    };
                    ScheduleRow {
                        id: v.id,
                        command: v.command,
                        args: v.args,
                        time: v.scheduled_at,
                        scheduled_at: None,
                        is_recurring: v.is_recurring,
                    }
                };
                original_args.revise(|v| *v = saved.args.clone());
                id_cage.revise(|v| *v = saved.id.clone());
                editing.revise(|v| *v = true);
                preserved.revise(|v| *v = (saved.scheduled_at.clone(), saved.is_recurring));
                items.revise(|rows| {
                    rows.retain(|v| v.id != saved.id);
                    rows.push(saved);
                });
                Ok("Schedule saved".into())
            });
        }))
}

fn command_argument_text(value: &Option<serde_json::Value>) -> String {
    match value {
        Some(serde_json::Value::String(text)) => text.clone(),
        Some(value) => value.to_string(),
        None => String::new(),
    }
}
fn edited_command_arguments(
    text: &str,
    original: &Option<serde_json::Value>,
) -> Option<serde_json::Value> {
    if text == command_argument_text(original) {
        original.clone()
    } else if text.is_empty() {
        None
    } else {
        Some(command_arguments(text))
    }
}

fn command_arguments(value: &str) -> serde_json::Value {
    serde_json::from_str(value).unwrap_or_else(|_| serde_json::json!(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn command_arguments_preserve_structured_server_fields() {
        assert_eq!(
            command_arguments("{\"limit\":3}"),
            serde_json::json!({"limit":3})
        );
        assert_eq!(command_arguments("--all"), serde_json::json!("--all"));
        for value in [
            serde_json::json!("42"),
            serde_json::json!("{}"),
            serde_json::json!(""),
            serde_json::json!({"limit":3}),
        ] {
            let original = Some(value);
            assert_eq!(
                edited_command_arguments(&command_argument_text(&original), &original),
                original
            );
        }
        assert_eq!(
            edited_command_arguments("{\"limit\":4}", &Some(serde_json::json!("old"))),
            Some(serde_json::json!({"limit":4}))
        );
    }
    #[test]
    fn numeric_operation_parameters_do_not_silently_become_zero() {
        assert_eq!(number("12", "size").unwrap(), 12);
        assert!(number("-1", "size").is_err());
        assert!(number("1.5", "size").is_err());
        assert!(number("18446744073709551616", "size").is_err());
        assert_eq!(optional_number("", "uses").unwrap(), None);
        assert!(optional_number("garbage", "uses").is_err());
    }
}
