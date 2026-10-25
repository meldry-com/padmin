#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    pub path: String,
    pub params: Vec<String>,
}
pub const PATHS: &[&str] = &[
    "/login",
    "/oauth/callback",
    "/",
    "/users",
    "/users/create",
    "/users/known",
    "/users/known/:user_id",
    "/users/:user_id",
    "/rooms",
    "/rooms/:room_id",
    "/media",
    "/reports",
    "/reports/:report_id",
    "/destinations",
    "/destinations/:destination_id",
    "/registration-tokens",
    "/appservices",
    "/auth-status",
    "/server-status",
    "/server-actions",
    "/server-notices",
    "/server-notifications",
    "/settings/notifications",
    "/pasion/accounts",
    "/pasion/accounts/:user_id",
    "/pasion/audit-log",
    "/pasion/oauth2-sessions",
    "/pasion/personal-sessions",
    "/pasion/connector-health",
    "/pasion/upstream-providers",
    "/pasion/upstream-links",
    "/pasion/notification-channels",
    "/pasion/notification-templates",
];
pub fn parse(url: &str, prefix: &str) -> Route {
    let path = url.split(['?', '#']).next().unwrap_or("/");
    let prefix = prefix.trim_end_matches('/');
    let Some(path) = path
        .strip_prefix(prefix)
        .filter(|p| prefix.is_empty() || p.is_empty() || p.starts_with('/'))
    else {
        return Route {
            path: "/*".into(),
            params: vec![],
        };
    };
    let path = if path.is_empty() { "/" } else { path };
    let parts: Vec<_> = path.trim_matches('/').split('/').collect();
    for pattern in PATHS {
        let expected: Vec<_> = pattern.trim_matches('/').split('/').collect();
        if expected.len() != parts.len() {
            continue;
        }
        let mut params = Vec::new();
        let matches = expected.iter().zip(&parts).all(|(a, b)| {
            if a.starts_with(':') {
                match urlencoding::decode(b) {
                    Ok(value) if !value.is_empty() => {
                        params.push(value.into_owned());
                        true
                    }
                    _ => false,
                }
            } else {
                a == b
            }
        });
        if matches {
            return Route {
                path: pattern.to_string(),
                params,
            };
        }
    }
    Route {
        path: "/*".into(),
        params: vec![],
    }
}
pub fn reset_admin_cache() {
    crate::ui::invalidate_auth();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn route_boundaries_and_ids() {
        assert_eq!(
            parse("/admin/users/%40a%3As.test", "/admin").params,
            ["@a:s.test"]
        );
        assert_eq!(parse("/administrator/users", "/admin").path, "/*");
        assert_eq!(
            parse("/console/manage/users/create", "/console/manage").path,
            "/users/create"
        );
        assert_eq!(parse("/users/known", "").path, "/users/known");
        for path in PATHS.iter().filter(|p| !p.contains(':')) {
            assert_eq!(&parse(path, "").path, path);
        }
    }
}
