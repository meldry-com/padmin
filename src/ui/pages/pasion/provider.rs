use super::*;
use serde_json::{Map, Value};
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

fn validate_modes(
    form: &ProviderForm,
    original: Option<&PasionUpstreamProvider>,
) -> Result<(), String> {
    let checks: &[(&str, &str, Option<&str>, &[&str])] = &[
        (
            "token_endpoint_auth_method",
            &form.token_endpoint_auth_method,
            original.and_then(|p| p.token_endpoint_auth_method.as_deref()),
            &[
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
            ],
        ),
        (
            "discovery_mode",
            &form.discovery_mode,
            original.and_then(|p| p.discovery_mode.as_deref()),
            &["oidc", "insecure", "disabled"],
        ),
        (
            "pkce_mode",
            &form.pkce_mode,
            original.and_then(|p| p.pkce_mode.as_deref()),
            &["auto", "s256", "disabled"],
        ),
        (
            "response_mode",
            &form.response_mode,
            original.and_then(|p| p.response_mode.as_deref()),
            &["", "query", "form_post"],
        ),
        (
            "on_backchannel_logout",
            &form.on_backchannel_logout,
            original.and_then(|p| p.on_backchannel_logout.as_deref()),
            &["do_nothing", "logout_browser_only", "logout_all"],
        ),
    ];
    for (key, value, previous, allowed) in checks {
        let value = value.trim();
        if !allowed.contains(&value) && Some(value) != *previous {
            return Err(format!("{key} must be one of: {}", allowed.join(", ")));
        }
    }
    Ok(())
}

fn create_provider_body(form: &ProviderForm) -> Result<Value, String> {
    validate_modes(form, None)?;
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
    validate_modes(form, Some(original))?;
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

pub(super) fn render(scope: &Scope) -> View {
    let data = scope.cage(Vec::<PasionUpstreamProvider>::new());
    let (visible, pagination) = collection_window(scope, data, "providers");
    let original = scope.cage(None::<PasionUpstreamProvider>);
    let busy = scope.cage(false);
    let loading = scope.cage(false);
    let message = scope.cage(String::new());
    let s = scope.clone();
    let refresh =
        Rc::new(move || load(&s, loading, message, data, pasion_get_upstream_providers()));
    refresh();
    let defaults = ProviderForm::for_create();
    let mut inputs = Vec::new();
    let (issuer, input) = field(scope, "issuer");
    issuer.revise(|slot| *slot = defaults.issuer);
    inputs.push(view().fill((label("issuer"), input)));
    let (human_name, input) = field(scope, "human name");
    human_name.revise(|slot| *slot = defaults.human_name);
    inputs.push(view().fill((label("human name"), input)));
    let (brand_name, input) = field(scope, "brand name");
    brand_name.revise(|slot| *slot = defaults.brand_name);
    inputs.push(view().fill((label("brand name"), input)));
    let (client_id, input) = field(scope, "client id");
    client_id.revise(|slot| *slot = defaults.client_id);
    inputs.push(view().fill((label("client id"), input)));
    let (client_secret, input) = field(scope, "client secret");
    client_secret.revise(|slot| *slot = defaults.client_secret);
    inputs.push(view().fill((label("client secret"), input.purpose(Purpose::Password))));
    let (provider_scope, input) = field(scope, "scope");
    provider_scope.revise(|slot| *slot = defaults.scope);
    inputs.push(view().fill((label("scope"), input)));
    let (token_endpoint_auth_method, input) = field(scope, "token endpoint auth method");
    token_endpoint_auth_method.revise(|slot| *slot = defaults.token_endpoint_auth_method);
    inputs.push(view().fill((label("token endpoint auth method"), input)));
    let (token_endpoint_signing_alg, input) = field(scope, "token endpoint signing alg");
    token_endpoint_signing_alg.revise(|slot| *slot = defaults.token_endpoint_signing_alg);
    inputs.push(view().fill((label("token endpoint signing alg"), input)));
    let (id_token_signed_response_alg, input) = field(scope, "id token signed response alg");
    id_token_signed_response_alg.revise(|slot| *slot = defaults.id_token_signed_response_alg);
    inputs.push(view().fill((label("id token signed response alg"), input)));
    let (userinfo_signed_response_alg, input) = field(scope, "userinfo signed response alg");
    userinfo_signed_response_alg.revise(|slot| *slot = defaults.userinfo_signed_response_alg);
    inputs.push(view().fill((label("userinfo signed response alg"), input)));
    let (discovery_mode, input) = field(scope, "discovery mode");
    discovery_mode.revise(|slot| *slot = defaults.discovery_mode);
    inputs.push(view().fill((label("discovery mode"), input)));
    let (pkce_mode, input) = field(scope, "pkce mode");
    pkce_mode.revise(|slot| *slot = defaults.pkce_mode);
    inputs.push(view().fill((label("pkce mode"), input)));
    let (response_mode, input) = field(scope, "response mode");
    response_mode.revise(|slot| *slot = defaults.response_mode);
    inputs.push(view().fill((label("response mode"), input)));
    let (authorization_endpoint_override, input) = field(scope, "authorization endpoint override");
    authorization_endpoint_override.revise(|slot| *slot = defaults.authorization_endpoint_override);
    inputs.push(view().fill((label("authorization endpoint override"), input)));
    let (token_endpoint_override, input) = field(scope, "token endpoint override");
    token_endpoint_override.revise(|slot| *slot = defaults.token_endpoint_override);
    inputs.push(view().fill((label("token endpoint override"), input)));
    let (userinfo_endpoint_override, input) = field(scope, "userinfo endpoint override");
    userinfo_endpoint_override.revise(|slot| *slot = defaults.userinfo_endpoint_override);
    inputs.push(view().fill((label("userinfo endpoint override"), input)));
    let (jwks_uri_override, input) = field(scope, "jwks uri override");
    jwks_uri_override.revise(|slot| *slot = defaults.jwks_uri_override);
    inputs.push(view().fill((label("jwks uri override"), input)));
    let (additional_authorization_parameters, input) =
        field(scope, "additional authorization parameters");
    additional_authorization_parameters
        .revise(|slot| *slot = defaults.additional_authorization_parameters);
    inputs.push(view().fill((
        label("additional authorization parameters"),
        input.multiline(true),
    )));
    let (ui_order, input) = field(scope, "ui order");
    ui_order.revise(|slot| *slot = defaults.ui_order);
    inputs.push(view().fill((label("ui order"), input)));
    let (on_backchannel_logout, input) = field(scope, "on backchannel logout");
    on_backchannel_logout.revise(|slot| *slot = defaults.on_backchannel_logout);
    inputs.push(view().fill((label("on backchannel logout"), input)));
    let (claims_imports, input) = field(scope, "claims imports");
    claims_imports.revise(|slot| *slot = defaults.claims_imports);
    inputs.push(view().fill((label("claims imports"), input.multiline(true))));
    let (clear_client_secret, input) =
        check_field(scope, "clear client secret", defaults.clear_client_secret);
    inputs.push(view().fill(input));
    let (fetch_userinfo, input) = check_field(scope, "fetch userinfo", defaults.fetch_userinfo);
    inputs.push(view().fill(input));
    let (forward_login_hint, input) =
        check_field(scope, "forward login hint", defaults.forward_login_hint);
    inputs.push(view().fill(input));

    let s = scope.clone();
    let rows_scope = s.clone();
    let reset = action("New provider / reset form", move || {
        original.revise(|slot| *slot = None);
        let form = ProviderForm::for_create();
        issuer.revise(|slot| *slot = form.issuer);
        human_name.revise(|slot| *slot = form.human_name);
        brand_name.revise(|slot| *slot = form.brand_name);
        client_id.revise(|slot| *slot = form.client_id);
        client_secret.revise(|slot| *slot = form.client_secret);
        provider_scope.revise(|slot| *slot = form.scope);
        token_endpoint_auth_method.revise(|slot| *slot = form.token_endpoint_auth_method);
        token_endpoint_signing_alg.revise(|slot| *slot = form.token_endpoint_signing_alg);
        id_token_signed_response_alg.revise(|slot| *slot = form.id_token_signed_response_alg);
        userinfo_signed_response_alg.revise(|slot| *slot = form.userinfo_signed_response_alg);
        discovery_mode.revise(|slot| *slot = form.discovery_mode);
        pkce_mode.revise(|slot| *slot = form.pkce_mode);
        response_mode.revise(|slot| *slot = form.response_mode);
        authorization_endpoint_override.revise(|slot| *slot = form.authorization_endpoint_override);
        token_endpoint_override.revise(|slot| *slot = form.token_endpoint_override);
        userinfo_endpoint_override.revise(|slot| *slot = form.userinfo_endpoint_override);
        jwks_uri_override.revise(|slot| *slot = form.jwks_uri_override);
        additional_authorization_parameters
            .revise(|slot| *slot = form.additional_authorization_parameters);
        ui_order.revise(|slot| *slot = form.ui_order);
        on_backchannel_logout.revise(|slot| *slot = form.on_backchannel_logout);
        claims_imports.revise(|slot| *slot = form.claims_imports);
        clear_client_secret.revise(|slot| *slot = form.clear_client_secret);
        fetch_userinfo.revise(|slot| *slot = form.fetch_userinfo);
        forward_login_hint.revise(|slot| *slot = form.forward_login_hint);
    });
    let save = action("Save provider", move || {
        let form = ProviderForm {
            issuer: issuer.get(),
            human_name: human_name.get(),
            brand_name: brand_name.get(),
            client_id: client_id.get(),
            client_secret: client_secret.get(),
            scope: provider_scope.get(),
            token_endpoint_auth_method: token_endpoint_auth_method.get(),
            token_endpoint_signing_alg: token_endpoint_signing_alg.get(),
            id_token_signed_response_alg: id_token_signed_response_alg.get(),
            userinfo_signed_response_alg: userinfo_signed_response_alg.get(),
            discovery_mode: discovery_mode.get(),
            pkce_mode: pkce_mode.get(),
            response_mode: response_mode.get(),
            authorization_endpoint_override: authorization_endpoint_override.get(),
            token_endpoint_override: token_endpoint_override.get(),
            userinfo_endpoint_override: userinfo_endpoint_override.get(),
            jwks_uri_override: jwks_uri_override.get(),
            additional_authorization_parameters: additional_authorization_parameters.get(),
            ui_order: ui_order.get(),
            on_backchannel_logout: on_backchannel_logout.get(),
            claims_imports: claims_imports.get(),
            clear_client_secret: clear_client_secret.get(),
            fetch_userinfo: fetch_userinfo.get(),
            forward_login_hint: forward_login_hint.get(),
        };
        let stored = original.get();
        if stored
            .as_ref()
            .is_some_and(|p| p.source.as_deref() == Some("config"))
        {
            message.revise(|slot| *slot = "Config-sourced providers are read-only".into());
            return;
        }
        let payload = match &stored {
            Some(p) => update_provider_body(&form, p),
            None => create_provider_body(&form).map(Some),
        };
        let payload = match payload {
            Ok(Some(p)) => p,
            Ok(None) => {
                message.revise(|slot| *slot = "No changes".into());
                return;
            }
            Err(e) => {
                message.revise(|slot| *slot = e);
                return;
            }
        };
        if !confirm("Save this upstream provider configuration?") {
            return;
        }
        submit(&s, busy, message, async move {
            let provider = if let Some(p) = stored {
                pasion_update_upstream_provider(&p.id, payload).await?
            } else {
                pasion_create_upstream_provider(payload).await?
            };
            let _ = original.try_revise(|slot| *slot = Some(provider));
            let _ = client_secret.try_revise(|slot| *slot = String::new());
            let _ = clear_client_secret.try_revise(|slot| *slot = false);
            let loaded = pasion_get_upstream_providers().await?;
            let _ = data.try_revise(|slot| *slot = loaded);
            Ok("Provider saved".into())
        });
    })
    .enabled(busy.map(|b| !b));
    page("Upstream OAuth providers").fill((
        label("Stored client secrets are write-only. An empty secret keeps the stored secret; clear client secret removes it. Config-sourced providers are read-only."),
        action("Refresh",move||refresh()).enabled(loading.map(|b|!b)),status(message),
        pagination,each(visible,|r|r.id.clone(),move|r,_|{
            let id=r.get().id;let edit=id.clone();let toggle=id.clone();let s=rows_scope.clone();let es=s.clone();let ts=s.clone();
            view().class("record").fill((text().content(r.map(|r|format!("{} · {} · brand {} · issuer {} · source {} · {} · secret {}",r.id,r.human_name.as_deref().unwrap_or("unnamed"),r.brand_name.as_deref().unwrap_or("—"),r.issuer.as_deref().unwrap_or("—"),r.source.as_deref().unwrap_or("manual"),if r.disabled_at.is_some(){"disabled"}else{"enabled"},if r.has_client_secret{"stored"}else{"absent"}))),
                action("Load details / edit",move||{let id=edit.clone();submit(&es,busy,message,async move{
                    let provider=pasion_get_upstream_provider(&id).await?;let form=ProviderForm::from_provider(&provider);
                    let _=issuer.try_revise(|slot| *slot = form.issuer);
                    let _=human_name.try_revise(|slot| *slot = form.human_name);
                    let _=brand_name.try_revise(|slot| *slot = form.brand_name);
                    let _=client_id.try_revise(|slot| *slot = form.client_id);
                    let _=client_secret.try_revise(|slot| *slot = form.client_secret);
                    let _=provider_scope.try_revise(|slot| *slot = form.scope);
                    let _=token_endpoint_auth_method.try_revise(|slot| *slot = form.token_endpoint_auth_method);
                    let _=token_endpoint_signing_alg.try_revise(|slot| *slot = form.token_endpoint_signing_alg);
                    let _=id_token_signed_response_alg.try_revise(|slot| *slot = form.id_token_signed_response_alg);
                    let _=userinfo_signed_response_alg.try_revise(|slot| *slot = form.userinfo_signed_response_alg);
                    let _=discovery_mode.try_revise(|slot| *slot = form.discovery_mode);
                    let _=pkce_mode.try_revise(|slot| *slot = form.pkce_mode);
                    let _=response_mode.try_revise(|slot| *slot = form.response_mode);
                    let _=authorization_endpoint_override.try_revise(|slot| *slot = form.authorization_endpoint_override);
                    let _=token_endpoint_override.try_revise(|slot| *slot = form.token_endpoint_override);
                    let _=userinfo_endpoint_override.try_revise(|slot| *slot = form.userinfo_endpoint_override);
                    let _=jwks_uri_override.try_revise(|slot| *slot = form.jwks_uri_override);
                    let _=additional_authorization_parameters.try_revise(|slot| *slot = form.additional_authorization_parameters);
                    let _=ui_order.try_revise(|slot| *slot = form.ui_order);
                    let _=on_backchannel_logout.try_revise(|slot| *slot = form.on_backchannel_logout);
                    let _=claims_imports.try_revise(|slot| *slot = form.claims_imports);
                    let _=clear_client_secret.try_revise(|slot| *slot = form.clear_client_secret);
                    let _=fetch_userinfo.try_revise(|slot| *slot = form.fetch_userinfo);
                    let _=forward_login_hint.try_revise(|slot| *slot = form.forward_login_hint);

                    let _=original.try_revise(|slot| *slot = Some(provider));Ok("Provider details loaded".into())
                });}),
                action("Enable / disable",move||{
                    let row=r.get();if row.source.as_deref()==Some("config"){message.revise(|slot| *slot = "Config provider is read-only".into());return;}
                    if !confirm("Change provider availability?"){return;}let id=toggle.clone();let enable=row.disabled_at.is_some();submit(&ts,busy,message,async move{if enable{pasion_enable_upstream_provider(&id).await?}else{pasion_disable_upstream_provider(&id).await?}let loaded = pasion_get_upstream_providers().await?; let _ = data.try_revise(|slot| *slot = loaded);Ok("Provider availability updated".into())});
                }).enabled(r.map(|r|r.source.as_deref()!=Some("config"))),
                action("Delete provider",move||{
                    if r.get().source.as_deref()==Some("config"){message.revise(|slot| *slot = "Config provider is read-only".into());return;}
                    if !confirm("Delete this upstream provider? Existing account links may be affected."){return;}let id=id.clone();submit(&s,busy,message,async move{pasion_delete_upstream_provider(&id).await?;let _=data.try_revise(|rows|rows.retain(|r|r.id!=id));if original.try_get().ok().flatten().is_some_and(|p|p.id==id){let _=original.try_revise(|slot| *slot = None);}Ok("Provider deleted".into())});
                }).enabled(r.map(|r|r.source.as_deref()!=Some("config")))
            ))
        }),
        view().name("Provider configuration").fill((label("Provider configuration"),text().content(original.map(|p|p.as_ref().map(|p|format!("Editing {}",p.id)).unwrap_or_else(||"Creating a new provider".into()))),reset,view().fill(inputs),save))
    ))
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
    fn mode_changes_are_validated_but_unknown_stored_modes_are_preserved() {
        let mut provider = stored_github();
        provider.pkce_mode = Some("future-server-mode".into());
        let mut form = ProviderForm::from_provider(&provider);
        assert_eq!(update_provider_body(&form, &provider), Ok(None));
        form.pkce_mode = "typo".into();
        assert!(update_provider_body(&form, &provider).is_err());
        form.pkce_mode = "s256".into();
        assert_eq!(
            update_provider_body(&form, &provider),
            Ok(Some(json!({"pkce_mode":"s256"})))
        );
    }
}
