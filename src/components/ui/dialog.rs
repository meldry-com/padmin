use dioxus::prelude::*;

use super::button::{Button, ButtonVariant};

/// Width presets for [`Modal`]. Maps to the `max-w-*` Tailwind utility used by
/// the panel.
#[derive(Clone, Copy, PartialEq)]
pub enum ModalSize {
    Md,
    Lg,
    Xl2,
    Xl3,
}

impl ModalSize {
    fn max_width_class(self) -> &'static str {
        match self {
            ModalSize::Md => "max-w-md",
            ModalSize::Lg => "max-w-lg",
            ModalSize::Xl2 => "max-w-2xl",
            ModalSize::Xl3 => "max-w-3xl",
        }
    }
}

/// Generic modal shell: a full-screen centered overlay with a dimmed backdrop
/// and a centered panel. Clicking the backdrop invokes `on_close`; clicks
/// inside the panel do not propagate to the backdrop.
///
/// Renders nothing when `open` is false.
///
/// - `size` selects the panel's `max-w-*` width (default [`ModalSize::Lg`]).
/// - `bg` overrides the panel background class (default `"bg-background"`; pass
///   `"glass-panel"` for the frosted variant).
/// - `panel_class` appends extra classes to the panel (e.g.
///   `"max-h-[80vh] overflow-y-auto"`).
/// - `children` is the panel's content (header, body, footer).
#[component]
pub fn Modal(
    open: bool,
    on_close: EventHandler<()>,
    #[props(default = ModalSize::Lg)] size: ModalSize,
    #[props(default = "bg-background".to_string())] bg: String,
    #[props(default = String::new())] panel_class: String,
    children: Element,
) -> Element {
    if !open {
        return rsx! {};
    }

    let panel_class = format!(
        "relative z-50 w-full {} rounded-lg border {} p-6 shadow-lg {}",
        size.max_width_class(),
        bg,
        panel_class
    );
    let panel_class = panel_class.trim_end().to_string();

    rsx! {
        div { class: "fixed inset-0 z-50 flex items-center justify-center",
            // Backdrop
            div {
                class: "fixed inset-0 bg-black/80",
                onclick: move |_| on_close.call(()),
            }
            // Panel
            div {
                class: "{panel_class}",
                onclick: move |evt: MouseEvent| evt.stop_propagation(),
                {children}
            }
        }
    }
}

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
    rsx! {
        Modal {
            open,
            on_close: move |_| on_cancel.call(()),
            bg: "glass-panel".to_string(),
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
