use super::users::operation;
use crate::api::{reports, rooms, server_info, users};
use crate::ui::widgets as w;

use std::result::Result;

pub fn render(scope: &glory::ui::Scope, _: &crate::router::Route) -> glory::ui::View {
    let summary = scope.cage(String::from("Loading server statistics…"));
    let busy = scope.cage(false);
    let message = scope.cage(String::new());
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
    w::submit(scope, busy, message, load());
    w::page("Dashboard").fill((
        w::label("Welcome to Palpo Admin"),
        w::label("Server features").kind(glory::render::TextKind::Heading(2)),
        w::status(summary),
        operation(scope, "Refresh statistics", busy, message, None, load),
        w::status(message),
        w::action("Manage users", || crate::ui::navigate("/users")),
        w::action("Manage rooms", || crate::ui::navigate("/rooms")),
        w::action("Review reports", || crate::ui::navigate("/reports")),
    ))
}
