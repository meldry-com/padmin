//! Upstream OAuth provider management.
//!
//! Lists providers and supports admin CRUD added in pasion commit 87212ca:
//! create / update / delete plus enable / disable. Providers loaded from the
//! pasion config file (`source = "config"`) are read-only here — the API
//! refuses to mutate them, so the UI hides the destructive actions for them.

use dioxus::prelude::*;

use crate::api::pasion;
use crate::components::ui::badge::{Badge, BadgeVariant};
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::dialog::ConfirmDialog;
use crate::components::ui::error_banner::ErrorBanner;
use crate::components::ui::icons::Icon;
use crate::components::ui::input::{Input, Label};
use crate::components::ui::loading::{PageSkeleton, Spinner};
use crate::components::ui::page_header::PageHeader;
use crate::components::ui::table::*;
use crate::components::ui::toast::{ToastVariant, show_toast};
use crate::utils::i18n::t;

#[derive(Default, Clone, PartialEq)]
struct ProviderForm {
    issuer: String,
    human_name: String,
    brand_name: String,
    client_id: String,
    client_secret: String,
    scope: String,
    token_endpoint_auth_method: String,
    id_token_signed_response_alg: String,
    discovery_mode: String,
    pkce_mode: String,
    fetch_userinfo: bool,
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
            ..Default::default()
        }
    }

    fn from_provider(provider: &crate::types::PasionUpstreamProvider) -> Self {
        Self {
            issuer: provider.issuer.clone().unwrap_or_default(),
            human_name: provider.human_name.clone().unwrap_or_default(),
            brand_name: provider.brand_name.clone().unwrap_or_default(),
            client_id: provider.client_id.clone().unwrap_or_default(),
            client_secret: String::new(),
            scope: if provider.scope.is_empty() {
                "openid email profile".into()
            } else {
                provider.scope.clone()
            },
            token_endpoint_auth_method: provider
                .token_endpoint_auth_method
                .clone()
                .unwrap_or_else(|| "client_secret_basic".into()),
            id_token_signed_response_alg: provider
                .id_token_signed_response_alg
                .clone()
                .unwrap_or_else(|| "RS256".into()),
            discovery_mode: provider
                .discovery_mode
                .clone()
                .unwrap_or_else(|| "oidc".into()),
            pkce_mode: provider.pkce_mode.clone().unwrap_or_else(|| "auto".into()),
            fetch_userinfo: provider.fetch_userinfo,
            claims_imports: provider
                .claims_imports
                .as_ref()
                .and_then(|value| serde_json::to_string_pretty(value).ok())
                .unwrap_or_default(),
        }
    }
}

fn create_provider_body(form: &ProviderForm) -> Result<serde_json::Value, String> {
    if form.client_id.trim().is_empty() {
        return Err("client_id is required".into());
    }

    let mut body = serde_json::json!({
        "client_id": form.client_id.trim(),
        "scope": form.scope.trim(),
        "token_endpoint_auth_method": form.token_endpoint_auth_method.trim(),
        "id_token_signed_response_alg": form.id_token_signed_response_alg.trim(),
        "discovery_mode": form.discovery_mode.trim(),
        "pkce_mode": form.pkce_mode.trim(),
        "fetch_userinfo": form.fetch_userinfo,
    });

    if !form.issuer.trim().is_empty() {
        body["issuer"] = form.issuer.trim().into();
    }
    if !form.human_name.trim().is_empty() {
        body["human_name"] = form.human_name.trim().into();
    }
    if !form.brand_name.trim().is_empty() {
        body["brand_name"] = form.brand_name.trim().into();
    }
    if !form.client_secret.is_empty() {
        body["client_secret"] = form.client_secret.clone().into();
    }
    if !form.claims_imports.trim().is_empty() {
        body["claims_imports"] = serde_json::from_str::<serde_json::Value>(&form.claims_imports)
            .map_err(|e| format!("claims_imports must be valid JSON: {e}"))?;
    }

    Ok(body)
}

fn update_provider_body(
    form: &ProviderForm,
    original: &crate::types::PasionUpstreamProvider,
) -> Result<Option<serde_json::Value>, String> {
    let mut body = serde_json::Map::new();

    let issuer = form.issuer.trim();
    if !issuer.is_empty() && original.issuer.as_deref().unwrap_or_default() != issuer {
        body.insert("issuer".into(), issuer.into());
    }

    let human_name = form.human_name.trim();
    if !human_name.is_empty() && original.human_name.as_deref().unwrap_or_default() != human_name {
        body.insert("human_name".into(), human_name.into());
    }

    let brand_name = form.brand_name.trim();
    if !brand_name.is_empty() && original.brand_name.as_deref().unwrap_or_default() != brand_name {
        body.insert("brand_name".into(), brand_name.into());
    }

    let client_id = form.client_id.trim();
    if !client_id.is_empty() && original.client_id.as_deref().unwrap_or_default() != client_id {
        body.insert("client_id".into(), client_id.into());
    }

    let scope = form.scope.trim();
    if !scope.is_empty() && original.scope != scope {
        body.insert("scope".into(), scope.into());
    }

    let token_auth = form.token_endpoint_auth_method.trim();
    if !token_auth.is_empty()
        && original
            .token_endpoint_auth_method
            .as_deref()
            .unwrap_or_default()
            != token_auth
    {
        body.insert("token_endpoint_auth_method".into(), token_auth.into());
    }

    let id_token_alg = form.id_token_signed_response_alg.trim();
    if !id_token_alg.is_empty()
        && original
            .id_token_signed_response_alg
            .as_deref()
            .unwrap_or_default()
            != id_token_alg
    {
        body.insert("id_token_signed_response_alg".into(), id_token_alg.into());
    }

    let discovery_mode = form.discovery_mode.trim();
    if !discovery_mode.is_empty()
        && original.discovery_mode.as_deref().unwrap_or_default() != discovery_mode
    {
        body.insert("discovery_mode".into(), discovery_mode.into());
    }

    let pkce_mode = form.pkce_mode.trim();
    if !pkce_mode.is_empty() && original.pkce_mode.as_deref().unwrap_or_default() != pkce_mode {
        body.insert("pkce_mode".into(), pkce_mode.into());
    }

    if form.fetch_userinfo != original.fetch_userinfo {
        body.insert("fetch_userinfo".into(), form.fetch_userinfo.into());
    }

    if !form.client_secret.is_empty() {
        body.insert("client_secret".into(), form.client_secret.clone().into());
    }

    if !form.claims_imports.trim().is_empty() {
        let parsed = serde_json::from_str::<serde_json::Value>(&form.claims_imports)
            .map_err(|e| format!("claims_imports must be valid JSON: {e}"))?;
        if original.claims_imports.as_ref() != Some(&parsed) {
            body.insert("claims_imports".into(), parsed);
        }
    }

    if body.is_empty() {
        Ok(None)
    } else {
        Ok(Some(serde_json::Value::Object(body)))
    }
}

#[component]
pub fn UpstreamProvidersPage() -> Element {
    let mut providers_data =
        use_resource(|| async { pasion::pasion_get_upstream_providers().await });

    let mut form_open = use_signal(|| false);
    let mut form = use_signal(ProviderForm::default);
    let mut editing_provider_id = use_signal(|| Option::<String>::None);
    let mut loaded_provider = use_signal(|| Option::<crate::types::PasionUpstreamProvider>::None);
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
                        show_toast("No changes to save", ToastVariant::Default);
                        return;
                    }
                    Err(message) => {
                        show_toast(&message, ToastVariant::Error);
                        return;
                    }
                },
                None => {
                    show_toast("Provider details are not loaded yet", ToastVariant::Error);
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
                    show_toast(
                        if is_editing_now {
                            "Provider updated"
                        } else {
                            "Provider created"
                        },
                        ToastVariant::Success,
                    );
                    form_open.set(false);
                    form.set(ProviderForm::for_create());
                    editing_provider_id.set(None);
                    loaded_provider.set(None);
                    providers_data.restart();
                }
                Err(e) => show_toast(&format!("Failed: {}", e.message), ToastVariant::Error),
            }
            saving.set(false);
        });
    };

    let handle_delete_confirm = move |_| {
        if let Some(id) = delete_target.read().clone() {
            spawn(async move {
                match pasion::pasion_delete_upstream_provider(&id).await {
                    Ok(_) => {
                        show_toast("Provider deleted", ToastVariant::Success);
                        delete_open.set(false);
                        delete_target.set(None);
                        providers_data.restart();
                    }
                    Err(e) => show_toast(&format!("Failed: {}", e.message), ToastVariant::Error),
                }
            });
        }
    };

    let is_saving = *saving.read();
    let is_editing = editing_provider_id.read().is_some();
    let loading_provider_id = loading_provider.read().clone();
    let client_secret_placeholder = if is_editing {
        "Leave blank to keep existing secret".to_string()
    } else {
        String::new()
    };

    rsx! {
        div { class: "space-y-6",
            PageHeader {
                title: t("pasion.upstream_providers.title"),
                description: t("pasion.upstream_providers.description"),
                Button {
                    variant: ButtonVariant::Outline,
                    onclick: move |_| providers_data.restart(),
                    "Refresh"
                }
                Button {
                    onclick: move |_| {
                        form.set(ProviderForm::for_create());
                        editing_provider_id.set(None);
                        loaded_provider.set(None);
                        form_open.set(true);
                    },
                    Icon { name: "plus".to_string(), class: "h-4 w-4 mr-2".to_string() }
                    "Add Provider"
                }
            }

            match &*providers_data.read() {
                Some(Ok(providers)) => rsx! {
                    div { class: "rounded-md border",
                        Table {
                            TableHeader {
                                TableRow {
                                    TableHead { "Name" }
                                    TableHead { "Issuer" }
                                    TableHead { "Source" }
                                    TableHead { "Status" }
                                    TableHead { class: "text-right".to_string(), "Actions" }
                                }
                            }
                            TableBody {
                                if providers.is_empty() {
                                    EmptyRow { colspan: 5, message: t("pasion.upstream_providers.empty") }
                                } else {
                                    for provider in providers.iter() {
                                        {
                                            let pid = provider.id.clone();
                                            let pid_edit = pid.clone();
                                            let pid_toggle = pid.clone();
                                            let pid_delete = pid.clone();
                                            let display_name = provider
                                                .human_name
                                                .clone()
                                                .filter(|s| !s.is_empty())
                                                .or_else(|| {
                                                    provider
                                                        .brand_name
                                                        .clone()
                                                        .filter(|s| !s.is_empty())
                                                })
                                                .or_else(|| {
                                                    provider
                                                        .issuer
                                                        .clone()
                                                        .filter(|s| !s.is_empty())
                                                })
                                                .unwrap_or_else(|| pid.clone());
                                            let issuer = provider.issuer.clone().unwrap_or_else(|| "-".into());
                                            let source = provider.source.clone().unwrap_or_else(|| "manual".into());
                                            let is_config = source == "config";
                                            let is_disabled = provider.disabled_at.is_some();
                                            let busy = toggling.read().as_deref() == Some(pid.as_str());
                                            let is_loading_current =
                                                loading_provider_id.as_deref() == Some(pid.as_str());

                                            rsx! {
                                                TableRow { key: "{pid}",
                                                    TableCell {
                                                        div { class: "space-y-1",
                                                            span { class: "font-medium", "{display_name}" }
                                                            if let Some(brand_name) = provider.brand_name.clone() {
                                                                if !brand_name.is_empty() {
                                                                    div { class: "text-xs text-muted-foreground", "{brand_name}" }
                                                                }
                                                            }
                                                        }
                                                    }
                                                    TableCell {
                                                        span { class: "text-xs text-muted-foreground font-mono", "{issuer}" }
                                                    }
                                                    TableCell {
                                                        if is_config {
                                                            Badge { variant: BadgeVariant::Outline, "config" }
                                                        } else {
                                                            Badge { variant: BadgeVariant::Secondary, "manual" }
                                                        }
                                                    }
                                                    TableCell {
                                                        if is_disabled {
                                                            Badge { variant: BadgeVariant::Secondary, "Disabled" }
                                                        } else {
                                                            Badge { variant: BadgeVariant::Success, "Enabled" }
                                                        }
                                                    }
                                                    TableCell { class: "text-right space-x-1".to_string(),
                                                        if !is_config {
                                                            Button {
                                                                variant: ButtonVariant::Ghost,
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
                                                                            Err(e) => show_toast(&format!("Failed: {}", e.message), ToastVariant::Error),
                                                                        }
                                                                        loading_provider.set(None);
                                                                    });
                                                                },
                                                                if is_loading_current {
                                                                    Spinner { class: "mr-2".to_string() }
                                                                }
                                                                {t("common.edit")}
                                                            }
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
                                                                            show_toast(
                                                                                if is_disabled { "Provider enabled" } else { "Provider disabled" },
                                                                                ToastVariant::Success,
                                                                            );
                                                                            providers_data.restart();
                                                                        }
                                                                        Err(e) => show_toast(&format!("Failed: {}", e.message), ToastVariant::Error),
                                                                    }
                                                                    toggling.set(None);
                                                                });
                                                            },
                                                            if is_disabled { "Enable" } else { "Disable" }
                                                        }
                                                        if !is_config {
                                                            Button {
                                                                variant: ButtonVariant::Ghost,
                                                                size: ButtonSize::Sm,
                                                                class: "text-destructive hover:text-destructive".to_string(),
                                                                onclick: move |_| {
                                                                    delete_target.set(Some(pid_delete.clone()));
                                                                    delete_open.set(true);
                                                                },
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

        if *form_open.read() {
            div { class: "fixed inset-0 z-50 flex items-center justify-center",
                div {
                    class: "fixed inset-0 bg-black/80",
                    onclick: move |_| {
                        if !is_saving {
                            form_open.set(false);
                            editing_provider_id.set(None);
                            loaded_provider.set(None);
                        }
                    },
                }
                div { class: "relative z-50 w-full max-w-2xl rounded-lg border bg-background p-6 shadow-lg max-h-[90vh] overflow-y-auto",
                    h2 { class: "text-lg font-semibold",
                        if is_editing { "Edit Upstream Provider" } else { "Add Upstream Provider" }
                    }
                    p { class: "text-sm text-muted-foreground mb-4",
                        if is_editing {
                            "Update the provider metadata and advanced settings. Leave client secret blank to keep the current secret."
                        } else {
                            "Configure an OIDC / OAuth2 identity provider that users can sign in with."
                        }
                    }
                    div { class: "grid grid-cols-1 sm:grid-cols-2 gap-3",
                        div { class: "space-y-1 sm:col-span-2",
                            Label { "Issuer" }
                            Input {
                                placeholder: "https://accounts.google.com".to_string(),
                                value: form.read().issuer.clone(),
                                oninput: move |e: FormEvent| form.write().issuer = e.value(),
                                disabled: is_saving,
                            }
                        }
                        div { class: "space-y-1",
                            Label { "Human name" }
                            Input {
                                placeholder: "Google".to_string(),
                                value: form.read().human_name.clone(),
                                oninput: move |e: FormEvent| form.write().human_name = e.value(),
                                disabled: is_saving,
                            }
                        }
                        div { class: "space-y-1",
                            Label { "Brand" }
                            Input {
                                placeholder: "google".to_string(),
                                value: form.read().brand_name.clone(),
                                oninput: move |e: FormEvent| form.write().brand_name = e.value(),
                                disabled: is_saving,
                            }
                        }
                        div { class: "space-y-1",
                            Label { "Client ID" }
                            Input {
                                value: form.read().client_id.clone(),
                                oninput: move |e: FormEvent| form.write().client_id = e.value(),
                                disabled: is_saving,
                            }
                        }
                        div { class: "space-y-1",
                            Label { "Client secret" }
                            Input {
                                r#type: "password".to_string(),
                                placeholder: client_secret_placeholder,
                                value: form.read().client_secret.clone(),
                                oninput: move |e: FormEvent| form.write().client_secret = e.value(),
                                disabled: is_saving,
                            }
                        }
                        div { class: "space-y-1 sm:col-span-2",
                            Label { "Scope" }
                            Input {
                                value: form.read().scope.clone(),
                                oninput: move |e: FormEvent| form.write().scope = e.value(),
                                disabled: is_saving,
                            }
                        }
                        div { class: "space-y-1",
                            Label { "Token auth method" }
                            Input {
                                value: form.read().token_endpoint_auth_method.clone(),
                                oninput: move |e: FormEvent| form.write().token_endpoint_auth_method = e.value(),
                                disabled: is_saving,
                            }
                            p { class: "text-xs text-muted-foreground", "e.g. client_secret_basic, client_secret_post, none" }
                        }
                        div { class: "space-y-1",
                            Label { "ID token alg" }
                            Input {
                                value: form.read().id_token_signed_response_alg.clone(),
                                oninput: move |e: FormEvent| form.write().id_token_signed_response_alg = e.value(),
                                disabled: is_saving,
                            }
                        }
                        div { class: "space-y-1",
                            Label { "Discovery mode" }
                            Input {
                                value: form.read().discovery_mode.clone(),
                                oninput: move |e: FormEvent| form.write().discovery_mode = e.value(),
                                disabled: is_saving,
                            }
                            p { class: "text-xs text-muted-foreground", "oidc | insecure | disabled" }
                        }
                        div { class: "space-y-1",
                            Label { "PKCE mode" }
                            Input {
                                value: form.read().pkce_mode.clone(),
                                oninput: move |e: FormEvent| form.write().pkce_mode = e.value(),
                                disabled: is_saving,
                            }
                            p { class: "text-xs text-muted-foreground", "auto | s256 | disabled" }
                        }
                        div { class: "flex items-center gap-2 sm:col-span-2",
                            input {
                                r#type: "checkbox",
                                checked: form.read().fetch_userinfo,
                                oninput: move |e: FormEvent| {
                                    form.write().fetch_userinfo = e.value() == "true" || e.value() == "on";
                                },
                                disabled: is_saving,
                            }
                            Label { "Fetch userinfo endpoint" }
                        }
                        div { class: "space-y-1 sm:col-span-2",
                            Label { "Claims imports (JSON, optional)" }
                            textarea {
                                class: "flex min-h-[120px] w-full rounded-md border border-input bg-background px-3 py-2 text-sm font-mono focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                                value: form.read().claims_imports.clone(),
                                oninput: move |e: FormEvent| form.write().claims_imports = e.value(),
                                disabled: is_saving,
                            }
                            p { class: "text-xs text-muted-foreground",
                                "Optional. JSON object — see the pasion docs for the claims-import schema."
                            }
                        }
                    }
                    div { class: "flex flex-col-reverse sm:flex-row sm:justify-end sm:space-x-2 mt-6",
                        Button {
                            variant: ButtonVariant::Outline,
                            disabled: is_saving,
                            onclick: move |_| {
                                form_open.set(false);
                                editing_provider_id.set(None);
                                loaded_provider.set(None);
                            },
                            "Cancel"
                        }
                        Button {
                            disabled: is_saving,
                            onclick: handle_save,
                            if is_saving { Spinner { class: "mr-2".to_string() } }
                            if is_editing { "Save" } else { "Create" }
                        }
                    }
                }
            }
        }

        ConfirmDialog {
            open: *delete_open.read(),
            title: "Delete provider".to_string(),
            description: format!(
                "Permanently delete provider '{}'? Users linked to this provider will lose the ability to sign in with it.",
                delete_target.read().clone().unwrap_or_default()
            ),
            confirm_text: "Delete".to_string(),
            destructive: true,
            on_confirm: handle_delete_confirm,
            on_cancel: move |_| { delete_open.set(false); delete_target.set(None); },
        }
    }
}
