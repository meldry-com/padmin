use dioxus::prelude::*;

use super::button::{Button, ButtonVariant};

#[component]
pub fn ConfirmDialog(
    open: bool,
    title: String,
    description: String,
    #[props(default = "Confirm".to_string())] confirm_text: String,
    #[props(default = "Cancel".to_string())] cancel_text: String,
    #[props(default = false)] destructive: bool,
    on_confirm: EventHandler<()>,
    on_cancel: EventHandler<()>,
) -> Element {
    if !open {
        return rsx! {};
    }

    rsx! {
        div { class: "fixed inset-0 z-50 flex items-center justify-center",
            // Backdrop
            div {
                class: "fixed inset-0 bg-black/80",
                onclick: move |_| on_cancel.call(()),
            }
            // Dialog
            div { class: "relative z-50 w-full max-w-lg rounded-lg border glass-panel p-6 shadow-lg",
                div { class: "flex flex-col space-y-2 text-center sm:text-left",
                    h2 { class: "text-lg font-semibold", "{title}" }
                    p { class: "text-sm text-muted-foreground", "{description}" }
                }
                div { class: "flex flex-col-reverse sm:flex-row sm:justify-end sm:space-x-2 mt-4",
                    Button {
                        variant: ButtonVariant::Outline,
                        onclick: move |_| on_cancel.call(()),
                        "{cancel_text}"
                    }
                    Button {
                        variant: if destructive { ButtonVariant::Destructive } else { ButtonVariant::Default },
                        onclick: move |_| on_confirm.call(()),
                        "{confirm_text}"
                    }
                }
            }
        }
    }
}
