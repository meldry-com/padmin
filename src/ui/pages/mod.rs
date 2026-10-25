pub mod appservices;
pub mod dashboard;
pub mod operations;
pub mod pasion;
pub mod rooms;
pub mod users;
pub fn render(scope: &glory::ui::Scope, route: &crate::router::Route) -> glory::ui::View {
    match route.path.as_str() {
        "/" => dashboard::render(scope, route),
        p if p.starts_with("/users") => users::render(scope, route),
        p if p.starts_with("/rooms") => rooms::render(scope, route),
        "/appservices" => appservices::render(scope, route),
        p if p.starts_with("/pasion/") => pasion::render(scope, route),
        "/*" => super::widgets::page("Page not found"),
        _ => operations::render(scope, route),
    }
}
