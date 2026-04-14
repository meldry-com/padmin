use dioxus::prelude::*;

use crate::api::pasion;
use crate::components::ui::badge::{Badge, BadgeVariant};
use crate::components::ui::button::{Button, ButtonVariant};
use crate::components::ui::error_banner::ErrorBanner;
use crate::components::ui::loading::PageSkeleton;
use crate::components::ui::page_header::PageHeader;
use crate::components::ui::table::*;
use crate::utils::i18n::t;

fn channel_status_badge(status: &str) -> Element {
    let (variant, label) = match status {
        "active" => (BadgeVariant::Success, "Active"),
        "degraded" => (BadgeVariant::Secondary, "Degraded"),
        "down" => (BadgeVariant::Destructive, "Down"),
        _ => (BadgeVariant::Outline, status),
    };
    rsx! {
        Badge { variant, "{label}" }
    }
}

fn channel_type_badge(channel_type: &str) -> Element {
    let variant = match channel_type {
        "email" => BadgeVariant::Default,
        "sms" => BadgeVariant::Secondary,
        _ => BadgeVariant::Outline,
    };
    rsx! {
        Badge { variant, "{channel_type}" }
    }
}

#[component]
pub fn NotificationChannelsPage() -> Element {
    let mut channels_data =
        use_resource(|| async { pasion::pasion_get_notification_channels().await });

    rsx! {
        div { class: "space-y-6",
            PageHeader {
                title: t("pasion.notification_channels.title"),
                description: t("pasion.notification_channels.description"),
                Button {
                    variant: ButtonVariant::Outline,
                    onclick: move |_| channels_data.restart(),
                    {t("common.refresh")}
                }
            }

            match &*channels_data.read() {
                Some(Ok(channels)) => rsx! {
                    div { class: "rounded-md border",
                        Table {
                            TableHeader {
                                TableRow {
                                    TableHead { "Name" }
                                    TableHead { "Type" }
                                    TableHead { "Provider" }
                                    TableHead { "Status" }
                                }
                            }
                            TableBody {
                                if channels.is_empty() {
                                    EmptyRow { colspan: 4, message: t("pasion.notification_channels.empty") }
                                } else {
                                    for channel in channels.iter() {
                                        {
                                            let cid = channel.id.clone();
                                            let name = channel.name.clone();
                                            let ctype = channel.channel_type.clone();
                                            let provider = channel.provider.clone();
                                            let status = channel.status.clone();

                                            rsx! {
                                                TableRow { key: "{cid}",
                                                    TableCell {
                                                        span { class: "font-medium", "{name}" }
                                                    }
                                                    TableCell {
                                                        {channel_type_badge(&ctype)}
                                                    }
                                                    TableCell {
                                                        span { class: "text-sm text-muted-foreground", "{provider}" }
                                                    }
                                                    TableCell {
                                                        {channel_status_badge(&status)}
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
                        on_retry: move |_| channels_data.restart(),
                    }
                },
                None => rsx! { PageSkeleton {} },
            }
        }
    }
}
