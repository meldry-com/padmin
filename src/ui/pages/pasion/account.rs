use super::*;

fn own_username() -> Option<String> {
    let mxid = crate::utils::storage::get_item("user_id")?;
    Some(mxid.strip_prefix('@')?.split(':').next()?.to_string())
}

async fn reload(
    id: &str,
    user: Cage<Option<PasionUser>>,
    emails: Cage<Vec<PasionUserEmail>>,
    browser: Cage<Vec<PasionBrowserSession>>,
    oauth: Cage<Vec<PasionOAuth2Session>>,
) -> Result<(), HttpError> {
    let u = pasion_get_user(id).await?;
    let _ = user.try_revise(|slot| *slot = Some(u));
    let loaded = pasion_get_user_emails(Some(id)).await?;
    let _ = emails.try_revise(|slot| *slot = loaded);
    let loaded = pasion_get_browser_sessions(Some(id), true).await?;
    let _ = browser.try_revise(|slot| *slot = loaded);
    let loaded = pasion_get_active_oauth2_sessions_for_user(id).await?;
    let _ = oauth.try_revise(|slot| *slot = loaded);
    Ok(())
}

pub(super) fn render(scope: &Scope, id: String) -> View {
    let user = scope.cage(None::<PasionUser>);
    let emails = scope.cage(Vec::<PasionUserEmail>::new());
    let browser = scope.cage(Vec::<PasionBrowserSession>::new());
    let oauth = scope.cage(Vec::<PasionOAuth2Session>::new());
    let (visible_emails, email_pages) = collection_window(scope, emails, "emails");
    let (visible_browser, browser_pages) = collection_window(scope, browser, "browser sessions");
    let (visible_oauth, oauth_pages) = collection_window(scope, oauth, "client sessions");
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let (name, name_input) = field(scope, "Display name");
    let (locale, locale_input) = field(scope, "Preferred locale");
    let (avatar, avatar_input) = field(scope, "Avatar URL");
    let first = scope.cage(true);
    let s = scope.clone();
    let uid = id.clone();
    let refresh: Rc<dyn Fn()> = Rc::new(move || {
        let id = uid.clone();
        load_quiet(&s, busy, message, async move {
            reload(&id, user, emails, browser, oauth).await?;
            if first.try_get().unwrap_or(false) {
                if let Ok(Some(u)) = user.try_get() {
                    let _ = name.try_revise(|slot| *slot = u.display_name.unwrap_or_default());
                    let _ =
                        locale.try_revise(|slot| *slot = u.preferred_locale.unwrap_or_default());
                    let _ = avatar.try_revise(|slot| *slot = u.avatar_url.unwrap_or_default());
                }
                let _ = first.try_revise(|slot| *slot = false);
            }
            Ok("Account loaded".into())
        });
    });
    refresh();
    let s = scope.clone();
    let uid = id.clone();
    let profile=view().name("Account profile").fill((label("Profile"),name_input,locale_input,avatar_input,action("Save profile",move||{
        let id=uid.clone();let n=name.get();let l=locale.get();let a=avatar.get();submit(&s,busy,message,async move{let u=pasion_update_user(&id,serde_json::json!({"display_name":nullable(n),"preferred_locale":nullable(l),"avatar_url":nullable(a)})).await?;let _=user.try_revise(|slot| *slot = Some(u));Ok("Profile saved".into())});
    }).enabled(user.map(|u|u.as_ref().is_some_and(|u|u.deactivated_at.is_none())))));
    let (erase, erase_input) = check_field(
        scope,
        "Erase homeserver data on deactivation (irreversible)",
        false,
    );
    let mut controls = Vec::new();
    for (title, property, value, protect_self) in [
        ("Grant administrator", "admin", true, false),
        ("Revoke administrator", "admin", false, true),
        ("Lock account", "locked", true, true),
        ("Unlock account", "locked", false, false),
        ("Deactivate account", "deactivated", true, true),
        ("Reactivate account", "deactivated", false, false),
    ] {
        let s = scope.clone();
        let uid = id.clone();
        controls.push(
            action(title, move || {
                let Some(u) = user.get() else {
                    return;
                };
                if protect_self && own_username().as_deref() == Some(u.username.as_str()) {
                    message.revise(|slot| {
                        *slot = "This operation is disabled for your own account".into()
                    });
                    return;
                }
                if !confirm(&format!(
                    "{title} for {}?{}",
                    u.username,
                    if property == "deactivated" && value && erase.get() {
                        " Homeserver data will be erased permanently."
                    } else {
                        ""
                    }
                )) {
                    return;
                }
                let id = uid.clone();
                let mut payload = serde_json::json!({property:value});
                if property == "deactivated" && value {
                    payload["hs_erase"] = erase.get().into();
                }
                submit(&s, busy, message, async move {
                    let u = pasion_update_user(&id, payload).await?;
                    let _ = user.try_revise(|slot| *slot = Some(u));
                    let _ = erase.try_revise(|slot| *slot = false);
                    reload(&id, user, emails, browser, oauth).await?;
                    Ok(format!("{title} completed"))
                });
            })
            .enabled(user.map(move |u| {
                u.as_ref().is_some_and(|u| {
                    let self_account = own_username().as_deref() == Some(u.username.as_str());
                    if protect_self && self_account {
                        return false;
                    }
                    match property {
                        "admin" => u.admin != value,
                        "locked" => (u.locked_at.is_some()) != value,
                        "deactivated" => (u.deactivated_at.is_some()) != value,
                        _ => false,
                    }
                })
            })),
        );
    }
    let controls = view().name("Account access").fill((
        label("Access and account state"),
        erase_input,
        view().fill(controls),
    ));
    let (password, password_input) = field(scope, "New password");
    let (skip, skip_input) = check_field(scope, "Skip password policy", false);
    let (end_sessions, end_input) =
        check_field(scope, "End browser sessions after password reset", true);
    let s = scope.clone();
    let uid = id.clone();
    let password_form = view().name("Password reset").fill((
        label("Set password"),
        password_input.purpose(Purpose::Password),
        skip_input,
        end_input,
        action("Reset password", move || {
            if !confirm("Set a new password for this Pasion account?") {
                return;
            }
            let id = uid.clone();
            let pw = password.get();
            let skip = skip.get();
            let end = end_sessions.get();
            submit(&s, busy, message, async move {
                required(pw.clone(), "Password")?;
                pasion_set_password(&id, &pw, skip).await?;
                let _ = password.try_revise(|slot| *slot = String::new());
                if end {
                    if let Err(e) = pasion_risk_action(
                        &id,
                        "terminate_sessions",
                        Some("password reset by admin"),
                    )
                    .await
                    {
                        return Ok(format!("Password changed; ending sessions failed: {e}"));
                    }
                    let loaded = pasion_get_browser_sessions(Some(&id), true).await?;
                    let _ = browser.try_revise(|slot| *slot = loaded);
                }
                Ok("Password changed".into())
            });
        })
        .enabled(user.map(|u| u.as_ref().is_some_and(|u| u.deactivated_at.is_none()))),
    ));
    let (reason, reason_input) = field(scope, "Risk action reason (optional)");
    let mut risk_buttons = Vec::new();
    for (title, risk) in [
        ("Terminate browser sessions", "terminate_sessions"),
        ("Risk: lock account", "lock"),
        (
            "Require password reset (locks account)",
            "force_password_reset",
        ),
    ] {
        let s = scope.clone();
        let uid = id.clone();
        risk_buttons.push(
            action(title, move || {
                if risk != "terminate_sessions"
                    && user
                        .get()
                        .as_ref()
                        .is_some_and(|u| own_username().as_deref() == Some(u.username.as_str()))
                {
                    message.revise(|slot| *slot = "Cannot lock your own account".into());
                    return;
                }
                if !confirm(title) {
                    return;
                }
                let id = uid.clone();
                let reason = optional(reason.get());
                submit(&s, busy, message, async move {
                    let result = pasion_risk_action(&id, risk, reason.as_deref()).await?;
                    reload(&id, user, emails, browser, oauth).await?;
                    Ok(format!(
                        "Risk action completed; {} sessions terminated",
                        result.sessions_terminated.unwrap_or(0)
                    ))
                });
            })
            .enabled(busy.map(|b| !b)),
        );
    }
    let risks = view().name("Risk actions").fill((
        label("Risk actions"),
        reason_input,
        view().fill(risk_buttons),
    ));
    let (email, email_input) = field(scope, "New email address");
    let s = scope.clone();
    let uid = id.clone();
    let es = scope.clone();
    let email_section = view().name("Account emails").fill((
        label("Email addresses"),
        email_input.purpose(Purpose::Email),
        action("Add email", move || {
            let id = uid.clone();
            let address = email.get();
            submit(&s, busy, message, async move {
                let address = required(address, "Email")?;
                if !address.contains('@') || address.chars().any(char::is_whitespace) {
                    return Err(HttpError::message("Enter a valid email address"));
                }
                pasion_add_user_email(&id, &address).await?;
                let loaded = pasion_get_user_emails(Some(&id)).await?;
                let _ = emails.try_revise(|slot| *slot = loaded);
                let _ = email.try_revise(|slot| *slot = String::new());
                Ok("Email added".into())
            });
        }),
        email_pages,
        each(
            visible_emails,
            |e| e.id.clone(),
            move |e, _| {
                let id = e.get().id;

                let s = es.clone();

                view().fill((
                    text().content(e.map(|e| {
                        format!(
                            "{} · {} · {}",
                            e.email,
                            if e.is_primary { "primary" } else { "secondary" },
                            if e.confirmed_at.is_some() {
                                "confirmed"
                            } else {
                                "unconfirmed"
                            }
                        )
                    })),
                    action("Remove email", move || {
                        if !confirm("Remove this email address?") {
                            return;
                        }
                        let id = id.clone();
                        submit(&s, busy, message, async move {
                            pasion_delete_user_email(&id).await?;
                            let _ = emails.try_revise(|rows| rows.retain(|row| row.id != id));
                            Ok("Email removed".into())
                        });
                    }),
                ))
            },
        ),
    ));
    let bs = scope.clone();
    let os = scope.clone();
    let sessions = view().name("Account sessions").fill((
        label("Active browser sessions"),
        browser_pages,
        each(
            visible_browser,
            |r| r.id.clone(),
            move |r, _| {
                let s = bs.clone();
                let id = r.get().id;
                view().fill((
                    text().content(r.map(|r| {
                        format!(
                            "{} · created {} · last active {} · IP {} · {}",
                            r.id,
                            r.created_at,
                            r.last_active_at.as_deref().unwrap_or("—"),
                            r.last_active_ip.as_deref().unwrap_or("—"),
                            r.user_agent.as_deref().unwrap_or("—")
                        )
                    })),
                    action("End browser session", move || {
                        if !confirm("End this browser session?") {
                            return;
                        }
                        let id = id.clone();
                        submit(&s, busy, message, async move {
                            pasion_finish_browser_session(&id).await?;
                            let _ = browser.try_revise(|rows| rows.retain(|row| row.id != id));
                            Ok("Browser session ended".into())
                        });
                    }),
                ))
            },
        ),
        label("Active OAuth2 client sessions"),
        oauth_pages,
        each(
            visible_oauth,
            |r| r.id.clone(),
            move |r, _| {
                let s = os.clone();
                let id = r.get().id;
                view().fill((
                    text().content(r.map(|r| {
                        format!(
                            "{} · client {} · scope {} · last active {}",
                            r.id,
                            r.client_id,
                            r.scope,
                            r.last_active_at.as_deref().unwrap_or("—")
                        )
                    })),
                    action("End client session", move || {
                        if !confirm("End this client session and revoke its tokens?") {
                            return;
                        }
                        let id = id.clone();
                        submit(&s, busy, message, async move {
                            pasion_finish_oauth2_session(&id).await?;
                            let _ = oauth.try_revise(|rows| rows.retain(|row| row.id != id));
                            Ok("Client session ended".into())
                        });
                    }),
                ))
            },
        ),
    ));
    page("Pasion account").fill((
        action("Back to accounts", || {
            crate::ui::navigate("/pasion/accounts")
        }),
        detail("Account ID", id),
        action("Refresh account", move || refresh()).enabled(busy.map(|b| !b)),
        status(message),
        text().content(user.map(|u| {
            u.as_ref()
                .map(|u| {
                    format!(
                        "{} · {} · {} · created {}",
                        u.username,
                        if u.admin { "administrator" } else { "user" },
                        if u.deactivated_at.is_some() {
                            "deactivated"
                        } else if u.locked_at.is_some() {
                            "locked"
                        } else {
                            "active"
                        },
                        u.created_at
                    )
                })
                .unwrap_or_default()
        })),
        profile,
        controls,
        password_form,
        risks,
        email_section,
        sessions,
    ))
}
