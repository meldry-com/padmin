use super::users::operation;
use crate::api::{reports, rooms, server_info, users};
use crate::ui::widgets as w;

use std::result::Result;

pub fn render(scope: &glory::ui::Scope, _: &crate::router::Route) -> glory::ui::View {
    let summary = scope.cage(String::from("Loading server statistics…"));
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
    let feature_summary = scope.cage(String::new());
    let metrics = scope.cage(vec![String::from("—"); 6]);
    let load = move || async move {
        // A failure stays visible next to the metric; one unavailable endpoint
        // does not hide the successful results from other endpoints.
        let (version, all, active, rooms, reports, features) = futures::join!(
            server_info::get_server_version(),
            users::get_user_count(),
            users::get_active_user_count(),
            rooms::get_room_count(),
            reports::get_report_count(),
            server_info::get_supported_features()
        );
        feature_summary.revise(|v| {
            *v = match &features {
                Ok(features) => format!(
                    "{}\n{}",
                    features.versions.join(", "),
                    features
                        .unstable_features
                        .iter()
                        .filter(|(_, enabled)| **enabled)
                        .map(|(key, _)| key.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                Err(e) => format!("Unable to load server features: {e}"),
            }
        });
        metrics.revise(|values| {
            *values = vec![
                version
                    .as_ref()
                    .map(String::clone)
                    .unwrap_or_else(|_| "Unavailable".into()),
                all.as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|_| "—".into()),
                rooms
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|_| "—".into()),
                reports
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|_| "—".into()),
                active
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|_| "—".into()),
                format!("{:.0}ms", crate::utils::perf::average_latency()),
            ]
        });
        fn metric<T: std::fmt::Display>(
            name: &str,
            value: Result<T, crate::utils::error::HttpError>,
        ) -> String {
            match value {
                Ok(v) => format!("{name}: {v}"),
                Err(e) => format!("{name}: failed — {e}"),
            }
        }
        summary.revise(|v| {
            *v = [
                metric("Server version", version),
                metric("Users", all),
                metric("Active users", active),
                metric("Rooms", rooms),
                metric("Reports", reports),
                metric(
                    "Matrix versions / enabled server features",
                    features.map(|v| {
                        let mut enabled: Vec<_> = v
                            .unstable_features
                            .iter()
                            .filter(|(_, enabled)| **enabled)
                            .map(|(name, _)| name.clone())
                            .collect();
                        enabled.sort();
                        enabled.truncate(12);
                        format!("{}\n{}", v.versions.join(", "), enabled.join(", "))
                    }),
                ),
                format!(
                    "Average API latency: {:.0}ms",
                    crate::utils::perf::average_latency()
                ),
            ]
            .join("\n")
        });
        Ok("Statistics refreshed".into())
    };
    let initial = load();
    busy.revise(|value| *value = true);
    w::spawn(scope, async move {
        let result = initial.await;
        message.revise(|value| {
            *value = result
                .err()
                .map(|e: crate::utils::error::HttpError| e.to_string())
                .unwrap_or_default()
        });
        busy.revise(|value| *value = false);
    });
    let cells = [
        ("Server version", "Server online"),
        ("Total users", "Total registered users"),
        ("Total rooms", "Total rooms on server"),
        ("Total reports", "Pending reports"),
        ("Active users", "Non-guest, non-deactivated"),
        ("API latency", "Average API response"),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (name, description))| {
        glory::ui::view().class("dashboard-metric").fill((
            w::label(name).class("metric-title"),
            glory::ui::text()
                .class("metric-value")
                .content(metrics.map(move |values| values[index].clone())),
            w::label(description).class("metric-description"),
        ))
    })
    .collect::<Vec<_>>();
    w::page("Dashboard").class("page dashboard-page").fill((
        w::label("Welcome to Palpo Admin").class("dashboard-greeting"),
        glory::ui::view()
            .class("dashboard-stats glass-panel")
            .fill(cells),
        glory::ui::view()
            .class("dashboard-features glass-panel")
            .fill((
                w::label("Server features").kind(glory::render::TextKind::Heading(2)),
                w::label("Supported Matrix specification versions and unstable features")
                    .class("login-subtitle"),
                w::status(feature_summary),
            )),
        glory::ui::view().class("dashboard-actions").fill((
            operation(scope, "Refresh statistics", busy, message, None, load),
            w::action("Manage users", || crate::ui::navigate("/users")),
            w::action("Manage rooms", || crate::ui::navigate("/rooms")),
            w::action("Review reports", || crate::ui::navigate("/reports")),
        )),
        w::status(message),
    ))
}
