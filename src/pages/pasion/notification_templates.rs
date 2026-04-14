use dioxus::prelude::*;

use crate::api::pasion;
use crate::components::ui::badge::{Badge, BadgeVariant};
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::error_banner::ErrorBanner;
use crate::components::ui::loading::{PageSkeleton, Spinner};
use crate::components::ui::page_header::PageHeader;
use crate::components::ui::table::*;
use crate::components::ui::toast::{ToastVariant, show_toast};
use crate::types::PasionNotificationTemplate;
use crate::utils::i18n::t;

#[component]
pub fn NotificationTemplatesPage() -> Element {
    let mut templates_data =
        use_resource(|| async { pasion::pasion_get_notification_templates().await });

    let mut preview_template = use_signal(|| Option::<PasionNotificationTemplate>::None);

    // Publish dialog state
    let mut show_publish = use_signal(|| false);
    let mut publish_name = use_signal(|| String::new());
    let mut publish_locale = use_signal(|| "en".to_string());
    let mut publish_channel = use_signal(|| "email".to_string());
    let mut publish_subject = use_signal(|| String::new());
    let mut publish_body = use_signal(|| String::new());
    let mut publishing = use_signal(|| false);

    let handle_publish = move |_: MouseEvent| {
        let name = publish_name.read().clone();
        let locale = publish_locale.read().clone();
        let channel = publish_channel.read().clone();
        let subject = publish_subject.read().clone();
        let body_text = publish_body.read().clone();

        let mut data = serde_json::json!({
            "name": name,
            "locale": locale,
            "channel": channel,
        });
        if !subject.is_empty() {
            data["subject"] = serde_json::Value::String(subject);
        }
        if !body_text.is_empty() {
            data["body"] = serde_json::Value::String(body_text);
        }

        publishing.set(true);
        spawn(async move {
            match pasion::pasion_publish_template(data).await {
                Ok(_) => {
                    show_toast("Template published", ToastVariant::Success);
                    show_publish.set(false);
                    publish_name.set(String::new());
                    publish_subject.set(String::new());
                    publish_body.set(String::new());
                    templates_data.restart();
                }
                Err(e) => show_toast(&format!("Failed: {}", e.message), ToastVariant::Error),
            }
            publishing.set(false);
        });
    };

    let is_publishing = *publishing.read();

    rsx! {
        div { class: "space-y-6",
            PageHeader {
                title: t("pasion.notification_templates.title"),
                description: t("pasion.notification_templates.description"),
                Button {
                    onclick: move |_| show_publish.set(true),
                    "Publish Template"
                }
            }

            match &*templates_data.read() {
                Some(Ok(templates)) => rsx! {
                    div { class: "rounded-md border",
                        Table {
                            TableHeader {
                                TableRow {
                                    TableHead { "Name" }
                                    TableHead { "Locale" }
                                    TableHead { "Channel" }
                                    TableHead { "Last Updated" }
                                    TableHead { class: "text-right".to_string(), "Actions" }
                                }
                            }
                            TableBody {
                                if templates.is_empty() {
                                    EmptyRow { colspan: 5, message: t("pasion.notification_templates.empty") }
                                } else {
                                    for template in templates.iter() {
                                        {
                                            let tid = template.id.clone();
                                            let name = template.name.clone();
                                            let locale = template.locale.clone();
                                            let channel = template.channel.clone();
                                            let updated = template.updated_at.clone();
                                            let tmpl_clone = template.clone();

                                            rsx! {
                                                TableRow { key: "{tid}",
                                                    TableCell {
                                                        span { class: "font-medium text-sm", "{name}" }
                                                    }
                                                    TableCell {
                                                        Badge { variant: BadgeVariant::Outline, "{locale}" }
                                                    }
                                                    TableCell {
                                                        Badge { variant: BadgeVariant::Secondary, "{channel}" }
                                                    }
                                                    TableCell {
                                                        span { class: "text-xs text-muted-foreground", "{updated}" }
                                                    }
                                                    TableCell { class: "text-right".to_string(),
                                                        Button {
                                                            variant: ButtonVariant::Ghost,
                                                            size: ButtonSize::Sm,
                                                            onclick: move |_| preview_template.set(Some(tmpl_clone.clone())),
                                                            "Preview"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                Some(Err(e)) => rsx! {
                    ErrorBanner {
                        message: e.message.clone(),
                        on_retry: move |_| templates_data.restart(),
                    }
                },
                None => rsx! { PageSkeleton {} },
            }
        }

        // Preview dialog
        if let Some(tmpl) = preview_template.read().clone() {
            div { class: "fixed inset-0 z-50 flex items-center justify-center",
                div {
                    class: "fixed inset-0 bg-black/80",
                    onclick: move |_| preview_template.set(None),
                }
                div { class: "relative z-50 w-full max-w-2xl rounded-lg border bg-background p-6 shadow-lg max-h-[80vh] overflow-y-auto",
                    div { class: "flex items-center justify-between mb-4",
                        h2 { class: "text-lg font-semibold", "{tmpl.name}" }
                        div { class: "flex items-center gap-2",
                            Badge { variant: BadgeVariant::Outline, "{tmpl.locale}" }
                            Badge { variant: BadgeVariant::Secondary, "{tmpl.channel}" }
                        }
                    }
                    if let Some(subject) = tmpl.subject {
                        div { class: "mb-4",
                            p { class: "text-xs font-medium text-muted-foreground mb-1", "Subject" }
                            p { class: "text-sm border rounded p-2 bg-muted/50", "{subject}" }
                        }
                    }
                    if let Some(body) = tmpl.body {
                        div {
                            p { class: "text-xs font-medium text-muted-foreground mb-1", "Body" }
                            pre { class: "text-xs bg-muted p-3 rounded overflow-auto whitespace-pre-wrap font-mono",
                                "{body}"
                            }
                        }
                    }
                    div { class: "mt-4 flex justify-end",
                        Button {
                            variant: ButtonVariant::Outline,
                            onclick: move |_| preview_template.set(None),
                            "Close"
                        }
                    }
                }
            }
        }

        // Publish dialog
        if *show_publish.read() {
            div { class: "fixed inset-0 z-50 flex items-center justify-center",
                div {
                    class: "fixed inset-0 bg-black/80",
                    onclick: move |_| {
                        if !is_publishing { show_publish.set(false); }
                    },
                }
                div { class: "relative z-50 w-full max-w-lg rounded-lg border bg-background p-6 shadow-lg",
                    h2 { class: "text-lg font-semibold mb-4", "Publish Notification Template" }
                    div { class: "space-y-4",
                        div { class: "space-y-2",
                            label { class: "text-sm font-medium leading-none", "Name" }
                            input {
                                class: "flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm",
                                placeholder: "Template name",
                                value: publish_name.read().clone(),
                                oninput: move |evt: FormEvent| publish_name.set(evt.value()),
                                disabled: is_publishing,
                            }
                        }
                        div { class: "grid grid-cols-2 gap-4",
                            div { class: "space-y-2",
                                label { class: "text-sm font-medium leading-none", "Locale" }
                                input {
                                    class: "flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm",
                                    placeholder: "en",
                                    value: publish_locale.read().clone(),
                                    oninput: move |evt: FormEvent| publish_locale.set(evt.value()),
                                    disabled: is_publishing,
                                }
                            }
                            div { class: "space-y-2",
                                label { class: "text-sm font-medium leading-none", "Channel" }
                                select {
                                    class: "flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm",
                                    disabled: is_publishing,
                                    onchange: move |evt: FormEvent| publish_channel.set(evt.value()),
                                    option { value: "email", "Email" }
                                    option { value: "sms", "SMS" }
                                }
                            }
                        }
                        div { class: "space-y-2",
                            label { class: "text-sm font-medium leading-none", "Subject (optional)" }
                            input {
                                class: "flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm",
                                placeholder: "Email subject",
                                value: publish_subject.read().clone(),
                                oninput: move |evt: FormEvent| publish_subject.set(evt.value()),
                                disabled: is_publishing,
                            }
                        }
                        div { class: "space-y-2",
                            label { class: "text-sm font-medium leading-none", "Body" }
                            textarea {
                                class: "flex min-h-[120px] w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring resize-y font-mono",
                                placeholder: "Template body content...",
                                disabled: is_publishing,
                                oninput: move |evt: FormEvent| publish_body.set(evt.value()),
                                "{publish_body.read().clone()}"
                            }
                        }
                    }
                    div { class: "flex flex-col-reverse sm:flex-row sm:justify-end sm:space-x-2 mt-6",
                        Button {
                            variant: ButtonVariant::Outline,
                            disabled: is_publishing,
                            onclick: move |_| show_publish.set(false),
                            "Cancel"
                        }
                        Button {
                            disabled: is_publishing,
                            onclick: handle_publish,
                            if is_publishing {
                                Spinner { class: "mr-2".to_string() }
                            }
                            "Publish"
                        }
                    }
                }
            }
        }
    }
}
