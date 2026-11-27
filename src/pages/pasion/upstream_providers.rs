//! Upstream OAuth provider management.
//!
//! Lists providers and supports admin CRUD: create / update / delete plus
//! enable / disable. Providers loaded from the pasion config file
//! (`source = "config"`) are read-only here — the API refuses to mutate them,
//! so the UI hides the destructive actions for them.
//!
//! Updates are sent as a partial PATCH containing only the fields that
//! changed. Clearing an optional field sends `null`, which pasion treats as
//! "remove this value"; omitted fields keep their stored value.

use dioxus::prelude::*;
use serde_json::{Map, Value};

use crate::api::pasion;
use crate::components::ui::badge::{Badge, BadgeVariant};
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::dialog::{ConfirmDialog, Modal, ModalSize};
use crate::components::ui::error_banner::ErrorBanner;
use crate::components::ui::icons::Icon;
use crate::components::ui::input::{Input, Label, SearchInput};
use crate::components::ui::loading::{PageSkeleton, Spinner};
use crate::components::ui::page_header::PageHeader;
use crate::components::ui::toast::{ToastVariant, show_toast};
use crate::types::PasionUpstreamProvider;
use crate::utils::i18n::{t, t_with};

const SELECT_CLASS: &str =
    "flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm";

const TOKEN_AUTH_METHODS: &[&str] = &[
    "client_secret_basic",
    "client_secret_post",
    "client_secret_jwt",
    "private_key_jwt",
    "none",
    "sign_in_with_apple",
    "qq_connect",
    "feishu",
    "lark",
    "dingtalk",
    "wechat",
    "wecom",
];
const DISCOVERY_MODES: &[&str] = &["oidc", "insecure", "disabled"];
const PKCE_MODES: &[&str] = &["auto", "s256", "disabled"];
/// The empty value means "provider default" and is sent as `null`.
const RESPONSE_MODES: &[&str] = &["", "query", "form_post"];
const BACKCHANNEL_LOGOUT_MODES: &[&str] = &["do_nothing", "logout_browser_only", "logout_all"];

#[derive(Default, Clone, PartialEq)]
struct ProviderForm {
    issuer: String,
    human_name: String,
    brand_name: String,
    client_id: String,
    client_secret: String,
    /// Edit mode only: remove the stored secret (sends `client_secret: null`).
    clear_client_secret: bool,
    scope: String,
    token_endpoint_auth_method: String,
    token_endpoint_signing_alg: String,
    id_token_signed_response_alg: String,
    userinfo_signed_response_alg: String,
    discovery_mode: String,
    pkce_mode: String,
    response_mode: String,
    authorization_endpoint_override: String,
    token_endpoint_override: String,
    userinfo_endpoint_override: String,
    jwks_uri_override: String,
    fetch_userinfo: bool,
    forward_login_hint: bool,
    /// One `key=value` pair per line.
    additional_authorization_parameters: String,
    ui_order: String,
    on_backchannel_logout: String,
    claims_imports: String,
}

impl ProviderForm {
    fn for_create() -> Self {
        Self {
            scope: "openid email profile".into(),
            token_endpoint_auth_method: "client_secret_basic".into(),
            id_token_signed_response_alg: "RS256".into(),
            discovery_mode: "oidc".into(),
            pkce_mode: "auto".into(),
            on_backchannel_logout: "do_nothing".into(),
            ui_order: "0".into(),
            ..Default::default()
        }
    }

    fn from_provider(provider: &PasionUpstreamProvider) -> Self {
        let text = |value: &Option<String>| value.clone().unwrap_or_default();
        Self {
            issuer: text(&provider.issuer),
            human_name: text(&provider.human_name),
            brand_name: text(&provider.brand_name),
            client_id: text(&provider.client_id),
            client_secret: String::new(),
            clear_client_secret: false,
            scope: provider.scope.clone(),
            token_endpoint_auth_method: text(&provider.token_endpoint_auth_method),
            token_endpoint_signing_alg: text(&provider.token_endpoint_signing_alg),
            id_token_signed_response_alg: text(&provider.id_token_signed_response_alg),
            userinfo_signed_response_alg: text(&provider.userinfo_signed_response_alg),
            discovery_mode: text(&provider.discovery_mode),
            pkce_mode: text(&provider.pkce_mode),
            response_mode: text(&provider.response_mode),
            authorization_endpoint_override: text(&provider.authorization_endpoint_override),
            token_endpoint_override: text(&provider.token_endpoint_override),
            userinfo_endpoint_override: text(&provider.userinfo_endpoint_override),
            jwks_uri_override: text(&provider.jwks_uri_override),
            fetch_userinfo: provider.fetch_userinfo,
            forward_login_hint: provider.forward_login_hint,
            additional_authorization_parameters: format_parameters(
                &provider.additional_authorization_parameters,
            ),
            ui_order: provider.ui_order.to_string(),
            on_backchannel_logout: text(&provider.on_backchannel_logout),
            claims_imports: provider
                .claims_imports
                .as_ref()
                .and_then(|value| serde_json::to_string_pretty(value).ok())
                .unwrap_or_default(),
        }
    }
}

fn format_parameters(params: &[(String, String)]) -> String {
    params
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_parameters(text: &str) -> Result<Vec<(String, String)>, String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| match line.split_once('=') {
            Some((key, value)) if !key.trim().is_empty() => {
                Ok((key.trim().to_owned(), value.trim().to_owned()))
            }
            _ => Err(format!(
                "Authorization parameters must be key=value, got: {line}"
            )),
        })
        .collect()
}

fn parse_ui_order(text: &str) -> Result<i32, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(0);
    }
    text.parse()
        .map_err(|_| format!("UI order must be an integer, got: {text}"))
}

fn parse_claims_imports(text: &str) -> Result<Option<Value>, String> {
    if text.trim().is_empty() {
        return Ok(None);
    }
    serde_json::from_str(text)
        .map(Some)
        .map_err(|e| format!("claims_imports must be valid JSON: {e}"))
}

fn require<'a>(field: &str, value: &'a str) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{field} is required"))
    } else {
        Ok(value)
    }
}

fn provider_display_name(provider: &PasionUpstreamProvider) -> String {
    provider
        .human_name
        .as_deref()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            provider
                .brand_name
                .as_deref()
                .filter(|value| !value.is_empty())
        })
        .or_else(|| provider.issuer.as_deref().filter(|value| !value.is_empty()))
        .unwrap_or(&provider.id)
        .to_string()
}

fn provider_initial(name: &str) -> String {
    name.chars()
        .find(|character| !character.is_whitespace())
        .map(|character| character.to_uppercase().collect())
        .unwrap_or_else(|| "?".to_string())
}

fn provider_is_config(provider: &PasionUpstreamProvider) -> bool {
    provider.source.as_deref() == Some("config")
}

fn display_optional(value: &Option<String>) -> String {
    value
        .as_deref()
        .filter(|value| !value.is_empty())
        .unwrap_or("-")
        .to_string()
}

fn create_provider_body(form: &ProviderForm) -> Result<Value, String> {
    let mut body = Map::new();
    body.insert(
        "client_id".into(),
        require("client_id", &form.client_id)?.into(),
    );
    body.insert("scope".into(), require("scope", &form.scope)?.into());
    for (key, value) in [
        (
            "token_endpoint_auth_method",
            &form.token_endpoint_auth_method,
        ),
        (
            "id_token_signed_response_alg",
            &form.id_token_signed_response_alg,
        ),
        ("discovery_mode", &form.discovery_mode),
        ("pkce_mode", &form.pkce_mode),
        ("on_backchannel_logout", &form.on_backchannel_logout),
    ] {
        body.insert(key.into(), require(key, value)?.into());
    }
    for (key, value) in [
        ("issuer", &form.issuer),
        ("human_name", &form.human_name),
        ("brand_name", &form.brand_name),
        (
            "token_endpoint_signing_alg",
            &form.token_endpoint_signing_alg,
        ),
        (
            "userinfo_signed_response_alg",
            &form.userinfo_signed_response_alg,
        ),
        ("response_mode", &form.response_mode),
        (
            "authorization_endpoint_override",
            &form.authorization_endpoint_override,
        ),
        ("token_endpoint_override", &form.token_endpoint_override),
        (
            "userinfo_endpoint_override",
            &form.userinfo_endpoint_override,
        ),
        ("jwks_uri_override", &form.jwks_uri_override),
    ] {
        let value = value.trim();
        if !value.is_empty() {
            body.insert(key.into(), value.into());
        }
    }
    if !form.client_secret.is_empty() {
        body.insert("client_secret".into(), form.client_secret.clone().into());
    }
    body.insert("fetch_userinfo".into(), form.fetch_userinfo.into());
    body.insert("forward_login_hint".into(), form.forward_login_hint.into());
    body.insert(
        "additional_authorization_parameters".into(),
        serde_json::to_value(parse_parameters(&form.additional_authorization_parameters)?)
            .map_err(|e| e.to_string())?,
    );
    body.insert("ui_order".into(), parse_ui_order(&form.ui_order)?.into());
    if let Some(claims_imports) = parse_claims_imports(&form.claims_imports)? {
        body.insert("claims_imports".into(), claims_imports);
    }
    Ok(Value::Object(body))
}

/// Build a partial PATCH body with only the fields that differ from
/// `original`. Returns `Ok(None)` when nothing changed.
fn update_provider_body(
    form: &ProviderForm,
    original: &PasionUpstreamProvider,
) -> Result<Option<Value>, String> {
    let mut body = Map::new();

    // Required fields: must stay non-empty.
    for (key, value, current) in [
        ("client_id", &form.client_id, original.client_id.as_deref()),
        ("scope", &form.scope, Some(original.scope.as_str())),
        (
            "token_endpoint_auth_method",
            &form.token_endpoint_auth_method,
            original.token_endpoint_auth_method.as_deref(),
        ),
        (
            "id_token_signed_response_alg",
            &form.id_token_signed_response_alg,
            original.id_token_signed_response_alg.as_deref(),
        ),
        (
            "discovery_mode",
            &form.discovery_mode,
            original.discovery_mode.as_deref(),
        ),
        ("pkce_mode", &form.pkce_mode, original.pkce_mode.as_deref()),
        (
            "on_backchannel_logout",
            &form.on_backchannel_logout,
            original.on_backchannel_logout.as_deref(),
        ),
    ] {
        let value = require(key, value)?;
        if Some(value) != current {
            body.insert(key.into(), value.into());
        }
    }

    // Optional fields: an emptied field is sent as `null` to clear it.
    for (key, value, current) in [
        ("issuer", &form.issuer, &original.issuer),
        ("human_name", &form.human_name, &original.human_name),
        ("brand_name", &form.brand_name, &original.brand_name),
        (
            "token_endpoint_signing_alg",
            &form.token_endpoint_signing_alg,
            &original.token_endpoint_signing_alg,
        ),
        (
            "userinfo_signed_response_alg",
            &form.userinfo_signed_response_alg,
            &original.userinfo_signed_response_alg,
        ),
        (
            "response_mode",
            &form.response_mode,
            &original.response_mode,
        ),
        (
            "authorization_endpoint_override",
            &form.authorization_endpoint_override,
            &original.authorization_endpoint_override,
        ),
        (
            "token_endpoint_override",
            &form.token_endpoint_override,
            &original.token_endpoint_override,
        ),
        (
            "userinfo_endpoint_override",
            &form.userinfo_endpoint_override,
            &original.userinfo_endpoint_override,
        ),
        (
            "jwks_uri_override",
            &form.jwks_uri_override,
            &original.jwks_uri_override,
        ),
    ] {
        let value = value.trim();
        if value != current.as_deref().unwrap_or_default() {
            let value = if value.is_empty() {
                Value::Null
            } else {
                value.into()
            };
            body.insert(key.into(), value);
        }
    }

    if !form.client_secret.is_empty() {
        body.insert("client_secret".into(), form.client_secret.clone().into());
    } else if form.clear_client_secret && original.has_client_secret {
        body.insert("client_secret".into(), Value::Null);
    }

    if form.fetch_userinfo != original.fetch_userinfo {
        body.insert("fetch_userinfo".into(), form.fetch_userinfo.into());
    }
    if form.forward_login_hint != original.forward_login_hint {
        body.insert("forward_login_hint".into(), form.forward_login_hint.into());
    }

    let params = parse_parameters(&form.additional_authorization_parameters)?;
    if params != original.additional_authorization_parameters {
        body.insert(
            "additional_authorization_parameters".into(),
            serde_json::to_value(params).map_err(|e| e.to_string())?,
        );
    }

    let ui_order = parse_ui_order(&form.ui_order)?;
    if ui_order != original.ui_order {
        body.insert("ui_order".into(), ui_order.into());
    }

    if let Some(parsed) = parse_claims_imports(&form.claims_imports)? {
        if original.claims_imports.as_ref() != Some(&parsed) {
            body.insert("claims_imports".into(), parsed);
        }
    }

    if body.is_empty() {
        Ok(None)
    } else {
        Ok(Some(Value::Object(body)))
    }
}

/// A `<select>` bound to one string field of the form.
#[component]
fn EnumSelect(
    value: String,
    options: &'static [&'static str],
    disabled: bool,
    onchange: EventHandler<String>,
) -> Element {
    // Keep unknown stored values selectable instead of silently replacing them.
    let unknown = (!options.contains(&value.as_str())).then(|| value.clone());
    rsx! {
        select {
            class: SELECT_CLASS,
            disabled,
            onchange: move |e: FormEvent| onchange.call(e.value()),
            if let Some(unknown) = unknown {
                option { value: "{unknown}", selected: true, "{unknown}" }
            }
            for option_value in options.iter() {
                option {
                    value: "{option_value}",
                    selected: *option_value == value.as_str(),
                    if option_value.is_empty() { "(default)" } else { "{option_value}" }
                }
            }
        }
    }
}

#[component]
fn FormSection(title: String, description: String, children: Element) -> Element {
    rsx! {
        section { class: "provider-form-section",
            div { class: "provider-form-section-heading",
                h3 { "{title}" }
                p { "{description}" }
            }
            div { class: "provider-form-grid", {children} }
        }
    }
}

#[component]
fn FormCheck(
    checked: bool,
    disabled: bool,
    label: String,
    description: String,
    #[props(default)] class: String,
    onchange: EventHandler<bool>,
) -> Element {
    rsx! {
        label { class: if disabled { "provider-check disabled {class}" } else { "provider-check {class}" },
            input {
                r#type: "checkbox",
                checked,
                disabled,
                oninput: move |event: FormEvent| {
                    onchange.call(event.value() == "true" || event.value() == "on");
                },
            }
            span { class: "provider-check-copy",
                strong { "{label}" }
                if !description.is_empty() {
                    small { "{description}" }
                }
            }
        }
    }
}

#[component]
pub fn UpstreamProvidersPage() -> Element {
    let mut providers_data =
        use_resource(|| async { pasion::pasion_get_upstream_providers().await });
    let mut provider_search = use_signal(String::new);

    let mut form_open = use_signal(|| false);
    let mut form = use_signal(ProviderForm::default);
    let mut editing_provider_id = use_signal(|| Option::<String>::None);
    let mut loaded_provider = use_signal(|| Option::<PasionUpstreamProvider>::None);
    let mut saving = use_signal(|| false);
    let mut loading_provider = use_signal(|| Option::<String>::None);

    let mut delete_open = use_signal(|| false);
    let mut delete_target = use_signal(|| Option::<String>::None);

    let mut toggling = use_signal(|| Option::<String>::None);

    let handle_save = move |_| {
        let form_data = form.read().clone();
        let editing_id = editing_provider_id.read().clone();
        let original_provider = loaded_provider.read().clone();
        let is_editing_now = editing_id.is_some();

        let body = match editing_id.as_deref() {
            Some(_) => match original_provider.as_ref() {
                Some(provider) => match update_provider_body(&form_data, provider) {
                    Ok(Some(body)) => body,
                    Ok(None) => {
                        show_toast(
                            &t("pasion.upstream_providers.no_changes"),
                            ToastVariant::Default,
                        );
                        return;
                    }
                    Err(message) => {
                        show_toast(&message, ToastVariant::Error);
                        return;
                    }
                },
                None => {
                    show_toast(
                        &t("pasion.upstream_providers.details_not_loaded"),
                        ToastVariant::Error,
                    );
                    return;
                }
            },
            None => match create_provider_body(&form_data) {
                Ok(body) => body,
                Err(message) => {
                    show_toast(&message, ToastVariant::Error);
                    return;
                }
            },
        };

        saving.set(true);
        spawn(async move {
            let res = if let Some(id) = editing_id {
                pasion::pasion_update_upstream_provider(&id, body).await
            } else {
                pasion::pasion_create_upstream_provider(body).await
            };
            match res {
                Ok(_) => {
                    let message = if is_editing_now {
                        t("pasion.upstream_providers.updated_toast")
                    } else {
                        t("pasion.upstream_providers.created_toast")
                    };
                    show_toast(&message, ToastVariant::Success);
                    form_open.set(false);
                    form.set(ProviderForm::for_create());
                    editing_provider_id.set(None);
                    loaded_provider.set(None);
                    providers_data.restart();
                }
                Err(e) => show_toast(
                    &format!("{}: {}", t("common.failed"), e.message),
                    ToastVariant::Error,
                ),
            }
            saving.set(false);
        });
    };

    let handle_delete_confirm = move |_| {
        if let Some(id) = delete_target.read().clone() {
            spawn(async move {
                match pasion::pasion_delete_upstream_provider(&id).await {
                    Ok(_) => {
                        show_toast(
                            &t("pasion.upstream_providers.deleted_toast"),
                            ToastVariant::Success,
                        );
                        delete_open.set(false);
                        delete_target.set(None);
                        providers_data.restart();
                    }
                    Err(e) => show_toast(
                        &format!("{}: {}", t("common.failed"), e.message),
                        ToastVariant::Error,
                    ),
                }
            });
        }
    };

    let is_saving = *saving.read();
    let is_editing = editing_provider_id.read().is_some();
    let is_view_only = is_editing
        && loaded_provider
            .read()
            .as_ref()
            .is_some_and(provider_is_config);
    let fields_disabled = is_saving || is_view_only;
    let loading_provider_id = loading_provider.read().clone();
    let has_stored_secret = is_editing
        && loaded_provider
            .read()
            .as_ref()
            .is_some_and(|provider| provider.has_client_secret);
    let client_secret_placeholder = if has_stored_secret {
        t("pasion.upstream_providers.secret_keep")
    } else if is_editing {
        t("pasion.upstream_providers.secret_none")
    } else {
        String::new()
    };
    let current_human_name = form.read().human_name.trim().to_string();
    let current_brand_name = form.read().brand_name.trim().to_string();
    let sheet_name = if !current_human_name.is_empty() {
        current_human_name
    } else if !current_brand_name.is_empty() {
        current_brand_name
    } else {
        t("pasion.upstream_providers.new_provider")
    };
    let sheet_initial = provider_initial(&sheet_name);
    let sheet_title = if is_view_only {
        t("pasion.upstream_providers.view_title")
    } else if is_editing {
        t("pasion.upstream_providers.edit_title")
    } else {
        t("pasion.upstream_providers.create_title")
    };
    let sheet_description = if is_view_only {
        t("pasion.upstream_providers.view_description")
    } else if is_editing {
        t("pasion.upstream_providers.edit_description")
    } else {
        t("pasion.upstream_providers.create_description")
    };
    let delete_provider_name = delete_target.read().clone().unwrap_or_default();
    let delete_description = t_with(
        "pasion.upstream_providers.delete_description",
        &[("name", &delete_provider_name)],
    );

    rsx! {
        div { class: "space-y-6",
            PageHeader {
                title: t("pasion.upstream_providers.title"),
                description: t("pasion.upstream_providers.description"),
                Button {
                    variant: ButtonVariant::Outline,
                    onclick: move |_| providers_data.restart(),
                    Icon { name: "refresh-cw".to_string(), class: "h-4 w-4 mr-2".to_string() }
                    {t("common.refresh")}
                }
                Button {
                    onclick: move |_| {
                        form.set(ProviderForm::for_create());
                        editing_provider_id.set(None);
                        loaded_provider.set(None);
                        provider_search.set(String::new());
                        form_open.set(true);
                    },
                    Icon { name: "plus".to_string(), class: "h-4 w-4 mr-2".to_string() }
                    {t("pasion.upstream_providers.add_provider")}
                }
            }

            match &*providers_data.read() {
                Some(Ok(providers)) => {
                    let query = provider_search.read().trim().to_lowercase();
                    let filtered: Vec<&PasionUpstreamProvider> = providers
                        .iter()
                        .filter(|provider| {
                            if query.is_empty() {
                                return true;
                            }
                            let searchable = [
                                provider.human_name.as_deref(),
                                provider.brand_name.as_deref(),
                                provider.issuer.as_deref(),
                                provider.client_id.as_deref(),
                            ]
                            .into_iter()
                            .flatten()
                            .collect::<Vec<_>>()
                            .join(" ")
                            .to_lowercase();
                            searchable.contains(&query)
                        })
                        .collect();
                    let total_count = providers.len().to_string();
                    let enabled_count = providers
                        .iter()
                        .filter(|provider| provider.disabled_at.is_none())
                        .count()
                        .to_string();
                    let config_count = providers
                        .iter()
                        .filter(|provider| provider_is_config(provider))
                        .count()
                        .to_string();

                    rsx! {
                        if providers.is_empty() {
                            div { class: "provider-empty",
                                div { class: "provider-empty-icon",
                                    Icon { name: "shield".to_string(), class: "h-7 w-7".to_string() }
                                }
                                div { class: "provider-empty-copy",
                                    p { class: "provider-eyebrow", {t("pasion.upstream_providers.federation")} }
                                    h2 { {t("pasion.upstream_providers.empty_title")} }
                                    p { {t("pasion.upstream_providers.empty_description")} }
                                }
                                Button {
                                    onclick: move |_| {
                                        form.set(ProviderForm::for_create());
                                        editing_provider_id.set(None);
                                        loaded_provider.set(None);
                                        form_open.set(true);
                                    },
                                    Icon { name: "plus".to_string(), class: "h-4 w-4 mr-2".to_string() }
                                    {t("pasion.upstream_providers.empty_action")}
                                }
                            }
                        } else {
                            div { class: "provider-overview",
                                div { class: "provider-overview-copy",
                                    div { class: "provider-overview-icon",
                                        Icon { name: "globe".to_string(), class: "h-5 w-5".to_string() }
                                    }
                                    div {
                                        p { class: "provider-eyebrow", {t("pasion.upstream_providers.federation")} }
                                        h2 { {t("pasion.upstream_providers.federation_title")} }
                                        p { {t("pasion.upstream_providers.federation_description")} }
                                    }
                                }
                                div { class: "provider-overview-stats",
                                    div { class: "provider-stat",
                                        strong { "{total_count}" }
                                        span { {t("pasion.upstream_providers.stat_total")} }
                                    }
                                    div { class: "provider-stat",
                                        strong { "{enabled_count}" }
                                        span { {t("pasion.upstream_providers.stat_enabled")} }
                                    }
                                    div { class: "provider-stat",
                                        strong { "{config_count}" }
                                        span { {t("pasion.upstream_providers.stat_config")} }
                                    }
                                }
                            }

                            div { class: "provider-toolbar",
                                div { class: "provider-search",
                                    SearchInput {
                                        placeholder: t("pasion.upstream_providers.search_placeholder"),
                                        value: provider_search.read().clone(),
                                        oninput: move |event: FormEvent| provider_search.set(event.value()),
                                    }
                                }
                                span { class: "provider-result-count",
                                    {t_with(
                                        "pasion.upstream_providers.result_count",
                                        &[("count", &filtered.len().to_string())],
                                    )}
                                }
                            }

                            if filtered.is_empty() {
                                div { class: "provider-no-results",
                                    Icon { name: "globe".to_string(), class: "h-6 w-6".to_string() }
                                    div {
                                        strong { {t("pasion.upstream_providers.no_matches")} }
                                        p { {t("pasion.upstream_providers.no_matches_description")} }
                                    }
                                    Button {
                                        variant: ButtonVariant::Outline,
                                        size: ButtonSize::Sm,
                                        onclick: move |_| provider_search.set(String::new()),
                                        {t("pasion.upstream_providers.clear_search")}
                                    }
                                }
                            } else {
                                div { class: "provider-grid",
                                    for provider in filtered.iter().copied() {
                                        {
                                            let pid = provider.id.clone();
                                            let pid_edit = pid.clone();
                                            let pid_toggle = pid.clone();
                                            let pid_delete = pid.clone();
                                            let display_name = provider_display_name(provider);
                                            let initial = provider_initial(&display_name);
                                            let issuer = display_optional(&provider.issuer);
                                            let client_id = display_optional(&provider.client_id);
                                            let discovery = display_optional(&provider.discovery_mode);
                                            let pkce = display_optional(&provider.pkce_mode);
                                            let is_config = provider_is_config(provider);
                                            let is_disabled = provider.disabled_at.is_some();
                                            let busy = toggling.read().as_deref() == Some(pid.as_str());
                                            let is_loading_current =
                                                loading_provider_id.as_deref() == Some(pid.as_str());

                                            rsx! {
                                                article { class: "provider-card", key: "{pid}",
                                                    div { class: "provider-card-header",
                                                        div { class: "provider-mark", "{initial}" }
                                                        div { class: "provider-card-heading",
                                                            h3 { "{display_name}" }
                                                            p { class: "provider-issuer", title: "{issuer}", "{issuer}" }
                                                        }
                                                        div { class: "provider-badges",
                                                            if is_config {
                                                                Badge {
                                                                    variant: BadgeVariant::Outline,
                                                                    {t("pasion.upstream_providers.source_config")}
                                                                }
                                                            } else {
                                                                Badge {
                                                                    variant: BadgeVariant::Secondary,
                                                                    {t("pasion.upstream_providers.source_manual")}
                                                                }
                                                            }
                                                            if is_disabled {
                                                                Badge {
                                                                    variant: BadgeVariant::Secondary,
                                                                    {t("pasion.upstream_providers.status_disabled")}
                                                                }
                                                            } else {
                                                                Badge {
                                                                    variant: BadgeVariant::Success,
                                                                    {t("pasion.upstream_providers.status_enabled")}
                                                                }
                                                            }
                                                        }
                                                    }
                                                    div { class: "provider-card-details",
                                                        div { class: "provider-card-detail",
                                                            span { {t("pasion.upstream_providers.field_discovery")} }
                                                            strong { "{discovery}" }
                                                        }
                                                        div { class: "provider-card-detail",
                                                            span { {t("pasion.upstream_providers.field_pkce")} }
                                                            strong { "{pkce}" }
                                                        }
                                                        div { class: "provider-card-detail provider-card-detail-wide",
                                                            span { {t("pasion.upstream_providers.field_client_id")} }
                                                            strong { class: "font-mono", title: "{client_id}", "{client_id}" }
                                                        }
                                                    }
                                                    div { class: "provider-card-footer",
                                                        span { class: "provider-card-hint",
                                                            if is_config {
                                                                {t("pasion.upstream_providers.managed_config_hint")}
                                                            } else {
                                                                {t("pasion.upstream_providers.managed_manual_hint")}
                                                            }
                                                        }
                                                        div { class: "provider-card-actions",
                                                            if is_config {
                                                                Button {
                                                                    variant: ButtonVariant::Outline,
                                                                    size: ButtonSize::Sm,
                                                                    disabled: is_loading_current,
                                                                    onclick: move |_| {
                                                                        let id = pid_edit.clone();
                                                                        loading_provider.set(Some(id.clone()));
                                                                        spawn(async move {
                                                                            match pasion::pasion_get_upstream_provider(&id).await {
                                                                                Ok(provider) => {
                                                                                    form.set(ProviderForm::from_provider(&provider));
                                                                                    loaded_provider.set(Some(provider));
                                                                                    editing_provider_id.set(Some(id));
                                                                                    form_open.set(true);
                                                                                }
                                                                                Err(e) => show_toast(&format!("{}: {}", t("common.failed"), e.message), ToastVariant::Error),
                                                                            }
                                                                            loading_provider.set(None);
                                                                        });
                                                                    },
                                                                    if is_loading_current {
                                                                        Spinner { class: "mr-2".to_string() }
                                                                    }
                                                                    Icon { name: "key".to_string(), class: "h-4 w-4 mr-2".to_string() }
                                                                    {t("pasion.upstream_providers.view")}
                                                                }
                                                            } else {
                                                                Button {
                                                                    variant: ButtonVariant::Outline,
                                                                    size: ButtonSize::Sm,
                                                                    disabled: is_loading_current,
                                                                    onclick: move |_| {
                                                                        let id = pid_edit.clone();
                                                                        loading_provider.set(Some(id.clone()));
                                                                        spawn(async move {
                                                                            match pasion::pasion_get_upstream_provider(&id).await {
                                                                                Ok(provider) => {
                                                                                    form.set(ProviderForm::from_provider(&provider));
                                                                                    loaded_provider.set(Some(provider));
                                                                                    editing_provider_id.set(Some(id));
                                                                                    form_open.set(true);
                                                                                }
                                                                                Err(e) => show_toast(&format!("{}: {}", t("common.failed"), e.message), ToastVariant::Error),
                                                                            }
                                                                            loading_provider.set(None);
                                                                        });
                                                                    },
                                                                    if is_loading_current {
                                                                        Spinner { class: "mr-2".to_string() }
                                                                    }
                                                                    Icon { name: "edit".to_string(), class: "h-4 w-4 mr-2".to_string() }
                                                                    {t("common.edit")}
                                                                }
                                                                Button {
                                                                    variant: ButtonVariant::Ghost,
                                                                    size: ButtonSize::Sm,
                                                                    disabled: busy,
                                                                    onclick: move |_| {
                                                                        let id = pid_toggle.clone();
                                                                        toggling.set(Some(id.clone()));
                                                                        spawn(async move {
                                                                            let res = if is_disabled {
                                                                                pasion::pasion_enable_upstream_provider(&id).await
                                                                            } else {
                                                                                pasion::pasion_disable_upstream_provider(&id).await
                                                                            };
                                                                            match res {
                                                                                Ok(_) => {
                                                                                    let message = if is_disabled {
                                                                                        t("pasion.upstream_providers.enabled_toast")
                                                                                    } else {
                                                                                        t("pasion.upstream_providers.disabled_toast")
                                                                                    };
                                                                                    show_toast(
                                                                                        &message,
                                                                                        ToastVariant::Success,
                                                                                    );
                                                                                    providers_data.restart();
                                                                                }
                                                                                Err(e) => show_toast(&format!("{}: {}", t("common.failed"), e.message), ToastVariant::Error),
                                                                            }
                                                                            toggling.set(None);
                                                                        });
                                                                    },
                                                                    if is_disabled {
                                                                        {t("pasion.upstream_providers.enable")}
                                                                    } else {
                                                                        {t("pasion.upstream_providers.disable")}
                                                                    }
                                                                }
                                                                Button {
                                                                    variant: ButtonVariant::Ghost,
                                                                    size: ButtonSize::Sm,
                                                                    class: "text-destructive hover:text-destructive".to_string(),
                                                                    onclick: move |_| {
                                                                        delete_target.set(Some(pid_delete.clone()));
                                                                        delete_open.set(true);
                                                                    },
                                                                    Icon { name: "trash".to_string(), class: "h-4 w-4 mr-2".to_string() }
                                                                    {t("common.delete")}
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
                },
                Some(Err(e)) => rsx! {
                    ErrorBanner {
                        message: e.message.clone(),
                        on_retry: move |_| providers_data.restart(),
                    }
                },
                None => rsx! { PageSkeleton {} },
            }
        }

        Modal {
            open: *form_open.read(),
            on_close: move |_| {
                if !is_saving {
                    form_open.set(false);
                    editing_provider_id.set(None);
                    loaded_provider.set(None);
                }
            },
            size: ModalSize::Xl2,
            default_padding: false,
            container_class: "oauth-provider-modal".to_string(),
            panel_class: "oauth-provider-sheet".to_string(),
            div {
                class: "provider-sheet",
                role: "dialog",
                aria_modal: "true",
                aria_labelledby: "upstream-provider-sheet-title",
                aria_describedby: "upstream-provider-sheet-description",
                div { class: "provider-sheet-header",
                    div { class: "provider-sheet-identity",
                        div { class: "provider-mark provider-mark-lg", "{sheet_initial}" }
                        div {
                            p { class: "provider-eyebrow",
                                if is_view_only {
                                    {t("pasion.upstream_providers.eyebrow_config")}
                                } else if is_editing {
                                    {t("pasion.upstream_providers.eyebrow_edit")}
                                } else {
                                    {t("pasion.upstream_providers.eyebrow_new")}
                                }
                            }
                            h2 { id: "upstream-provider-sheet-title", "{sheet_title}" }
                            p { id: "upstream-provider-sheet-description", "{sheet_description}" }
                        }
                    }
                    button {
                        r#type: "button",
                        class: "provider-sheet-close",
                        aria_label: t("pasion.upstream_providers.close"),
                        title: t("pasion.upstream_providers.close"),
                        disabled: is_saving,
                        onclick: move |_| {
                            if !is_saving {
                                form_open.set(false);
                                editing_provider_id.set(None);
                                loaded_provider.set(None);
                            }
                        },
                        Icon { name: "x".to_string(), class: "h-5 w-5".to_string() }
                    }
                }
                if is_view_only {
                    div { class: "provider-readonly-banner",
                        Icon { name: "lock".to_string(), class: "h-4 w-4".to_string() }
                        span { {t("pasion.upstream_providers.readonly_banner")} }
                    }
                }
                div { class: "provider-sheet-body",
                    FormSection {
                        title: t("pasion.upstream_providers.section_identity"),
                        description: t("pasion.upstream_providers.section_identity_description"),
                        div { class: "provider-field provider-field-wide",
                            Label { r#for: "provider-issuer",
                                {t("pasion.upstream_providers.issuer")}
                            }
                            Input {
                                id: "provider-issuer".to_string(),
                                placeholder: "https://accounts.google.com".to_string(),
                                value: form.read().issuer.clone(),
                                oninput: move |e: FormEvent| form.write().issuer = e.value(),
                                disabled: fields_disabled,
                            }
                            p { class: "provider-field-help",
                                {t("pasion.upstream_providers.issuer_help")}
                            }
                        }
                        div { class: "provider-field",
                            Label { r#for: "provider-human-name",
                                {t("pasion.upstream_providers.human_name")}
                            }
                            Input {
                                id: "provider-human-name".to_string(),
                                placeholder: "Google".to_string(),
                                value: form.read().human_name.clone(),
                                oninput: move |e: FormEvent| form.write().human_name = e.value(),
                                disabled: fields_disabled,
                            }
                        }
                        div { class: "provider-field",
                            Label { r#for: "provider-brand-name",
                                {t("pasion.upstream_providers.brand_name")}
                            }
                            Input {
                                id: "provider-brand-name".to_string(),
                                placeholder: "google".to_string(),
                                value: form.read().brand_name.clone(),
                                oninput: move |e: FormEvent| form.write().brand_name = e.value(),
                                disabled: fields_disabled,
                            }
                        }
                        div { class: "provider-field",
                            Label { r#for: "provider-ui-order",
                                {t("pasion.upstream_providers.ui_order")}
                            }
                            Input {
                                id: "provider-ui-order".to_string(),
                                r#type: "number".to_string(),
                                value: form.read().ui_order.clone(),
                                oninput: move |e: FormEvent| form.write().ui_order = e.value(),
                                disabled: fields_disabled,
                            }
                            p { class: "provider-field-help",
                                {t("pasion.upstream_providers.ui_order_help")}
                            }
                        }
                    }

                    FormSection {
                        title: t("pasion.upstream_providers.section_client"),
                        description: t("pasion.upstream_providers.section_client_description"),
                        div { class: "provider-field",
                            Label { r#for: "provider-client-id",
                                {t("pasion.upstream_providers.client_id")}
                            }
                            Input {
                                id: "provider-client-id".to_string(),
                                value: form.read().client_id.clone(),
                                oninput: move |e: FormEvent| form.write().client_id = e.value(),
                                disabled: fields_disabled,
                            }
                        }
                        div { class: "provider-field provider-field-wide",
                            Label { r#for: "provider-client-secret",
                                {t("pasion.upstream_providers.client_secret")}
                            }
                            Input {
                                id: "provider-client-secret".to_string(),
                                r#type: "password".to_string(),
                                autocomplete: "off".to_string(),
                                placeholder: client_secret_placeholder,
                                value: form.read().client_secret.clone(),
                                oninput: move |e: FormEvent| form.write().client_secret = e.value(),
                                disabled: fields_disabled || form.read().clear_client_secret,
                            }
                            if has_stored_secret {
                                FormCheck {
                                    checked: form.read().clear_client_secret,
                                    disabled: fields_disabled,
                                    label: t("pasion.upstream_providers.remove_secret"),
                                    description: t("pasion.upstream_providers.remove_secret_description"),
                                    class: "provider-check-compact".to_string(),
                                    onchange: move |checked| {
                                            let mut f = form.write();
                                            f.clear_client_secret = checked;
                                            if checked {
                                                f.client_secret.clear();
                                            }
                                    }
                                }
                            }
                        }
                        div { class: "provider-field",
                            Label { r#for: "provider-token-auth-method",
                                {t("pasion.upstream_providers.token_auth_method")}
                            }
                            EnumSelect {
                                value: form.read().token_endpoint_auth_method.clone(),
                                options: TOKEN_AUTH_METHODS,
                                disabled: fields_disabled,
                                onchange: move |v: String| form.write().token_endpoint_auth_method = v,
                            }
                        }
                        div { class: "provider-field",
                            Label { r#for: "provider-token-signing-alg",
                                {t("pasion.upstream_providers.token_signing_alg")}
                            }
                            Input {
                                id: "provider-token-signing-alg".to_string(),
                                placeholder: "RS256".to_string(),
                                value: form.read().token_endpoint_signing_alg.clone(),
                                oninput: move |e: FormEvent| form.write().token_endpoint_signing_alg = e.value(),
                                disabled: fields_disabled,
                            }
                            p { class: "provider-field-help",
                                {t("pasion.upstream_providers.token_signing_alg_help")}
                            }
                        }
                        div { class: "provider-field provider-field-wide",
                            Label { r#for: "provider-scope",
                                {t("pasion.upstream_providers.scope")}
                            }
                            Input {
                                id: "provider-scope".to_string(),
                                value: form.read().scope.clone(),
                                oninput: move |e: FormEvent| form.write().scope = e.value(),
                                disabled: fields_disabled,
                            }
                        }
                    }

                    FormSection {
                        title: t("pasion.upstream_providers.section_discovery"),
                        description: t("pasion.upstream_providers.section_discovery_description"),
                        div { class: "provider-field",
                            Label { r#for: "provider-discovery-mode",
                                {t("pasion.upstream_providers.discovery_mode")}
                            }
                            EnumSelect {
                                value: form.read().discovery_mode.clone(),
                                options: DISCOVERY_MODES,
                                disabled: fields_disabled,
                                onchange: move |v: String| form.write().discovery_mode = v,
                            }
                        }
                        div { class: "provider-field",
                            Label { r#for: "provider-pkce-mode",
                                {t("pasion.upstream_providers.pkce_mode")}
                            }
                            EnumSelect {
                                value: form.read().pkce_mode.clone(),
                                options: PKCE_MODES,
                                disabled: fields_disabled,
                                onchange: move |v: String| form.write().pkce_mode = v,
                            }
                        }
                        div { class: "provider-field",
                            Label { r#for: "provider-response-mode",
                                {t("pasion.upstream_providers.response_mode")}
                            }
                            EnumSelect {
                                value: form.read().response_mode.clone(),
                                options: RESPONSE_MODES,
                                disabled: fields_disabled,
                                onchange: move |v: String| form.write().response_mode = v,
                            }
                        }
                        div { class: "provider-field provider-field-wide",
                            Label { r#for: "provider-authorization-endpoint",
                                {t("pasion.upstream_providers.authorization_endpoint")}
                            }
                            Input {
                                id: "provider-authorization-endpoint".to_string(),
                                placeholder: "https://github.com/login/oauth/authorize".to_string(),
                                value: form.read().authorization_endpoint_override.clone(),
                                oninput: move |e: FormEvent| form.write().authorization_endpoint_override = e.value(),
                                disabled: fields_disabled,
                            }
                        }
                        div { class: "provider-field provider-field-wide",
                            Label { r#for: "provider-token-endpoint",
                                {t("pasion.upstream_providers.token_endpoint")}
                            }
                            Input {
                                id: "provider-token-endpoint".to_string(),
                                placeholder: "https://github.com/login/oauth/access_token".to_string(),
                                value: form.read().token_endpoint_override.clone(),
                                oninput: move |e: FormEvent| form.write().token_endpoint_override = e.value(),
                                disabled: fields_disabled,
                            }
                        }
                        div { class: "provider-field provider-field-wide",
                            Label { r#for: "provider-userinfo-endpoint",
                                {t("pasion.upstream_providers.userinfo_endpoint")}
                            }
                            Input {
                                id: "provider-userinfo-endpoint".to_string(),
                                placeholder: "https://api.github.com/user".to_string(),
                                value: form.read().userinfo_endpoint_override.clone(),
                                oninput: move |e: FormEvent| form.write().userinfo_endpoint_override = e.value(),
                                disabled: fields_disabled,
                            }
                        }
                        div { class: "provider-field provider-field-wide",
                            Label { r#for: "provider-jwks-uri",
                                {t("pasion.upstream_providers.jwks_uri")}
                            }
                            Input {
                                id: "provider-jwks-uri".to_string(),
                                value: form.read().jwks_uri_override.clone(),
                                oninput: move |e: FormEvent| form.write().jwks_uri_override = e.value(),
                                disabled: fields_disabled,
                            }
                            p { class: "provider-field-help",
                                {t("pasion.upstream_providers.endpoints_help")}
                            }
                        }
                    }

                    FormSection {
                        title: t("pasion.upstream_providers.section_tokens"),
                        description: t("pasion.upstream_providers.section_tokens_description"),
                        div { class: "provider-field",
                            Label { r#for: "provider-id-token-alg",
                                {t("pasion.upstream_providers.id_token_alg")}
                            }
                            Input {
                                id: "provider-id-token-alg".to_string(),
                                value: form.read().id_token_signed_response_alg.clone(),
                                oninput: move |e: FormEvent| form.write().id_token_signed_response_alg = e.value(),
                                disabled: fields_disabled,
                            }
                        }
                        div { class: "provider-field",
                            Label { r#for: "provider-userinfo-alg",
                                {t("pasion.upstream_providers.userinfo_signed_alg")}
                            }
                            Input {
                                id: "provider-userinfo-alg".to_string(),
                                placeholder: "(unsigned)".to_string(),
                                value: form.read().userinfo_signed_response_alg.clone(),
                                oninput: move |e: FormEvent| form.write().userinfo_signed_response_alg = e.value(),
                                disabled: fields_disabled,
                            }
                        }
                        FormCheck {
                            checked: form.read().fetch_userinfo,
                            disabled: fields_disabled,
                            label: t("pasion.upstream_providers.fetch_userinfo"),
                            description: t("pasion.upstream_providers.fetch_userinfo_description"),
                            onchange: move |checked| form.write().fetch_userinfo = checked,
                        }
                        FormCheck {
                            checked: form.read().forward_login_hint,
                            disabled: fields_disabled,
                            label: t("pasion.upstream_providers.forward_login_hint"),
                            description: t("pasion.upstream_providers.forward_login_hint_description"),
                            onchange: move |checked| form.write().forward_login_hint = checked,
                        }
                    }

                    details { class: "provider-advanced",
                        summary {
                            div {
                                Icon { name: "code".to_string(), class: "h-4 w-4".to_string() }
                                span {
                                    strong { {t("pasion.upstream_providers.advanced")} }
                                    small { {t("pasion.upstream_providers.advanced_description")} }
                                }
                            }
                            Icon { name: "chevron-down".to_string(), class: "h-4 w-4".to_string() }
                        }
                        div { class: "provider-advanced-body",
                            div { class: "provider-field",
                                Label { r#for: "provider-backchannel-logout",
                                    {t("pasion.upstream_providers.backchannel_logout")}
                                }
                                EnumSelect {
                                    value: form.read().on_backchannel_logout.clone(),
                                    options: BACKCHANNEL_LOGOUT_MODES,
                                    disabled: fields_disabled,
                                    onchange: move |v: String| form.write().on_backchannel_logout = v,
                                }
                            }
                            div { class: "provider-field provider-field-wide",
                                Label { r#for: "provider-extra-params",
                                    {t("pasion.upstream_providers.extra_params")}
                                }
                                textarea {
                                    id: "provider-extra-params",
                                    class: "provider-textarea",
                                    placeholder: "prompt=consent",
                                    value: form.read().additional_authorization_parameters.clone(),
                                    oninput: move |e: FormEvent| form.write().additional_authorization_parameters = e.value(),
                                    disabled: fields_disabled,
                                }
                                p { class: "provider-field-help",
                                    {t("pasion.upstream_providers.extra_params_help")}
                                }
                            }
                            div { class: "provider-field provider-field-wide",
                                Label { r#for: "provider-claims-imports",
                                    {t("pasion.upstream_providers.claims_imports")}
                                }
                                textarea {
                                    id: "provider-claims-imports",
                                    class: "provider-textarea provider-textarea-lg",
                                    value: form.read().claims_imports.clone(),
                                    oninput: move |e: FormEvent| form.write().claims_imports = e.value(),
                                    disabled: fields_disabled,
                                }
                                p { class: "provider-field-help",
                                    {t("pasion.upstream_providers.claims_imports_help")}
                                }
                            }
                        }
                    }
                }
                div { class: "provider-sheet-footer",
                    p { class: "provider-sheet-footer-note",
                        if is_view_only {
                            {t("pasion.upstream_providers.view_footer_note")}
                        } else {
                            {t("pasion.upstream_providers.edit_footer_note")}
                        }
                    }
                    div { class: "provider-sheet-footer-actions",
                        Button {
                            variant: ButtonVariant::Outline,
                            disabled: is_saving,
                            onclick: move |_| {
                                form_open.set(false);
                                editing_provider_id.set(None);
                                loaded_provider.set(None);
                            },
                            if is_view_only {
                                {t("pasion.upstream_providers.close")}
                            } else {
                                {t("common.cancel")}
                            }
                        }
                        if !is_view_only {
                            Button {
                                disabled: is_saving,
                                onclick: handle_save,
                                if is_saving { Spinner { class: "mr-2".to_string() } }
                                if is_editing {
                                    {t("pasion.upstream_providers.save_changes")}
                                } else {
                                    {t("pasion.upstream_providers.create_provider")}
                                }
                            }
                        }
                    }
                }
            }
        }

        ConfirmDialog {
            open: *delete_open.read(),
            title: t("pasion.upstream_providers.delete_title"),
            description: delete_description,
            confirm_text: t("common.delete"),
            destructive: true,
            on_confirm: handle_delete_confirm,
            on_cancel: move |_| { delete_open.set(false); delete_target.set(None); },
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn stored_github() -> PasionUpstreamProvider {
        PasionUpstreamProvider {
            id: "01FSHN9AG0E6J8AS3YVE0HPDQ1".into(),
            human_name: Some("GitHub".into()),
            brand_name: Some("github".into()),
            source: Some("manual".into()),
            client_id: Some("gh-client".into()),
            has_client_secret: true,
            scope: "read:user user:email".into(),
            token_endpoint_auth_method: Some("client_secret_post".into()),
            id_token_signed_response_alg: Some("RS256".into()),
            discovery_mode: Some("disabled".into()),
            pkce_mode: Some("auto".into()),
            authorization_endpoint_override: Some(
                "https://github.com/login/oauth/authorize".into(),
            ),
            token_endpoint_override: Some("https://github.com/login/oauth/access_token".into()),
            userinfo_endpoint_override: Some("https://api.github.com/user".into()),
            fetch_userinfo: true,
            additional_authorization_parameters: vec![("allow_signup".into(), "false".into())],
            ui_order: 5,
            on_backchannel_logout: Some("do_nothing".into()),
            claims_imports: Some(json!({ "skip_confirmation": false })),
            ..Default::default()
        }
    }

    #[test]
    fn unchanged_form_produces_no_update() {
        let provider = stored_github();
        let form = ProviderForm::from_provider(&provider);
        assert_eq!(update_provider_body(&form, &provider), Ok(None));
    }

    #[test]
    fn update_sends_only_changed_fields() {
        let provider = stored_github();
        let mut form = ProviderForm::from_provider(&provider);
        form.human_name = "GitHub Enterprise".into();
        form.brand_name.clear();
        form.ui_order = "1".into();

        let body = update_provider_body(&form, &provider).unwrap().unwrap();
        assert_eq!(
            body,
            json!({
                "human_name": "GitHub Enterprise",
                "brand_name": null,
                "ui_order": 1,
            })
        );
    }

    #[test]
    fn update_secret_handling() {
        let provider = stored_github();

        let mut form = ProviderForm::from_provider(&provider);
        form.client_secret = "rotated".into();
        let body = update_provider_body(&form, &provider).unwrap().unwrap();
        assert_eq!(body, json!({ "client_secret": "rotated" }));

        let mut form = ProviderForm::from_provider(&provider);
        form.clear_client_secret = true;
        let body = update_provider_body(&form, &provider).unwrap().unwrap();
        assert_eq!(body, json!({ "client_secret": null }));
    }

    #[test]
    fn update_rejects_empty_required_field() {
        let provider = stored_github();
        let mut form = ProviderForm::from_provider(&provider);
        form.client_id = "  ".into();
        assert!(update_provider_body(&form, &provider).is_err());
    }

    #[test]
    fn create_body_includes_endpoints_and_parameters() {
        let mut form = ProviderForm::for_create();
        form.client_id = "gh-client".into();
        form.discovery_mode = "disabled".into();
        form.token_endpoint_override = "https://github.com/login/oauth/access_token".into();
        form.additional_authorization_parameters =
            "allow_signup=false\n\n prompt = consent ".into();

        let body = create_provider_body(&form).unwrap();
        assert_eq!(body["client_id"], "gh-client");
        assert_eq!(body["discovery_mode"], "disabled");
        assert_eq!(
            body["token_endpoint_override"],
            "https://github.com/login/oauth/access_token"
        );
        assert_eq!(
            body["additional_authorization_parameters"],
            json!([["allow_signup", "false"], ["prompt", "consent"]])
        );
        assert_eq!(body["ui_order"], 0);
        assert!(body.get("issuer").is_none());
        assert!(body.get("client_secret").is_none());
    }

    #[test]
    fn invalid_parameter_line_is_rejected() {
        assert!(parse_parameters("no-equals-sign").is_err());
        assert!(parse_parameters("=value").is_err());
    }

    #[test]
    fn provider_card_helpers_fall_back_cleanly() {
        let provider = stored_github();
        assert_eq!(provider_display_name(&provider), "GitHub");
        assert_eq!(provider_initial(&provider_display_name(&provider)), "G");

        let fallback = PasionUpstreamProvider {
            id: "provider-id".into(),
            ..Default::default()
        };
        assert_eq!(provider_display_name(&fallback), "provider-id");
        assert_eq!(provider_initial("provider-id"), "P");
    }

    #[test]
    fn config_sources_are_read_only() {
        let manual = stored_github();
        assert!(!provider_is_config(&manual));

        let config = PasionUpstreamProvider {
            source: Some("config".into()),
            ..manual
        };
        assert!(provider_is_config(&config));
    }
}
