use dioxus::prelude::*;

#[component]
pub fn Footer() -> Element {
    rsx! {
        footer { class: "border-t py-4 px-6 text-center text-xs text-muted-foreground",
            "Palpo Admin"
            " | "
            a {
                href: "https://palpo.im",
                target: "_blank",
                class: "text-primary hover:underline",
                "palpo.im"
            }
        }
    }
}
