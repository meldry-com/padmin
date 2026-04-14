use dioxus::prelude::*;

use crate::api::palpo_admin;
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::card::*;
use crate::components::ui::dialog::ConfirmDialog;
use crate::components::ui::input::{Input, Label};
use crate::components::ui::loading::{LoadingSkeleton, Spinner};
use crate::components::ui::table::*;
use crate::components::ui::toast::{ToastVariant, show_toast};
use crate::types::ScheduledCommand;

// ── Scheduled Commands List ──

#[component]
pub fn ScheduledCommandsList(palpo_admin_url: String) -> Element {
    let url = palpo_admin_url.clone();
    let url_for_resource = url.clone();
    let mut data = use_resource(move || {
        let u = url_for_resource.clone();
        async move { palpo_admin::get_scheduled_commands(&u).await.ok() }
    });
    let mut show_create = use_signal(|| false);
    let mut delete_id = use_signal(|| Option::<String>::None);

    rsx! {
        Card {
            CardHeader { class: "flex flex-row items-center justify-between".to_string(),
                CardTitle { class: "text-base".to_string(), "Scheduled Commands" }
                Button {
                    size: ButtonSize::Sm,
                    onclick: move |_| show_create.set(true),
                    "Create"
                }
            }
            CardContent {
                match &*data.read() {
                    Some(Some(commands)) => {
                        if commands.is_empty() {
                            rsx! { p { class: "text-sm text-muted-foreground", "No scheduled commands." } }
                        } else {
                            rsx! {
                                Table {
                                    TableHeader {
                                        TableRow {
                                            TableHead { "Command" }
                                            TableHead { "Arguments" }
                                            TableHead { "Scheduled At" }
                                            TableHead { class: "text-right".to_string(), "Actions" }
                                        }
                                    }
                                    TableBody {
                                        for cmd in commands.iter() {
                                            {
                                                let name = cmd.command.clone();
                                                let args = cmd.args.as_ref().map(|a| a.to_string()).unwrap_or_else(|| "-".to_string());
                                                let scheduled = cmd.scheduled_at.clone().unwrap_or_else(|| "-".to_string());
                                                let cmd_id = cmd.id.clone();
                                                rsx! {
                                                    TableRow {
                                                        TableCell { class: "font-medium".to_string(), "{name}" }
                                                        TableCell { class: "text-muted-foreground text-xs font-mono".to_string(), "{args}" }
                                                        TableCell { "{scheduled}" }
                                                        TableCell { class: "text-right".to_string(),
                                                            Button {
                                                                variant: ButtonVariant::Ghost,
                                                                size: ButtonSize::Sm,
                                                                class: "text-destructive".to_string(),
                                                                onclick: move |_| delete_id.set(Some(cmd_id.clone())),
                                                                "Delete"
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
                    Some(None) => rsx! {
                        div { class: "flex items-center justify-between rounded-md bg-destructive/10 p-3",
                            p { class: "text-sm text-destructive", "Failed to load commands." }
                            button { class: "text-sm font-medium text-primary hover:underline", onclick: move |_| data.restart(), "Retry" }
                        }
                    },
                    None => rsx! { LoadingSkeleton {} },
                }
            }
        }

        if *show_create.read() {
            ScheduledCommandCreateDialog {
                palpo_admin_url: url.clone(),
                on_close: move |_| { show_create.set(false); data.restart(); },
            }
        }

        ConfirmDialog {
            open: delete_id.read().is_some(),
            title: "Delete Scheduled Command".to_string(),
            description: "Are you sure you want to delete this scheduled command?".to_string(),
            confirm_text: "Delete".to_string(),
            destructive: true,
            on_confirm: {
                let url = url.clone();
                move |_| {
                    let id = delete_id.read().clone();
                    let u = url.clone();
                    if let Some(id) = id {
                        spawn(async move {
                            match palpo_admin::delete_scheduled_command(&u, &id).await {
                                Ok(_) => { show_toast("Deleted", ToastVariant::Success); data.restart(); }
                                Err(e) => show_toast(&format!("Failed: {}", e.message), ToastVariant::Error),
                            }
                            delete_id.set(None);
                        });
                    }
                }
            },
            on_cancel: move |_| delete_id.set(None),
        }
    }
}

#[component]
fn ScheduledCommandCreateDialog(palpo_admin_url: String, on_close: EventHandler<()>) -> Element {
    let mut command = use_signal(|| String::new());
    let mut scheduled_at = use_signal(|| String::new());
    let mut args = use_signal(|| String::new());
    let mut loading = use_signal(|| false);

    let url = palpo_admin_url.clone();
    let handle_save = move |_: MouseEvent| {
        let u = url.clone();
        let cmd = command.read().clone();
        let at = scheduled_at.read().clone();
        let a = args.read().clone();

        if cmd.is_empty() || at.is_empty() {
            show_toast(
                "Command and schedule time are required",
                ToastVariant::Error,
            );
            return;
        }

        loading.set(true);
        spawn(async move {
            let sc = ScheduledCommand {
                command: cmd,
                scheduled_at: Some(at),
                args: if a.is_empty() {
                    None
                } else {
                    Some(serde_json::json!(a))
                },
                ..Default::default()
            };
            match palpo_admin::create_scheduled_command(&u, &sc).await {
                Ok(_) => {
                    show_toast("Command scheduled", ToastVariant::Success);
                    on_close.call(());
                }
                Err(e) => {
                    show_toast(&format!("Failed: {}", e.message), ToastVariant::Error);
                    loading.set(false);
                }
            }
        });
    };

    let is_loading = *loading.read();

    rsx! {
        div { class: "fixed inset-0 z-50 flex items-center justify-center",
            div { class: "fixed inset-0 bg-black/80", onclick: move |_| on_close.call(()) }
            div { class: "relative z-50 w-full max-w-lg rounded-lg border bg-background p-6 shadow-lg",
                h2 { class: "text-lg font-semibold mb-4", "Schedule Command" }
                div { class: "space-y-4",
                    div { class: "space-y-2",
                        Label { "Command" }
                        Input { placeholder: "Command name".to_string(), value: command.read().clone(), oninput: move |e: FormEvent| command.set(e.value()) }
                    }
                    div { class: "space-y-2",
                        Label { "Arguments (optional)" }
                        Input { placeholder: "Arguments".to_string(), value: args.read().clone(), oninput: move |e: FormEvent| args.set(e.value()) }
                    }
                    div { class: "space-y-2",
                        Label { "Scheduled At (ISO 8601)" }
                        Input { placeholder: "2025-01-15T10:00:00Z".to_string(), value: scheduled_at.read().clone(), oninput: move |e: FormEvent| scheduled_at.set(e.value()) }
                    }
                }
                div { class: "responsive-action-row mt-4",
                    Button { variant: ButtonVariant::Outline, onclick: move |_| on_close.call(()), "Cancel" }
                    Button { disabled: is_loading, onclick: handle_save, if is_loading { Spinner { class: "mr-2".to_string() } } "Schedule" }
                }
            }
        }
    }
}

// ── Recurring Commands List ──

#[component]
pub fn RecurringCommandsList(palpo_admin_url: String) -> Element {
    let url = palpo_admin_url.clone();
    let url_for_resource = url.clone();
    let mut data = use_resource(move || {
        let u = url_for_resource.clone();
        async move { palpo_admin::get_recurring_commands(&u).await.ok() }
    });
    let mut show_create = use_signal(|| false);
    let mut delete_id = use_signal(|| Option::<String>::None);

    rsx! {
        Card {
            CardHeader { class: "flex flex-row items-center justify-between".to_string(),
                CardTitle { class: "text-base".to_string(), "Recurring Commands" }
                Button {
                    size: ButtonSize::Sm,
                    onclick: move |_| show_create.set(true),
                    "Create"
                }
            }
            CardContent {
                match &*data.read() {
                    Some(Some(commands)) => {
                        if commands.is_empty() {
                            rsx! { p { class: "text-sm text-muted-foreground", "No recurring commands." } }
                        } else {
                            rsx! {
                                Table {
                                    TableHeader {
                                        TableRow {
                                            TableHead { "Command" }
                                            TableHead { "Arguments" }
                                            TableHead { "Time (UTC)" }
                                            TableHead { class: "text-right".to_string(), "Actions" }
                                        }
                                    }
                                    TableBody {
                                        for cmd in commands.iter() {
                                            {
                                                let name = cmd.command.clone();
                                                let args = cmd.args.as_ref().map(|a| a.to_string()).unwrap_or_else(|| "-".to_string());
                                                let time = cmd.time.clone().unwrap_or_else(|| "-".to_string());
                                                let cmd_id = cmd.id.clone();
                                                rsx! {
                                                    TableRow {
                                                        TableCell { class: "font-medium".to_string(), "{name}" }
                                                        TableCell { class: "text-muted-foreground text-xs font-mono".to_string(), "{args}" }
                                                        TableCell { "{time}" }
                                                        TableCell { class: "text-right".to_string(),
                                                            Button {
                                                                variant: ButtonVariant::Ghost,
                                                                size: ButtonSize::Sm,
                                                                class: "text-destructive".to_string(),
                                                                onclick: move |_| delete_id.set(Some(cmd_id.clone())),
                                                                "Delete"
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
                    Some(None) => rsx! {
                        div { class: "flex items-center justify-between rounded-md bg-destructive/10 p-3",
                            p { class: "text-sm text-destructive", "Failed to load commands." }
                            button { class: "text-sm font-medium text-primary hover:underline", onclick: move |_| data.restart(), "Retry" }
                        }
                    },
                    None => rsx! { LoadingSkeleton {} },
                }
            }
        }

        if *show_create.read() {
            RecurringCommandCreateDialog {
                palpo_admin_url: url.clone(),
                on_close: move |_| { show_create.set(false); data.restart(); },
            }
        }

        ConfirmDialog {
            open: delete_id.read().is_some(),
            title: "Delete Recurring Command".to_string(),
            description: "Are you sure you want to delete this recurring command?".to_string(),
            confirm_text: "Delete".to_string(),
            destructive: true,
            on_confirm: {
                let url = url.clone();
                move |_| {
                    let id = delete_id.read().clone();
                    let u = url.clone();
                    if let Some(id) = id {
                        spawn(async move {
                            match palpo_admin::delete_recurring_command(&u, &id).await {
                                Ok(_) => { show_toast("Deleted", ToastVariant::Success); data.restart(); }
                                Err(e) => show_toast(&format!("Failed: {}", e.message), ToastVariant::Error),
                            }
                            delete_id.set(None);
                        });
                    }
                }
            },
            on_cancel: move |_| delete_id.set(None),
        }
    }
}

#[component]
fn RecurringCommandCreateDialog(palpo_admin_url: String, on_close: EventHandler<()>) -> Element {
    let mut command = use_signal(|| String::new());
    let mut time = use_signal(|| String::new());
    let mut args = use_signal(|| String::new());
    let mut loading = use_signal(|| false);

    let url = palpo_admin_url.clone();
    let handle_save = move |_: MouseEvent| {
        let u = url.clone();
        let cmd = command.read().clone();
        let t = time.read().clone();
        let a = args.read().clone();

        if cmd.is_empty() || t.is_empty() {
            show_toast("Command and time are required", ToastVariant::Error);
            return;
        }

        loading.set(true);
        spawn(async move {
            let rc = crate::types::RecurringCommand {
                command: cmd,
                time: Some(t),
                args: if a.is_empty() {
                    None
                } else {
                    Some(serde_json::json!(a))
                },
                ..Default::default()
            };
            match palpo_admin::create_recurring_command(&u, &rc).await {
                Ok(_) => {
                    show_toast("Recurring command created", ToastVariant::Success);
                    on_close.call(());
                }
                Err(e) => {
                    show_toast(&format!("Failed: {}", e.message), ToastVariant::Error);
                    loading.set(false);
                }
            }
        });
    };

    let is_loading = *loading.read();

    rsx! {
        div { class: "fixed inset-0 z-50 flex items-center justify-center",
            div { class: "fixed inset-0 bg-black/80", onclick: move |_| on_close.call(()) }
            div { class: "relative z-50 w-full max-w-lg rounded-lg border bg-background p-6 shadow-lg",
                h2 { class: "text-lg font-semibold mb-4", "Create Recurring Command" }
                div { class: "space-y-4",
                    div { class: "space-y-2",
                        Label { "Command" }
                        Input { placeholder: "Command name".to_string(), value: command.read().clone(), oninput: move |e: FormEvent| command.set(e.value()) }
                    }
                    div { class: "space-y-2",
                        Label { "Arguments (optional)" }
                        Input { placeholder: "Arguments".to_string(), value: args.read().clone(), oninput: move |e: FormEvent| args.set(e.value()) }
                    }
                    div { class: "space-y-2",
                        Label { "Time (UTC, e.g. 03:00)" }
                        Input { placeholder: "HH:MM".to_string(), value: time.read().clone(), oninput: move |e: FormEvent| time.set(e.value()) }
                    }
                }
                div { class: "responsive-action-row mt-4",
                    Button { variant: ButtonVariant::Outline, onclick: move |_| on_close.call(()), "Cancel" }
                    Button { disabled: is_loading, onclick: handle_save, if is_loading { Spinner { class: "mr-2".to_string() } } "Create" }
                }
            }
        }
    }
}
