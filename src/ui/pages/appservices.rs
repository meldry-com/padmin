//! Appservice catalog and structured registration editor.
use crate::api::appservices as api;
use crate::types::{AppserviceRegistration, AppserviceSummary};
use crate::ui::widgets::*;
use crate::utils::error::HttpError;
use glory::prelude::*;
use glory::ui::{Scope, View};
use std::rc::Rc;
use std::result::Result;
/// Static description of a ready-to-install appservice.
#[derive(Clone, Copy)]
struct Template {
    key: &'static str,
    name: &'static str,
    description: &'static str,
    _icon: &'static str,
    /// Tailwind classes used on the icon chip (bg + text color).
    _accent: &'static str,
    category: &'static str,
    default_id_prefix: &'static str,
    default_sender_localpart: &'static str,
    default_url: &'static str,
    /// Used as the localpart regex prefix — e.g. `slack_` → `@slack_.*`.
    user_prefix: &'static str,
    /// Used as the alias regex prefix — e.g. `slack_` → `#slack_.*`.
    alias_prefix: &'static str,
    /// Extra guidance shown in the configuration screen.
    notes: &'static str,
}

const TEMPLATES: &[Template] = &[
    Template {
        key: "slack",
        name: "Slack",
        description: "Bridge Matrix rooms to Slack channels via mautrix-slack.",
        _icon: "hash",
        _accent: "bg-purple-500/10 text-purple-500",
        category: "Bridge",
        default_id_prefix: "slack",
        default_sender_localpart: "slackbot",
        default_url: "http://localhost:29335",
        user_prefix: "slack_",
        alias_prefix: "slack_",
        notes: "Copy the as_token / hs_token from the mautrix-slack registration file \
                the bridge generated on first run.",
    },
    Template {
        key: "telegram",
        name: "Telegram",
        description: "Bridge Matrix rooms to Telegram chats via mautrix-telegram.",
        _icon: "send",
        _accent: "bg-sky-500/10 text-sky-500",
        category: "Bridge",
        default_id_prefix: "telegram",
        default_sender_localpart: "telegrambot",
        default_url: "http://localhost:29317",
        user_prefix: "telegram_",
        alias_prefix: "telegram_",
        notes: "Obtain api_id/api_hash at my.telegram.org, then use the tokens printed by \
                mautrix-telegram in its generated registration.yaml.",
    },
    Template {
        key: "discord",
        name: "Discord",
        description: "Bridge Matrix rooms to Discord servers via mautrix-discord.",
        _icon: "gamepad-2",
        _accent: "bg-indigo-500/10 text-indigo-500",
        category: "Bridge",
        default_id_prefix: "discord",
        default_sender_localpart: "discordbot",
        default_url: "http://localhost:29334",
        user_prefix: "discord_",
        alias_prefix: "discord_",
        notes: "Requires a Discord bot token created in the Discord developer portal.",
    },
    Template {
        key: "whatsapp",
        name: "WhatsApp",
        description: "Bridge Matrix rooms to WhatsApp chats via mautrix-whatsapp.",
        _icon: "message-circle",
        _accent: "bg-green-500/10 text-green-500",
        category: "Bridge",
        default_id_prefix: "whatsapp",
        default_sender_localpart: "whatsappbot",
        default_url: "http://localhost:29318",
        user_prefix: "whatsapp_",
        alias_prefix: "whatsapp_",
        notes: "Uses multidevice API — no business account required. Users pair by scanning a QR code.",
    },
    Template {
        key: "wechat",
        name: "WeChat",
        description: "Bridge Matrix rooms to WeChat (微信) via matrix-bridge-wechat.",
        _icon: "message-square",
        _accent: "bg-emerald-500/10 text-emerald-500",
        category: "Bridge",
        default_id_prefix: "wechat",
        default_sender_localpart: "wechatbot",
        default_url: "http://localhost:29319",
        user_prefix: "wechat_",
        alias_prefix: "wechat_",
        notes: "Requires the bridge to have an authenticated WeChat session (scan QR on startup).",
    },
    Template {
        key: "lark",
        name: "Feishu / Lark",
        description: "Bridge Matrix rooms to Feishu (飞书) or Lark workspaces.",
        _icon: "briefcase",
        _accent: "bg-cyan-500/10 text-cyan-500",
        category: "Bridge",
        default_id_prefix: "lark",
        default_sender_localpart: "larkbot",
        default_url: "http://localhost:29320",
        user_prefix: "lark_",
        alias_prefix: "lark_",
        notes: "Create a Feishu custom robot app and copy its App ID / App Secret into the bridge config.",
    },
    Template {
        key: "dingtalk",
        name: "DingTalk",
        description: "Bridge Matrix rooms to DingTalk (钉钉) groups.",
        _icon: "bell",
        _accent: "bg-blue-500/10 text-blue-500",
        category: "Bridge",
        default_id_prefix: "dingtalk",
        default_sender_localpart: "dingtalkbot",
        default_url: "http://localhost:29321",
        user_prefix: "dingtalk_",
        alias_prefix: "dingtalk_",
        notes: "Create an internal application in the DingTalk admin console and grant it IM permissions.",
    },
    Template {
        key: "qq",
        name: "QQ",
        description: "Bridge Matrix rooms to Tencent QQ groups.",
        _icon: "users",
        _accent: "bg-yellow-500/10 text-yellow-600",
        category: "Bridge",
        default_id_prefix: "qq",
        default_sender_localpart: "qqbot",
        default_url: "http://localhost:29322",
        user_prefix: "qq_",
        alias_prefix: "qq_",
        notes: "Uses an onebot-compatible backend (go-cqhttp, LLOneBot, Napcat …). Point the bridge at it.",
    },
    Template {
        key: "signal",
        name: "Signal",
        description: "Bridge Matrix rooms to Signal via mautrix-signal.",
        _icon: "shield",
        _accent: "bg-blue-600/10 text-blue-600",
        category: "Bridge",
        default_id_prefix: "signal",
        default_sender_localpart: "signalbot",
        default_url: "http://localhost:29328",
        user_prefix: "signal_",
        alias_prefix: "signal_",
        notes: "Users link Signal accounts by scanning a QR code generated by the bridge.",
    },
    Template {
        key: "irc",
        name: "IRC",
        description: "Bridge Matrix rooms to IRC networks via matrix-appservice-irc.",
        _icon: "terminal",
        _accent: "bg-orange-500/10 text-orange-500",
        category: "Bridge",
        default_id_prefix: "irc",
        default_sender_localpart: "ircbot",
        default_url: "http://localhost:9999",
        user_prefix: "irc_",
        alias_prefix: "irc_",
        notes: "The IRC bridge supports multiple networks. Configure them in config.yaml on the bridge side.",
    },
    Template {
        key: "hookshot",
        name: "Hookshot",
        description: "Forward GitHub, GitLab, JIRA, Figma and generic webhooks into Matrix rooms.",
        _icon: "webhook",
        _accent: "bg-fuchsia-500/10 text-fuchsia-500",
        category: "Bot",
        default_id_prefix: "hookshot",
        default_sender_localpart: "hookshot",
        default_url: "http://localhost:9993",
        user_prefix: "_webhooks_",
        alias_prefix: "webhooks_",
        notes: "Hookshot ships its own admin room — invite the sender bot after registration to configure it.",
    },
    Template {
        key: "custom",
        name: "Custom Appservice",
        description: "Paste a raw registration manually. Use this for bridges or bots without a built-in template.",
        _icon: "wrench",
        _accent: "bg-muted text-muted-foreground",
        category: "Custom",
        default_id_prefix: "appservice",
        default_sender_localpart: "",
        default_url: "",
        user_prefix: "",
        alias_prefix: "",
        notes: "",
    },
];

#[derive(Clone, Copy)]
struct Form {
    id: Cage<String>,
    url: Cage<String>,
    sender: Cage<String>,
    as_token: Cage<String>,
    hs_token: Cage<String>,
    namespaces: Cage<String>,
    protocols: Cage<String>,
    original_protocols: Cage<Option<Vec<String>>>,
    rate: Cage<String>,
    ephemeral: Cage<bool>,
    device: Cage<bool>,
    disabled: Cage<bool>,
}
impl Form {
    fn apply(self, reg: AppserviceRegistration) {
        self.id.revise(|v| *v = reg.id);
        self.url.revise(|v| *v = reg.url.unwrap_or_default());
        self.sender.revise(|v| *v = reg.sender_localpart);
        self.as_token.revise(|v| *v = reg.as_token);
        self.hs_token.revise(|v| *v = reg.hs_token);
        self.namespaces
            .revise(|v| *v = serde_json::to_string_pretty(&reg.namespaces).unwrap_or_default());
        self.original_protocols
            .revise(|v| *v = reg.protocols.clone());
        self.protocols
            .revise(|v| *v = reg.protocols.unwrap_or_default().join(", "));
        self.rate
            .revise(|v| *v = reg.rate_limited.map(|v| v.to_string()).unwrap_or_default());
        self.ephemeral.revise(|v| *v = reg.receive_ephemeral);
        self.device.revise(|v| *v = reg.device_management);
        self.disabled.revise(|v| *v = reg.disabled);
    }
    fn registration(self) -> Result<AppserviceRegistration, HttpError> {
        let id = self.id.get().trim().to_owned();
        let sender = self.sender.get().trim().to_owned();
        let at = self.as_token.get().trim().to_owned();
        let ht = self.hs_token.get().trim().to_owned();
        if id.is_empty() || sender.is_empty() || at.is_empty() || ht.is_empty() {
            return Err(HttpError::message(
                "ID, sender localpart, AS token and HS token are required",
            ));
        }
        if sender.contains(':')
            || sender.starts_with('@')
            || sender.chars().any(char::is_whitespace)
        {
            return Err(HttpError::message(
                "Sender localpart must not be a complete Matrix ID or contain whitespace",
            ));
        }
        let url = self.url.get().trim().to_owned();
        if !url.is_empty() && !(url.starts_with("http://") || url.starts_with("https://")) {
            return Err(HttpError::message(
                "Appservice URL must be HTTP(S), or empty",
            ));
        }
        let ns: serde_json::Value = serde_json::from_str(&self.namespaces.get())
            .map_err(|e| HttpError::message(format!("Namespaces must be JSON: {e}")))?;
        if !ns.is_object() {
            return Err(HttpError::message("Namespaces must be an object"));
        }
        for key in ["users", "aliases", "rooms"] {
            if let Some(entries) = ns.get(key) {
                let entries = entries.as_array().ok_or_else(|| {
                    HttpError::message(format!("{key} namespace must be an array"))
                })?;
                for entry in entries {
                    if entry.get("exclusive").and_then(|v| v.as_bool()).is_none()
                        || entry.get("regex").and_then(|v| v.as_str()).is_none()
                    {
                        return Err(HttpError::message(format!(
                            "Each {key} namespace requires exclusive and regex"
                        )));
                    }
                    // Namespace regex syntax belongs to the server's engine.
                    // regex-lite rejects supported Unicode classes, so the UI only
                    // checks structure and retains the original expression.
                }
            }
        }
        let rate = match self.rate.get().trim() {
            "" => None,
            "true" => Some(true),
            "false" => Some(false),
            _ => {
                return Err(HttpError::message(
                    "Rate limited must be blank, true or false",
                ));
            }
        };
        let raw_protocols = self.protocols.get();
        let original = self.original_protocols.get();
        let protocols =
            if raw_protocols == original.as_ref().map(|v| v.join(", ")).unwrap_or_default() {
                original
            } else {
                let edited = raw_protocols
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .collect::<Vec<_>>();
                if edited.is_empty() {
                    None
                } else {
                    Some(edited)
                }
            };
        Ok(AppserviceRegistration {
            id,
            url: if url.is_empty() { None } else { Some(url) },
            sender_localpart: sender,
            as_token: at,
            hs_token: ht,
            namespaces: ns,
            rate_limited: rate,
            protocols,
            receive_ephemeral: self.ephemeral.get(),
            device_management: self.device.get(),
            disabled: self.disabled.get(),
        })
    }
}
fn random_token() -> Result<String, HttpError> {
    #[cfg(target_arch = "wasm32")]
    {
        let mut bytes = [0u8; 32];
        let crypto = web_sys::window()
            .and_then(|w| w.crypto().ok())
            .ok_or_else(|| HttpError::message("Secure randomness unavailable"))?;
        crypto
            .get_random_values_with_u8_array(&mut bytes)
            .map_err(|_| HttpError::message("Secure token generation failed"))?;
        Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Err(HttpError::message("Secure browser randomness required"))
    }
}
fn parse_registration(value: &str) -> Result<AppserviceRegistration, HttpError> {
    if value.len() > 2 * 1024 * 1024 {
        return Err(HttpError::message("Registration file exceeds 2 MiB"));
    }
    if value.trim().is_empty() {
        return Err(HttpError::message(
            "Paste a YAML or JSON registration first",
        ));
    }
    serde_json::from_str(value)
        .or_else(|_| {
            serde_saphyr::from_str(value).map_err(|e| {
                serde_json::Error::io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    e.to_string(),
                ))
            })
        })
        .map_err(|e| HttpError::message(format!("Invalid registration YAML/JSON: {e}")))
}
fn unique_id(base: &str, rows: &[AppserviceSummary]) -> String {
    if !rows.iter().any(|r| r.id == base) {
        return base.into();
    }
    for n in 2.. {
        let candidate = format!("{base}-{n}");
        if !rows.iter().any(|r| r.id == candidate) {
            return candidate;
        }
    }
    unreachable!()
}
fn namespaces(t: Template) -> serde_json::Value {
    let users = if t.user_prefix.is_empty() {
        vec![]
    } else {
        vec![serde_json::json!({"exclusive":true,"regex":format!("@{}.*",t.user_prefix)})]
    };
    let aliases = if t.alias_prefix.is_empty() {
        vec![]
    } else {
        vec![serde_json::json!({"exclusive":true,"regex":format!("#{}.*",t.alias_prefix)})]
    };
    serde_json::json!({"users":users,"aliases":aliases,"rooms":[]})
}

pub fn render(scope: &Scope, _route: &crate::router::Route) -> View {
    let rows = scope.cage(Vec::<AppserviceSummary>::new());
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let (id, id_input) = field(scope, "Appservice ID");
    let (url, url_input) = field(scope, "Appservice URL");
    let (sender, sender_input) = field(scope, "Sender localpart");
    let (as_token, as_input) = field(scope, "Application service token");
    let (hs_token, hs_input) = field(scope, "Homeserver token");
    let (ns, ns_input) = field(scope, "Namespaces (users, aliases, rooms)");
    ns.revise(|v| *v = "{\"users\":[],\"aliases\":[],\"rooms\":[]}".into());
    let (protocols, protocols_input) = field(scope, "Protocols (comma separated)");
    let (rate, rate_input) = field(scope, "Rate limited (blank, true or false)");
    let original_protocols = scope.cage(None::<Vec<String>>);
    let ephemeral = scope.cage(false);
    let device = scope.cage(false);
    let disabled = scope.cage(false);
    let reveal_tokens = scope.cage(false);
    let editing = scope.cage(None::<String>);
    let expected_url = scope.cage(None::<String>);
    let notes = scope.cage(String::new());
    let form = Form {
        id,
        url,
        sender,
        as_token,
        hs_token,
        namespaces: ns,
        protocols,
        original_protocols,
        rate,
        ephemeral,
        device,
        disabled,
    };
    let s = scope.clone();
    let reload: Rc<dyn Fn()> = Rc::new(move || {
        load_quiet(&s, busy, message, async move {
            let list = api::list_appservices().await?;
            let n = list.len();
            rows.revise(|v| *v = list);
            Ok(if n == 0 {
                "No installed appservices".into()
            } else {
                format!("{n} appservices")
            })
        })
    });
    reload();
    let s = scope.clone();
    let editor_open = scope.cage(false);
    let installed = each(
        rows,
        |r| r.id.clone(),
        move |entry, _| {
            let read_scope = s.clone();
            let switch_scope = s.clone();
            let delete_scope = s.clone();
            let export_scope = s.clone();
            view().class("record").fill(text().content(entry.map(|r|format!("{} · {} · Sender {} · {}",r.id,r.url.clone().unwrap_or_else(||"no URL".into()),r.sender_localpart,if r.disabled{"Disabled"}else{"Enabled"}))))
            .fill(action("Edit registration",move ||{let id=entry.get().id;submit(&read_scope,busy,message,async move{let reg=api::get_appservice(&id).await?;expected_url.revise(|v|*v=reg.url.clone());editor_open.revise(|v|*v=true);form.apply(reg);editing.revise(|v|*v=Some(id));notes.revise(|v|*v="Editing an installed registration: only the callback URL can be updated in place. Tokens, sender, namespaces, protocols and flags are read-only. Use the separately confirmed Delete action and a new registration to replace those fields.".into());Ok("Registration loaded into editor".into())});}))
            .fill(action("Enable / Disable",move ||{let row=entry.get();if !confirm(&format!("{} appservice {}?",if row.disabled{"Enable"}else{"Disable"},row.id)){return;}submit(&switch_scope,busy,message,async move{if row.disabled{api::enable_appservice(&row.id).await?;}else{api::disable_appservice(&row.id).await?;}rows.revise(|items|{if let Some(item)=items.iter_mut().find(|v|v.id==row.id){item.disabled=!row.disabled;}});if editing.get().as_deref()==Some(row.id.as_str()){disabled.revise(|v|*v=!row.disabled);}Ok("Appservice status updated".into())});}))
            .fill(action("Delete appservice",move ||{let id=entry.get().id;if !confirm(&format!("Permanently delete appservice {id}?")){return;}submit(&delete_scope,busy,message,async move{api::delete_appservice(&id).await?;rows.revise(|items|items.retain(|v|v.id!=id));Ok("Appservice deleted".into())});}))
            .fill(action("Export registration YAML",move ||{let id=entry.get().id;submit(&export_scope,busy,message,async move{let reg=api::get_appservice(&id).await?;let yaml=serde_saphyr::to_string(&reg).map_err(|e|HttpError::message(e.to_string()))?;download_text("registration.yaml",&yaml,"application/yaml")?;Ok("Registration downloaded".into())});}))
        },
    );
    let (search, search_input) = field(scope, "Search appservice templates");
    let category = scope.cage(String::new());
    let templates = scope.cage(TEMPLATES.to_vec());
    let filtered = templates.map(move |items| {
        let search = search.get().to_lowercase();
        let category = category.get();
        items
            .iter()
            .copied()
            .filter(|t| {
                (category.is_empty() || category == t.category)
                    && (t.name.to_lowercase().contains(&search)
                        || t.description.to_lowercase().contains(&search))
            })
            .collect::<Vec<_>>()
    });
    let gallery = each(
        filtered,
        |t| t.key,
        move |entry, _| {
            view()
                .class("record template-card")
                .fill((
                    text()
                        .class("template-icon")
                        .content(entry.map(|t| t._icon.to_owned())),
                    text()
                        .class("template-category")
                        .content(entry.map(|t| t.category.to_owned())),
                    text()
                        .class("template-title")
                        .content(entry.map(|t| t.name.to_owned())),
                    text()
                        .class("template-description")
                        .content(entry.map(|t| t.description.to_owned())),
                ))
                .fill(action("Configure template", move || {
                    let t = entry.get();
                    let id = rows.with(|rows| unique_id(t.default_id_prefix, rows));
                    editor_open.revise(|v| *v = true);
                    form.apply(AppserviceRegistration {
                        id,
                        url: if t.default_url.is_empty() {
                            None
                        } else {
                            Some(t.default_url.into())
                        },
                        sender_localpart: t.default_sender_localpart.into(),
                        namespaces: namespaces(t),
                        ..Default::default()
                    });
                    editing.revise(|v| *v = None);
                    notes.revise(|v| *v = t.notes.into());
                }))
        },
    );
    let (import, import_input) = field(scope, "Paste registration YAML or JSON");
    let file_scope = scope.clone();
    let save_scope = scope.clone();
    let copy_scope = scope.clone();
    let copy_scope2 = scope.clone();
    let yaml_copy_scope = scope.clone();
    page("Appservices").class("page appservices-page")
        .fill(label("Install and manage Matrix application services at runtime.").class("page-description"))
        .fill(view().class("page-actions").fill(action("Refresh / Retry",move ||reload())))
        .fill(status(message)).fill(label("Installed").kind(glory::render::TextKind::Heading(2)))
        .fill(view().class("installed-table table-container").fill((view().class("installed-head").fill((label("ID"),label("Sender"),label("URL"),label("Status"),label("Actions"))),installed,when(rows.map(|rows|rows.is_empty()),||view().class("empty-table").fill((label("").class("empty-icon plug-icon"),label("No appservices installed").class("empty-title"),label("Pick a template from the gallery below to install a bridge or bot.").class("empty-description")))))))
        .fill(view().class("gallery-heading").fill((label("Install a new appservice").kind(glory::render::TextKind::Heading(2)),label("Pick a ready-made bridge or bot — the form pre-fills sensible defaults.").class("page-description"),action("Import registration.yaml",move||editor_open.revise(|v|*v=true)),search_input)))
        .fill(view().class("template-categories").fill((action("All categories",move ||category.revise(|v|v.clear())),action("Bridges",move ||category.revise(|v|*v="Bridge".into())),action("Bots",move ||category.revise(|v|*v="Bot".into())),action("Custom",move ||category.revise(|v|*v="Custom".into())))))
        .fill(view().class("template-gallery").fill(gallery))
        .fill(crate::ui::widgets::modal("appservice-editor","Registration editor",editor_open,view().class("registration-editor operation-form").fill(action("Close registration editor",move||editor_open.revise(|v|*v=false)))
        .fill(status(notes)).fill(id_input.enabled(editing.map(|v|v.is_none()))).fill(url_input).fill(sender_input.enabled(editing.map(|v|v.is_none())))
        .fill(as_input.purpose(reveal_tokens.map(|show|if *show {Purpose::Text}else{Purpose::Password})).enabled(editing.map(|v|v.is_none()))).fill(hs_input.purpose(reveal_tokens.map(|show|if *show {Purpose::Text}else{Purpose::Password})).enabled(editing.map(|v|v.is_none())))
        .fill(check().name(localized(scope,"Show registration tokens".to_owned())).checked(reveal_tokens).on(checked_changed,move|event|{if let Event::CheckedChanged(show)=event{reveal_tokens.revise(|v|*v=show);}})).fill(label("Show registration tokens"))
        .fill(action("Generate secure tokens",move ||{if editing.get().is_some(){message.revise(|v|*v="Tokens cannot be changed in place. Delete and register explicitly to rotate credentials.".into());return;}match (random_token(),random_token()){(Ok(a),Ok(h))=>{as_token.revise(|v|*v=a);hs_token.revise(|v|*v=h);message.revise(|v|*v="Secure tokens generated; save and configure the bridge with matching tokens".into());},(Err(e),_)|(_,Err(e))=>message.revise(|v|*v=e.to_string())}}))
        .fill(action("Copy AS token",move ||{let value=as_token.get();submit(&copy_scope,busy,message,async move{copy(&value).await?;Ok("AS token copied".into())});}))
        .fill(action("Copy HS token",move ||{let value=hs_token.get();submit(&copy_scope2,busy,message,async move{copy(&value).await?;Ok("HS token copied".into())});}))
        .fill(ns_input.multiline(true).enabled(editing.map(|v|v.is_none()))).fill(protocols_input.enabled(editing.map(|v|v.is_none()))).fill(rate_input.enabled(editing.map(|v|v.is_none())))
        .fill(check().name(localized(scope, "Receive ephemeral events".to_owned())).checked(ephemeral).enabled(editing.map(|v|v.is_none())).on(checked_changed,move |event|{if let Event::CheckedChanged(next)=event{ephemeral.revise(|v|*v=next);}})).fill(label("Receive ephemeral events"))
        .fill(check().name(localized(scope, "Device management (MSC4190)".to_owned())).checked(device).enabled(editing.map(|v|v.is_none())).on(checked_changed,move |event|{if let Event::CheckedChanged(next)=event{device.revise(|v|*v=next);}})).fill(label("Device management (MSC4190)"))
        .fill(check().name(localized(scope, "Disabled registration".to_owned())).checked(disabled).enabled(editing.map(|v|v.is_none())).on(checked_changed,move |event|{if let Event::CheckedChanged(next)=event{disabled.revise(|v|*v=next);}})).fill(label("Disabled registration"))
        .fill(action("Save registration",move ||{let registration=form.registration();let target=editing.get();let observed_url=expected_url.get();submit(&save_scope,busy,message,async move{let reg=registration?;if let Some(target)=target{if target!=reg.id{return Err(HttpError::message("An installed appservice ID cannot be changed in place"));}let url=reg.url.as_deref().ok_or_else(||HttpError::message("An installed appservice callback URL cannot be cleared"))?;api::update_appservice_url(&target,url,observed_url.as_deref()).await?;}else{api::register_appservice(&reg).await?;if reg.disabled{api::disable_appservice(&reg.id).await?;}}let list=api::list_appservices().await?;rows.revise(|v|*v=list);expected_url.revise(|v|*v=reg.url.clone());editing.revise(|v|*v=Some(reg.id));Ok("Appservice registration saved".into())});}))
        .fill(action("Export editor YAML",move ||{let result=form.registration().and_then(|reg|serde_saphyr::to_string(&reg).map_err(|e|HttpError::message(e.to_string()))).and_then(|yaml|download_text("registration.yaml",&yaml,"application/yaml"));message.revise(|v|*v=match result{Ok(())=>"Registration downloaded".into(),Err(e)=>e.to_string()});}))
        .fill(action("Copy editor YAML", move || {
            let registration = form.registration();
            submit(&yaml_copy_scope, busy, message, async move {
                let yaml = serde_saphyr::to_string(&registration?).map_err(|e| HttpError::message(e.to_string()))?;
                copy(&yaml).await?;
                Ok("Registration YAML copied".into())
            });
        }))
        .fill(label("Import registration"))
        .fill(action("Choose registration file",move ||{
            if busy.get() { return; }
            // Open while handling activation, before the host polls the owned task.
            let selected = choose_registration_file();
            submit(&file_scope,busy,message,async move {
                match selected.await? {
                    Some(contents) => {
                        let reg = parse_registration(&contents)?;
                        editor_open.revise(|v|*v=true);form.apply(reg);
                        editing.revise(|v| *v = None);
                        Ok("Imported. Review all fields and save to register the appservice.".into())
                    }
                    None => Ok("File selection cancelled".into()),
                }
            });
        }))
        .fill(import_input.multiline(true))
        .fill(action("Import YAML / JSON into editor",move ||{match parse_registration(&import.get()){Ok(reg)=>{editor_open.revise(|v|*v=true);form.apply(reg);editing.revise(|v|*v=None);import.revise(|v|v.clear());message.revise(|v|*v="Imported. Review all fields and save to register the appservice.".into());},Err(e)=>message.revise(|v|*v=e.to_string())}}))))
}

/// Browser file picker is a host capability; it never mutates a semantic node.
/// JS listeners resolve data only. The page-owned task applies it on the host turn.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
export function padminRegistrationFile() {
    return new Promise((resolve, reject) => {
        const input = document.createElement('input');
        input.type = 'file';
        input.accept = '.yaml,.yml,.json,application/yaml,application/json,text/yaml';
        input.addEventListener('cancel', () => resolve(null), {once: true});
        input.addEventListener('change', async () => {
            const file = input.files && input.files[0];
            if (!file) { resolve(null); return; }
            if (file.size > 2 * 1024 * 1024) {
                reject(new Error('Registration file exceeds 2 MiB')); return;
            }
            try { resolve(await file.text()); } catch (error) { reject(error); }
        }, {once: true});
        input.click();
    });
}
"#)]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = padminRegistrationFile)]
    fn registration_file_promise() -> js_sys::Promise;
}
fn choose_registration_file() -> impl std::future::Future<Output = Result<Option<String>, HttpError>>
{
    #[cfg(target_arch = "wasm32")]
    let promise = registration_file_promise();
    async move {
        #[cfg(target_arch = "wasm32")]
        {
            let result = wasm_bindgen_futures::JsFuture::from(promise)
                .await
                .map_err(|error| {
                    HttpError::message(format!("Could not read registration file: {error:?}"))
                })?;
            if result.is_null() || result.is_undefined() {
                Ok(None)
            } else {
                result
                    .as_string()
                    .map(Some)
                    .ok_or_else(|| HttpError::message("Registration file is not text"))
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            Err(HttpError::message("Browser file picker required"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bridge_yaml_roundtrip_preserves_wire_flags_and_namespaces() {
        let yaml = "id: test-bridge\nurl: https://bridge.invalid\nsender_localpart: testbot\nas_token: fixture-as-token\nhs_token: fixture-hs-token\nrate_limited: false\nreceive_ephemeral: true\nio.element.msc4190: true\nprotocols: [test]\nnamespaces:\n  users:\n    - exclusive: true\n      regex: '@test_.*'\n  aliases: []\n  rooms: []\n";
        let registration = parse_registration(yaml).unwrap();
        assert_eq!(registration.rate_limited, Some(false));
        assert!(registration.receive_ephemeral && registration.device_management);
        assert_eq!(registration.namespaces["users"][0]["regex"], "@test_.*");
        let exported = serde_saphyr::to_string(&registration).unwrap();
        let restored = parse_registration(&exported).unwrap();
        assert_eq!(
            serde_json::to_value(registration).unwrap(),
            serde_json::to_value(restored).unwrap()
        );
    }
    #[test]
    fn invalid_import_is_reported_and_duplicate_template_ids_are_avoided() {
        assert!(parse_registration("").is_err());
        assert!(parse_registration("namespaces: [").is_err());
        let installed = vec![
            AppserviceSummary {
                id: "slack".into(),
                ..Default::default()
            },
            AppserviceSummary {
                id: "slack-2".into(),
                ..Default::default()
            },
        ];
        assert_eq!(unique_id("slack", &installed), "slack-3");
    }
    #[test]
    fn every_template_has_complete_namespace_sections() {
        for template in TEMPLATES {
            let value = namespaces(*template);
            for section in ["users", "aliases", "rooms"] {
                assert!(value[section].is_array(), "{}: {section}", template.key);
            }
        }
    }
}
