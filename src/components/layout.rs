use dioxus::prelude::*;

use crate::api::auth;
use crate::components::header::AppHeader;
use crate::components::keyboard_shortcuts::KeyboardShortcuts;
use crate::components::sidebar::AppSidebar;
use crate::components::ui::toast::Toaster;
use crate::router::Route;

#[component]
pub fn AppLayout(children: Element) -> Element {
    let collapsed = use_signal(|| false);
    let mut mobile_sidebar_open = use_signal(|| false);
    let nav = use_navigator();

    // Multi-tab session sync: periodically check if auth state changed in another tab.
    // use_hook keeps the Interval alive for the component's lifetime and drops it on unmount.
    use_hook(move || {
        std::rc::Rc::new(gloo_timers::callback::Interval::new(2_000, move || {
            if !auth::is_authenticated() {
                nav.replace(Route::LoginPage {});
            }
        }))
    });

    let mobile_sidebar_state = *mobile_sidebar_open.read();

    rsx! {
        div { class: "flex h-screen overflow-hidden",
            div {
                class: if mobile_sidebar_state {
                    "sidebar-backdrop sidebar-backdrop-open"
                } else {
                    "sidebar-backdrop"
                },
                onclick: move |_| mobile_sidebar_open.set(false),
            }
            AppSidebar {
                collapsed,
                mobile_open: mobile_sidebar_open,
            }
            div { class: "flex flex-1 flex-col overflow-hidden",
                AppHeader {
                    collapsed,
                    mobile_sidebar_open,
                }
                main { class: "flex-1 overflow-auto p-4 md:p-6",
                    {children}
                }
            }
        }
        Toaster {}
        KeyboardShortcuts {}
    }
}
