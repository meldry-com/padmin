//! Store-style UI for managing Matrix appservices.
//!
//! The page is a single scrollable view: installed appservices at the top
//! and a template gallery (with a filter search) below. Picking a
//! template opens a configure modal pre-filled from the template
//! defaults — the user only needs to supply the bridge tokens and
//! optionally tweak the ID. A template can be added multiple times;
//! each instance is an independent appservice registration.
//!
//! Backed by the palpo admin endpoints under `/_palpo/admin/v1/appservices`.

use dioxus::prelude::*;

use crate::api::appservices;
use crate::components::ui::badge::{Badge, BadgeVariant};
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::dialog::ConfirmDialog;
use crate::components::ui::empty_state::EmptyState;
use crate::components::ui::icons::Icon;
use crate::components::ui::input::{Input, Label, SearchInput};
use crate::components::ui::loading::{PageSkeleton, Spinner};
use crate::components::ui::page_header::PageHeader;
use crate::components::ui::table::*;
use crate::components::ui::toast::{ToastVariant, show_toast};
use crate::types::{AppserviceRegistration, AppserviceSummary};

// ── Templates ────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum TemplateCategory {
    Bridge,
    Bot,
    Custom,
}

impl TemplateCategory {
    fn label(self) -> &'static str {
        match self {
            Self::Bridge => "Bridge",
            Self::Bot => "Bot",
            Self::Custom => "Custom",
        }
    }

    fn badge_variant(self) -> BadgeVariant {
        match self {
            Self::Bridge => BadgeVariant::Default,
            Self::Bot => BadgeVariant::Secondary,
            Self::Custom => BadgeVariant::Outline,
        }
    }
}

/// Static description of a ready-to-install appservice.
struct Template {
    key: &'static str,
    name: &'static str,
    description: &'static str,
    icon: &'static str,
    /// Tailwind classes used on the icon chip (bg + text color).
    accent: &'static str,
    category: TemplateCategory,
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
        icon: "hash",
        accent: "bg-purple-500/10 text-purple-500",
        category: TemplateCategory::Bridge,
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
        icon: "send",
        accent: "bg-sky-500/10 text-sky-500",
        category: TemplateCategory::Bridge,
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
        icon: "gamepad-2",
        accent: "bg-indigo-500/10 text-indigo-500",
        category: TemplateCategory::Bridge,
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
        icon: "message-circle",
        accent: "bg-green-500/10 text-green-500",
        category: TemplateCategory::Bridge,
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
        icon: "message-square",
        accent: "bg-emerald-500/10 text-emerald-500",
        category: TemplateCategory::Bridge,
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
        icon: "briefcase",
        accent: "bg-cyan-500/10 text-cyan-500",
        category: TemplateCategory::Bridge,
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
        icon: "bell",
        accent: "bg-blue-500/10 text-blue-500",
        category: TemplateCategory::Bridge,
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
        icon: "users",
        accent: "bg-yellow-500/10 text-yellow-600",
        category: TemplateCategory::Bridge,
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
        icon: "shield",
        accent: "bg-blue-600/10 text-blue-600",
        category: TemplateCategory::Bridge,
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
        icon: "terminal",
        accent: "bg-orange-500/10 text-orange-500",
        category: TemplateCategory::Bridge,
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
        icon: "webhook",
        accent: "bg-fuchsia-500/10 text-fuchsia-500",
        category: TemplateCategory::Bot,
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
        icon: "wrench",
        accent: "bg-muted text-muted-foreground",
        category: TemplateCategory::Custom,
        default_id_prefix: "appservice",
        default_sender_localpart: "",
        default_url: "",
        user_prefix: "",
        alias_prefix: "",
        notes: "",
    },
];

fn find_template(key: &str) -> Option<&'static Template> {
    TEMPLATES.iter().find(|t| t.key == key)
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Generate a random 32-byte token encoded as lowercase hex.
fn gen_token() -> String {
    let mut buf = [0u8; 32];
    if let Some(crypto) = web_sys::window().and_then(|w| w.crypto().ok()) {
        let _ = crypto.get_random_values_with_u8_array(&mut buf);
    }
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

/// Pick a unique appservice id starting from `base` and appending `-N` if taken.
fn unique_id(base: &str, existing: &[String]) -> String {
    if base.is_empty() {
        return String::from("appservice");
    }
    if !existing.iter().any(|e| e == base) {
        return base.to_string();
    }
    for n in 2..1000 {
        let candidate = format!("{base}-{n}");
        if !existing.iter().any(|e| e == &candidate) {
            return candidate;
        }
    }
    base.to_string()
}

/// Default namespaces JSON for a bridge template. Uses exclusive regexes so
/// the bridge owns `@<prefix>.*` / `#<prefix>.*`.
fn default_namespaces(template: &Template) -> String {
    if template.key == "custom" {
        return "{\n  \"users\": [],\n  \"aliases\": [],\n  \"rooms\": []\n}".to_string();
    }

    let users = if template.user_prefix.is_empty() {
        "[]".to_string()
    } else {
        format!(
            "[\n    {{ \"exclusive\": true, \"regex\": \"@{}.*\" }}\n  ]",
            template.user_prefix
        )
    };
    let aliases = if template.alias_prefix.is_empty() {
        "[]".to_string()
    } else {
        format!(
            "[\n    {{ \"exclusive\": true, \"regex\": \"#{}.*\" }}\n  ]",
            template.alias_prefix
        )
    };
    format!("{{\n  \"users\": {users},\n  \"aliases\": {aliases},\n  \"rooms\": []\n}}")
}

// ── Configure form state ─────────────────────────────────────────────────────

#[derive(Default, Clone, PartialEq)]
struct ConfigForm {
    template_key: String,
    id: String,
    url: String,
    sender_localpart: String,
    as_token: String,
    hs_token: String,
    namespaces: String,
}

impl ConfigForm {
    fn from_template(template: &Template, existing_ids: &[String]) -> Self {
        let id = unique_id(template.default_id_prefix, existing_ids);
        Self {
            template_key: template.key.to_string(),
            id,
            url: template.default_url.to_string(),
            sender_localpart: template.default_sender_localpart.to_string(),
            as_token: String::new(),
            hs_token: String::new(),
            namespaces: default_namespaces(template),
        }
    }
}

// ── Page ─────────────────────────────────────────────────────────────────────

#[component]
pub fn AppserviceList() -> Element {
    let mut data = use_resource(|| async { appservices::list_appservices().await });

    // Configure modal state
    let mut configure_open = use_signal(|| false);
    let mut form = use_signal(ConfigForm::default);
    let mut search = use_signal(String::new);
    let mut creating = use_signal(|| false);

    // Delete confirmation
    let mut delete_open = use_signal(|| false);
    let mut delete_target = use_signal(|| Option::<String>::None);

    // Enable/disable in-flight tracking
    let mut toggling = use_signal(|| Option::<String>::None);

    let existing_ids = move || -> Vec<String> {
        match &*data.read() {
            Some(Ok(items)) => items.iter().map(|i| i.id.clone()).collect(),
            _ => Vec::new(),
        }
    };

    // Pick a template — seed the form with its defaults and open the
    // configure modal.
    let mut pick_template = move |key: &'static str| {
        if let Some(t) = find_template(key) {
            form.set(ConfigForm::from_template(t, &existing_ids()));
            configure_open.set(true);
        }
    };

    let close_dialog = move |_| {
        if !*creating.read() {
            configure_open.set(false);
            form.set(ConfigForm::default());
        }
    };

    let handle_create = move |_| {
        let f = form.read().clone();
        if f.id.trim().is_empty()
            || f.sender_localpart.trim().is_empty()
            || f.as_token.trim().is_empty()
            || f.hs_token.trim().is_empty()
        {
            show_toast(
                "id, sender_localpart, as_token and hs_token are required",
                ToastVariant::Error,
            );
            return;
        }
        let namespaces: serde_json::Value = match serde_json::from_str(&f.namespaces) {
            Ok(v) => v,
            Err(e) => {
                show_toast(
                    &format!("namespaces must be valid JSON: {e}"),
                    ToastVariant::Error,
                );
                return;
            }
        };
        let reg = AppserviceRegistration {
            id: f.id.trim().to_string(),
            url: if f.url.trim().is_empty() {
                None
            } else {
                Some(f.url.trim().to_string())
            },
            as_token: f.as_token.trim().to_string(),
            hs_token: f.hs_token.trim().to_string(),
            sender_localpart: f.sender_localpart.trim().to_string(),
            namespaces,
            rate_limited: None,
            protocols: None,
            receive_ephemeral: false,
            device_management: false,
            disabled: false,
        };
        creating.set(true);
        spawn(async move {
            match appservices::register_appservice(&reg).await {
                Ok(_) => {
                    show_toast("Appservice registered", ToastVariant::Success);
                    configure_open.set(false);
                    form.set(ConfigForm::default());
                    data.restart();
                }
                Err(e) => show_toast(&format!("Failed: {}", e.message), ToastVariant::Error),
            }
            creating.set(false);
        });
    };

    let handle_delete_confirm = move |_| {
        if let Some(id) = delete_target.read().clone() {
            spawn(async move {
                match appservices::delete_appservice(&id).await {
                    Ok(_) => {
                        show_toast("Appservice deleted", ToastVariant::Success);
                        delete_open.set(false);
                        delete_target.set(None);
                        data.restart();
                    }
                    Err(e) => show_toast(&format!("Failed: {}", e.message), ToastVariant::Error),
                }
            });
        }
    };

    let is_creating = *creating.read();
    let is_configure_open = *configure_open.read();

    // Filter templates by the inline search box.
    let query = search.read().to_lowercase();
    let filtered: Vec<&'static Template> = TEMPLATES
        .iter()
        .filter(|t| {
            if query.is_empty() {
                return true;
            }
            t.name.to_lowercase().contains(&query)
                || t.description.to_lowercase().contains(&query)
                || t.key.contains(query.as_str())
        })
        .collect();

    rsx! {
        div { class: "space-y-8",
            PageHeader {
                title: "Appservices".to_string(),
                description: "Install and manage Matrix application services at runtime.".to_string(),
                Button {
                    variant: ButtonVariant::Outline,
                    onclick: move |_| data.restart(),
                    "Refresh"
                }
            }

            // ── Installed appservices ────────────────────────────────
            section { class: "space-y-3",
                h2 { class: "text-base font-semibold", "Installed" }
                match &*data.read() {
                    Some(Ok(items)) => rsx! {
                        AppserviceTable {
                            items: items.clone(),
                            toggling: toggling,
                            on_toggle: move |(id, currently_disabled): (String, bool)| {
                                toggling.set(Some(id.clone()));
                                spawn(async move {
                                    let res = if currently_disabled {
                                        appservices::enable_appservice(&id).await
                                    } else {
                                        appservices::disable_appservice(&id).await
                                    };
                                    match res {
                                        Ok(_) => {
                                            show_toast(
                                                if currently_disabled { "Appservice enabled" } else { "Appservice disabled" },
                                                ToastVariant::Success,
                                            );
                                            data.restart();
                                        }
                                        Err(e) => show_toast(&format!("Failed: {}", e.message), ToastVariant::Error),
                                    }
                                    toggling.set(None);
                                });
                            },
                            on_delete: move |id: String| {
                                delete_target.set(Some(id));
                                delete_open.set(true);
                            },
                        }
                    },
                    Some(Err(e)) => rsx! {
                        div { class: "rounded-md bg-destructive/10 p-4",
                            div { class: "flex items-center justify-between",
                                p { class: "text-sm text-destructive", "Error: {e.message}" }
                                button {
                                    class: "text-sm font-medium text-primary hover:underline",
                                    onclick: move |_| data.restart(),
                                    "Retry"
                                }
                            }
                        }
                    },
                    None => rsx! { PageSkeleton {} },
                }
            }

            // ── Inline template gallery ──────────────────────────────
            section { class: "space-y-4",
                div { class: "flex flex-col gap-2 sm:flex-row sm:items-end sm:justify-between",
                    div { class: "space-y-1",
                        h2 { class: "text-base font-semibold", "Install a new appservice" }
                        p { class: "text-sm text-muted-foreground",
                            "Pick a ready-made bridge or bot — the form pre-fills sensible defaults."
                        }
                    }
                    div { class: "sm:w-72",
                        SearchInput {
                            placeholder: "Filter: Slack, Telegram, WeChat …".to_string(),
                            value: search.read().clone(),
                            oninput: move |e: FormEvent| search.set(e.value()),
                        }
                    }
                }

                if filtered.is_empty() {
                    div { class: "rounded-md border border-dashed p-8 text-center text-sm text-muted-foreground",
                        "No templates match your search."
                    }
                } else {
                    div { class: "grid gap-4 sm:grid-cols-2 lg:grid-cols-3",
                        for template in filtered.iter() {
                            {
                                let key = template.key;
                                rsx! {
                                    TemplateCard {
                                        key: "{key}",
                                        template_key: key,
                                        name: template.name,
                                        description: template.description,
                                        icon: template.icon,
                                        accent: template.accent,
                                        category: template.category,
                                        on_pick: move |k: &'static str| pick_template(k),
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Configure view — template-specific form (kept as a modal because
        // it's a multi-field form with token generators).
        if is_configure_open {
            ConfigureDialog {
                form: form,
                creating: is_creating,
                on_close: close_dialog,
                on_submit: handle_create,
            }
        }

        ConfirmDialog {
            open: *delete_open.read(),
            title: "Delete appservice".to_string(),
            description: format!(
                "Permanently unregister appservice '{}'? Its tokens will stop working immediately.",
                delete_target.read().clone().unwrap_or_default()
            ),
            confirm_text: "Delete".to_string(),
            destructive: true,
            on_confirm: handle_delete_confirm,
            on_cancel: move |_| { delete_open.set(false); delete_target.set(None); },
        }
    }
}

// ── Sub-components ───────────────────────────────────────────────────────────

#[component]
fn AppserviceTable(
    items: Vec<AppserviceSummary>,
    toggling: Signal<Option<String>>,
    on_toggle: EventHandler<(String, bool)>,
    on_delete: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "rounded-md border",
            Table {
                TableHeader {
                    TableRow {
                        TableHead { "ID" }
                        TableHead { "Sender" }
                        TableHead { "URL" }
                        TableHead { "Status" }
                        TableHead { class: "text-right".to_string(), "Actions" }
                    }
                }
                TableBody {
                    if items.is_empty() {
                        TableRow {
                            TableCell { class: "p-0".to_string(), colspan: 99,
                                EmptyState {
                                    icon: "plug".to_string(),
                                    title: "No appservices installed".to_string(),
                                    description: "Pick a template from the gallery below to install a bridge or bot.".to_string(),
                                }
                            }
                        }
                    } else {
                        for item in items.iter() {
                            {
                                let id = item.id.clone();
                                let id_toggle = id.clone();
                                let id_delete = id.clone();
                                let sender = item.sender_localpart.clone();
                                let url = item.url.clone().unwrap_or_else(|| "-".to_string());
                                let disabled = item.disabled;
                                let busy = toggling.read().as_deref() == Some(id.as_str());
                                rsx! {
                                    TableRow { key: "{id}",
                                        TableCell {
                                            code { class: "text-sm font-mono", "{id}" }
                                        }
                                        TableCell { "{sender}" }
                                        TableCell {
                                            span { class: "text-xs font-mono text-muted-foreground", "{url}" }
                                        }
                                        TableCell {
                                            if disabled {
                                                Badge { variant: BadgeVariant::Secondary, "Disabled" }
                                            } else {
                                                Badge { variant: BadgeVariant::Success, "Enabled" }
                                            }
                                        }
                                        TableCell { class: "text-right".to_string(),
                                            Button {
                                                variant: ButtonVariant::Ghost,
                                                size: ButtonSize::Sm,
                                                disabled: busy,
                                                onclick: move |_| on_toggle.call((id_toggle.clone(), disabled)),
                                                if disabled { "Enable" } else { "Disable" }
                                            }
                                            Button {
                                                variant: ButtonVariant::Ghost,
                                                size: ButtonSize::Sm,
                                                class: "text-destructive hover:text-destructive".to_string(),
                                                onclick: move |_| on_delete.call(id_delete.clone()),
                                                "Delete"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn TemplateCard(
    template_key: &'static str,
    name: &'static str,
    description: &'static str,
    icon: &'static str,
    accent: &'static str,
    category: TemplateCategory,
    on_pick: EventHandler<&'static str>,
) -> Element {
    rsx! {
        button {
            class: "group relative flex flex-col items-start gap-3 rounded-lg border bg-card p-4 text-left transition-all hover:border-primary hover:shadow-md",
            onclick: move |_| on_pick.call(template_key),
            div { class: "flex w-full items-start justify-between",
                div { class: "flex h-10 w-10 items-center justify-center rounded-lg {accent}",
                    Icon { name: icon.to_string(), class: "h-5 w-5".to_string() }
                }
                Badge { variant: category.badge_variant(),
                    "{category.label()}"
                }
            }
            div { class: "space-y-1",
                h3 { class: "font-semibold leading-tight", "{name}" }
                p { class: "text-xs text-muted-foreground leading-snug line-clamp-3",
                    "{description}"
                }
            }
            span { class: "mt-auto text-xs font-medium text-primary opacity-0 transition-opacity group-hover:opacity-100",
                "Install →"
            }
        }
    }
}

#[component]
fn ConfigureDialog(
    form: Signal<ConfigForm>,
    creating: bool,
    on_close: EventHandler<MouseEvent>,
    on_submit: EventHandler<MouseEvent>,
) -> Element {
    let template_key = form.read().template_key.clone();
    let template = find_template(&template_key);
    let name = template.map(|t| t.name).unwrap_or("Custom Appservice");
    let description = template.map(|t| t.description).unwrap_or("");
    let icon = template.map(|t| t.icon).unwrap_or("wrench");
    let accent = template
        .map(|t| t.accent)
        .unwrap_or("bg-muted text-muted-foreground");
    let notes = template.map(|t| t.notes).unwrap_or("");

    rsx! {
        div { class: "fixed inset-0 z-50 flex items-start justify-center overflow-y-auto p-4 sm:items-center",
            div {
                class: "fixed inset-0 bg-black/80",
                onclick: move |e| on_close.call(e),
            }
            div { class: "relative z-50 my-4 flex min-h-0 w-full max-w-2xl flex-col overflow-hidden rounded-lg border bg-background shadow-lg max-h-[calc(100vh-2rem)]",
                // Header with template brand
                div { class: "flex items-start gap-4 border-b p-6",
                    div { class: "flex h-10 w-10 items-center justify-center rounded-lg {accent} shrink-0",
                        Icon { name: icon.to_string(), class: "h-5 w-5".to_string() }
                    }
                    div { class: "flex-1 min-w-0 space-y-1",
                        h2 { class: "text-lg font-semibold leading-tight", "Install {name}" }
                        p { class: "text-sm text-muted-foreground", "{description}" }
                    }
                    button {
                        class: "rounded-md p-2 hover:bg-accent shrink-0",
                        disabled: creating,
                        onclick: move |e| on_close.call(e),
                        Icon { name: "x".to_string(), class: "h-4 w-4".to_string() }
                    }
                }

                // Body — form fields
                div { class: "min-h-0 flex-1 overflow-y-auto p-6 space-y-4",
                    if !notes.is_empty() {
                        div { class: "rounded-md border bg-muted/30 p-3 text-xs text-muted-foreground flex gap-2",
                            Icon { name: "info".to_string(), class: "h-4 w-4 shrink-0 mt-0.5".to_string() }
                            span { "{notes}" }
                        }
                    }

                    div { class: "grid gap-4 sm:grid-cols-2",
                        div { class: "space-y-1",
                            Label { "Instance ID" }
                            Input {
                                placeholder: "wechat-bridge".to_string(),
                                value: form.read().id.clone(),
                                oninput: move |e: FormEvent| form.write().id = e.value(),
                                disabled: creating,
                            }
                            p { class: "text-xs text-muted-foreground",
                                "Same template can be added multiple times — the ID must be unique."
                            }
                        }
                        div { class: "space-y-1",
                            Label { "Sender localpart" }
                            Input {
                                placeholder: "bot".to_string(),
                                value: form.read().sender_localpart.clone(),
                                oninput: move |e: FormEvent| form.write().sender_localpart = e.value(),
                                disabled: creating,
                            }
                        }
                    }

                    div { class: "space-y-1",
                        Label { "Bridge URL" }
                        Input {
                            placeholder: "http://localhost:29335".to_string(),
                            value: form.read().url.clone(),
                            oninput: move |e: FormEvent| form.write().url = e.value(),
                            disabled: creating,
                        }
                        p { class: "text-xs text-muted-foreground",
                            "Where the homeserver should POST transactions. Leave empty for push-only bots."
                        }
                    }

                    TokenField {
                        label: "as_token",
                        help: "Token the bridge uses to authenticate to the homeserver.",
                        value: form.read().as_token.clone(),
                        disabled: creating,
                        on_input: move |v: String| form.write().as_token = v,
                    }
                    TokenField {
                        label: "hs_token",
                        help: "Token the homeserver sends with transactions so the bridge can verify them.",
                        value: form.read().hs_token.clone(),
                        disabled: creating,
                        on_input: move |v: String| form.write().hs_token = v,
                    }

                    div { class: "space-y-1",
                        Label { "Namespaces (JSON)" }
                        textarea {
                            class: "flex min-h-[160px] w-full rounded-md border border-input bg-background px-3 py-2 text-xs font-mono focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                            value: form.read().namespaces.clone(),
                            oninput: move |e: FormEvent| form.write().namespaces = e.value(),
                            disabled: creating,
                        }
                        p { class: "text-xs text-muted-foreground",
                            "Each namespace entry is {{ exclusive: bool, regex: string }}. The template pre-fills a sensible default."
                        }
                    }
                }

                // Footer
                div { class: "flex flex-col-reverse sm:flex-row sm:justify-end sm:space-x-2 border-t p-6",
                    Button {
                        variant: ButtonVariant::Outline,
                        disabled: creating,
                        onclick: move |e| on_close.call(e),
                        "Cancel"
                    }
                    Button {
                        disabled: creating,
                        onclick: move |e| on_submit.call(e),
                        if creating { Spinner { class: "mr-2".to_string() } }
                        "Install"
                    }
                }
            }
        }
    }
}

#[component]
fn TokenField(
    label: &'static str,
    help: &'static str,
    value: String,
    disabled: bool,
    on_input: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "space-y-1",
            div { class: "flex items-center justify-between",
                Label { "{label}" }
                button {
                    class: "text-xs font-medium text-primary hover:underline disabled:opacity-50",
                    disabled: disabled,
                    onclick: move |_| on_input.call(gen_token()),
                    "Generate random"
                }
            }
            Input {
                value: value,
                oninput: move |e: FormEvent| on_input.call(e.value()),
                disabled: disabled,
            }
            p { class: "text-xs text-muted-foreground", "{help}" }
        }
    }
}
