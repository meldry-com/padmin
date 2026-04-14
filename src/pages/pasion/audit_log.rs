use dioxus::prelude::*;

use crate::api::pasion;
use crate::components::ui::badge::{Badge, BadgeVariant};
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::error_banner::ErrorBanner;
use crate::components::ui::input::Input;
use crate::components::ui::loading::{PageSkeleton, Spinner};
use crate::components::ui::page_header::PageHeader;
use crate::components::ui::table::*;
use crate::components::ui::toast::{ToastVariant, show_toast};
use crate::types::PasionAuditEntry;
use crate::utils::i18n::t;

const PAGE_LIMIT: u64 = 25;

#[component]
pub fn AuditLogPage() -> Element {
    let mut entries = use_signal(|| Vec::<PasionAuditEntry>::new());
    let mut cursor = use_signal(|| Option::<String>::None);
    let mut has_next = use_signal(|| false);
    let mut loading = use_signal(|| false);
    let mut load_error = use_signal(|| Option::<String>::None);
    let mut expanded_id = use_signal(|| Option::<String>::None);
    let mut op_filter = use_signal(|| String::new());

    // Initial load
    use_effect(move || {
        loading.set(true);
        spawn(async move {
            match pasion::pasion_get_audit_feed(None, PAGE_LIMIT).await {
                Ok(resp) => {
                    entries.set(resp.data);
                    has_next.set(resp.meta.has_next);
                    cursor.set(resp.meta.end_cursor);
                    load_error.set(None);
                }
                Err(e) => load_error.set(Some(e.message)),
            }
            loading.set(false);
        });
    });

    let handle_load_more = move |_: MouseEvent| {
        let cur = cursor.read().clone();
        loading.set(true);
        spawn(async move {
            match pasion::pasion_get_audit_feed(cur.as_deref(), PAGE_LIMIT).await {
                Ok(resp) => {
                    let mut all = entries.read().clone();
                    all.extend(resp.data);
                    entries.set(all);
                    has_next.set(resp.meta.has_next);
                    cursor.set(resp.meta.end_cursor);
                    load_error.set(None);
                }
                Err(e) => {
                    show_toast(
                        &format!("{} {}", t("pasion.audit_log.load_more_failed"), e.message),
                        ToastVariant::Error,
                    );
                }
            }
            loading.set(false);
        });
    };

    let is_loading = *loading.read();
    let filter_val = op_filter.read().clone();
    let all_entries = entries.read().clone();
    let filtered: Vec<_> = all_entries
        .iter()
        .filter(|e| filter_val.is_empty() || e.operation.contains(&filter_val))
        .collect::<Vec<_>>()
        .into_iter()
        .cloned()
        .collect();

    rsx! {
        div { class: "space-y-6",
            PageHeader {
                title: t("pasion.audit_log.title"),
                description: t("pasion.audit_log.description"),
            }

            // Filters
            div { class: "flex items-center gap-4 max-w-sm",
                Input {
                    placeholder: t("pasion.audit_log.filter_placeholder"),
                    value: op_filter.read().clone(),
                    oninput: move |evt: FormEvent| op_filter.set(evt.value()),
                }
            }

            if let Some(err) = load_error.read().clone() {
                ErrorBanner { message: err }
            } else if is_loading && filtered.is_empty() {
                PageSkeleton {}
            } else {
                div { class: "rounded-md border",
                    Table {
                        TableHeader {
                            TableRow {
                                TableHead { {t("pasion.audit_log.col_timestamp")} }
                                TableHead { {t("pasion.audit_log.col_operation")} }
                                TableHead { {t("pasion.audit_log.col_admin")} }
                                TableHead { {t("pasion.audit_log.col_resource")} }
                                TableHead { {t("pasion.audit_log.col_ip")} }
                                TableHead { {t("pasion.audit_log.col_detail")} }
                            }
                        }
                        TableBody {
                            if filtered.is_empty() {
                                EmptyRow { colspan: 6, message: t("pasion.audit_log.empty") }
                            } else {
                                for entry in filtered.iter() {
                                    {
                                        let entry_id = entry.id.clone();
                                        let entry_id2 = entry_id.clone();
                                        let op = entry.operation.clone();
                                        let created = entry.created_at.clone();
                                        let admin = entry.admin_id.clone().unwrap_or_else(|| "-".to_string());
                                        let resource = match (&entry.resource_type, &entry.resource_id) {
                                            (Some(rt), Some(rid)) => format!("{rt}: {rid}"),
                                            (Some(rt), None) => rt.clone(),
                                            _ => "-".to_string(),
                                        };
                                        let ip = entry.ip_address.clone().unwrap_or_else(|| "-".to_string());
                                        let has_detail = entry.detail.is_some();
                                        let detail_json = entry.detail.as_ref()
                                            .map(|d| serde_json::to_string_pretty(d).unwrap_or_default())
                                            .unwrap_or_default();
                                        let is_expanded = expanded_id.read().as_deref() == Some(&entry_id);

                                        rsx! {
                                            TableRow {
                                                key: "{entry_id}",
                                                TableCell {
                                                    span { class: "text-xs text-muted-foreground font-mono", "{created}" }
                                                }
                                                TableCell {
                                                    Badge { variant: BadgeVariant::Secondary, "{op}" }
                                                }
                                                TableCell {
                                                    span { class: "text-xs font-mono", "{admin}" }
                                                }
                                                TableCell {
                                                    span { class: "text-xs text-muted-foreground", "{resource}" }
                                                }
                                                TableCell {
                                                    span { class: "text-xs font-mono text-muted-foreground", "{ip}" }
                                                }
                                                TableCell {
                                                    if has_detail {
                                                        Button {
                                                            variant: ButtonVariant::Ghost,
                                                            size: ButtonSize::Sm,
                                                            onclick: move |_| {
                                                                if is_expanded {
                                                                    expanded_id.set(None);
                                                                } else {
                                                                    expanded_id.set(Some(entry_id2.clone()));
                                                                }
                                                            },
                                                            if is_expanded { {t("common.hide")} } else { {t("common.show")} }
                                                        }
                                                    }
                                                }
                                            }
                                            if is_expanded {
                                                TableRow {
                                                    key: "{entry_id}-detail",
                                                    td { colspan: 6,
                                                        pre { class: "text-xs bg-muted p-3 rounded overflow-auto max-h-48 font-mono",
                                                            "{detail_json}"
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
                }

                if *has_next.read() {
                    div { class: "flex justify-center mt-4",
                        Button {
                            variant: ButtonVariant::Outline,
                            disabled: is_loading,
                            onclick: handle_load_more,
                            if is_loading {
                                Spinner { class: "mr-2".to_string() }
                            }
                            {t("common.load_more")}
                        }
                    }
                }
            }
        }
    }
}
