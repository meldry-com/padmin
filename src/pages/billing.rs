use dioxus::prelude::*;

use crate::api::palpo_admin;
use crate::components::ui::card::*;
use crate::components::ui::loading::PageSkeleton;
use crate::components::ui::page_header::PageHeader;
use crate::components::ui::table::*;
use crate::utils::instance_config::get_palpo_admin_url;

#[component]
pub fn Billing() -> Element {
    let palpo_url = get_palpo_admin_url();

    let palpo_url_for_resource = palpo_url.clone();
    let payments_data = use_resource(move || {
        let url = palpo_url_for_resource.clone();
        async move {
            if let Some(url) = url {
                palpo_admin::get_payments(&url).await.ok()
            } else {
                None
            }
        }
    });

    if palpo_url.is_none() {
        return rsx! {
            div { class: "text-center p-8",
                p { class: "text-muted-foreground", "Palpo admin is not configured." }
            }
        };
    }

    rsx! {
        div { class: "space-y-6",
            PageHeader {
                title: "Billing".to_string(),
                description: "Subscription and payment information".to_string(),
            }

            match &*payments_data.read() {
                Some(Some(data)) => rsx! {
                    // Subscription info
                    if let Some(ref subscription) = data.subscription {
                        Card {
                            CardHeader {
                                CardTitle { "Current Subscription" }
                            }
                            CardContent {
                                div { class: "space-y-2",
                                    if let Some(plan) = subscription.get("plan").and_then(|p| p.as_str()) {
                                        div { class: "flex justify-between",
                                            span { class: "text-sm text-muted-foreground", "Plan" }
                                            span { class: "text-sm font-medium", "{plan}" }
                                        }
                                    }
                                    if let Some(method) = &data.payment_method {
                                        div { class: "flex justify-between",
                                            span { class: "text-sm text-muted-foreground", "Payment Method" }
                                            span { class: "text-sm font-medium", "{method}" }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Invoices
                    Card {
                        CardHeader {
                            CardTitle { "Invoices" }
                        }
                        CardContent {
                            if data.invoices.is_empty() {
                                p { class: "text-sm text-muted-foreground", "No invoices yet." }
                            } else {
                                Table {
                                    TableHeader {
                                        TableRow {
                                            TableHead { "ID" }
                                            TableHead { "Amount" }
                                            TableHead { "Status" }
                                            TableHead { "Date" }
                                        }
                                    }
                                    TableBody {
                                        for invoice in data.invoices.iter() {
                                            {
                                                let id = invoice.get("id").and_then(|v| v.as_str()).unwrap_or("-");
                                                let amount = invoice.get("amount").and_then(|v| v.as_f64()).map(|a| format!("${:.2}", a)).unwrap_or_else(|| "-".to_string());
                                                let status = invoice.get("status").and_then(|v| v.as_str()).unwrap_or("-");
                                                let date = invoice.get("date").and_then(|v| v.as_str()).unwrap_or("-");

                                                rsx! {
                                                    TableRow {
                                                        TableCell { class: "font-mono text-xs".to_string(), "{id}" }
                                                        TableCell { "{amount}" }
                                                        TableCell { "{status}" }
                                                        TableCell { class: "text-muted-foreground".to_string(), "{date}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Payment history
                    if !data.payments.is_empty() {
                        Card {
                            CardHeader {
                                CardTitle { "Payment History" }
                            }
                            CardContent {
                                Table {
                                    TableHeader {
                                        TableRow {
                                            TableHead { "Transaction" }
                                            TableHead { "Amount" }
                                            TableHead { "Type" }
                                            TableHead { "Date" }
                                        }
                                    }
                                    TableBody {
                                        for payment in data.payments.iter() {
                                            {
                                                let tx_id = payment.get("transaction_id").and_then(|v| v.as_str()).unwrap_or("-");
                                                let amount = payment.get("amount").and_then(|v| v.as_f64()).map(|a| format!("${:.2}", a)).unwrap_or_else(|| "-".to_string());
                                                let is_sub = payment.get("is_subscription").and_then(|v| v.as_bool()).unwrap_or(false);
                                                let paid_at = payment.get("paid_at").and_then(|v| v.as_str()).unwrap_or("-");

                                                rsx! {
                                                    TableRow {
                                                        TableCell { class: "font-mono text-xs max-w-[200px] truncate".to_string(), "{tx_id}" }
                                                        TableCell { "{amount}" }
                                                        TableCell {
                                                            if is_sub { "Subscription" } else { "One-time" }
                                                        }
                                                        TableCell { class: "text-muted-foreground".to_string(), "{paid_at}" }
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
                Some(None) => rsx! {
                    div { class: "text-center p-8",
                        p { class: "text-muted-foreground", "Unable to fetch billing information." }
                    }
                },
                None => rsx! { PageSkeleton {} },
            }
        }
    }
}
