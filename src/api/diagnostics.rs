//! Authentication diagnostics. Every result retains the endpoint and HTTP status.
use crate::api::{auth, server_info};
use gloo_net::http::Request;

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub name: String,
    pub endpoint: String,
    pub outcome: String,
}
fn check(name: &str, endpoint: &str, outcome: impl Into<String>) -> Diagnostic {
    Diagnostic {
        name: name.into(),
        endpoint: endpoint.into(),
        outcome: outcome.into(),
    }
}
pub async fn discover() -> Vec<Diagnostic> {
    let mut out = Vec::new();
    out.push(check(
        "Server version",
        "/_palpo/admin/v1/server_version",
        match server_info::get_server_version_unauthenticated("").await {
            Ok(v) => v,
            Err(e) => e.to_string(),
        },
    ));
    out.push(check(
        "Login methods",
        "/_matrix/client/v3/login",
        match auth::get_login_flows("").await {
            Ok(v) => v
                .iter()
                .map(|v| v.flow_type.clone())
                .collect::<Vec<_>>()
                .join(", "),
            Err(e) => e.to_string(),
        },
    ));
    match auth::get_auth_issuer("").await {
        Ok(value) => {
            if let Some(issuer) = value.get("issuer").and_then(|v| v.as_str()) {
                out.push(check(
                    "Authentication issuer",
                    "/_matrix/client/unstable/org.matrix.msc2965/auth_issuer",
                    issuer,
                ));
                match auth::get_oidc_discovery(issuer).await {
                    Ok(doc) => {
                        for key in [
                            "authorization_endpoint",
                            "token_endpoint",
                            "registration_endpoint",
                            "device_authorization_endpoint",
                        ] {
                            out.push(check(
                                key,
                                issuer,
                                doc.get(key)
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Not advertised"),
                            ));
                        }
                        out.push(check(
                            "Supported scopes",
                            issuer,
                            doc.get("scopes_supported")
                                .map(ToString::to_string)
                                .unwrap_or_else(|| "Not advertised".into()),
                        ));
                    }
                    Err(e) => out.push(check("OIDC discovery", issuer, e.to_string())),
                }
            } else {
                out.push(check(
                    "Authentication issuer",
                    "/_matrix/client/unstable/org.matrix.msc2965/auth_issuer",
                    "No issuer advertised; OIDC discovery skipped",
                ));
            }
        }
        Err(e) => out.push(check(
            "Authentication issuer",
            "/_matrix/client/unstable/org.matrix.msc2965/auth_issuer",
            e.to_string(),
        )),
    }
    out.push(check(
        "Pasion site configuration",
        "/api/admin/v1/site-config",
        match crate::api::pasion::pasion_get_site_config().await {
            Ok(value) => {
                let config = value
                    .get("data")
                    .and_then(|v| v.get("attributes"))
                    .unwrap_or(&value);
                let fields = [
                    "issuer",
                    "server_name",
                    "registration_enabled",
                    "password_login_enabled",
                    "email_verification_required",
                ];
                let known = fields
                    .into_iter()
                    .filter_map(|key| config.get(key).map(|v| format!("{key}: {v}")))
                    .collect::<Vec<_>>();
                if known.is_empty() {
                    "Site configuration available; no recognized public diagnostic flags".into()
                } else {
                    known.join("; ")
                }
            }
            Err(error) => error.to_string(),
        },
    ));
    out
}
pub async fn probe() -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for (name, path) in [
        ("Matrix login", "/_matrix/client/v3/login"),
        ("Well-known discovery", "/.well-known/matrix/client"),
        ("Consent endpoint", "/_matrix/consent"),
    ] {
        match Request::get(path)
            .header("Accept", "application/json")
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status();
                let mut detail = format!("HTTP {status}");
                if path.contains("well-known") && status < 400 {
                    detail.push_str(&match response.json::<serde_json::Value>().await {
                        Ok(json) => format!(
                            "; homeserver: {}; delegated authentication: {}",
                            json.get("m.homeserver").is_some(),
                            json.get("org.matrix.msc2965.authentication")
                                .or_else(|| json.get("m.authentication"))
                                .map(ToString::to_string)
                                .unwrap_or_else(|| "not advertised".into())
                        ),
                        Err(e) => format!("; invalid JSON: {e}"),
                    });
                }
                out.push(check(name, path, detail));
            }
            Err(e) => out.push(check(name, path, format!("Unreachable: {e}"))),
        }
    }
    let path = "/_matrix/client/v3/register";
    match Request::post(path)
        .header("Content-Type", "application/json")
        .body("{}")
    {
        Ok(request) => match request.send().await {
            Ok(response) => {
                let status = response.status();
                let detail = match status {
                    401 => "Registration available; UIA required",
                    403 => "Registration disabled or requires a token",
                    500.. => "Server failure",
                    _ => "Registration endpoint responded",
                };
                out.push(check(
                    "Registration",
                    path,
                    format!("HTTP {status}; {detail}"),
                ));
            }
            Err(e) => out.push(check("Registration", path, format!("Unreachable: {e}"))),
        },
        Err(e) => out.push(check("Registration", path, e.to_string())),
    }
    out
}
