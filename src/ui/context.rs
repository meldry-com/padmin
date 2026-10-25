//! Application-owned state. The registry contains weak references only.
use crate::utils::{i18n::Language, storage};
use glory::{
    core::{Cage, RuntimeId},
    ui::Scope,
};
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    rc::{Rc, Weak},
};

#[derive(Clone, Debug)]
pub struct Notification {
    pub id: u64,
    pub kind: String,
    pub message: String,
    pub failed: bool,
    pub read: bool,
    pub timestamp: String,
    pub severity: String,
}
#[derive(Clone, Debug)]
pub struct Toast {
    pub id: u64,
    pub message: String,
    pub failed: bool,
    pub expires: i64,
}
pub enum Command {
    Navigate(String),
    Pop(String),
    Reverify,
    Retire,
    Key(String, f64),
    SystemTheme(bool),
}
pub struct Context {
    pub language: Cage<Language>,
    pub theme: Cage<String>,
    pub contrast: Cage<bool>,
    pub dark: Cage<bool>,
    pub help: Cage<bool>,
    pub drawer: Cage<bool>,
    pub collapsed: Cage<bool>,
    pub identity: Cage<(String, String)>,
    pub help_origin: RefCell<Option<glory::render::NodeId>>,
    pub dialog_focus: RefCell<Option<glory::render::NodeId>>,
    pub focus_request: Cage<Option<glory::render::NodeId>>,
    pub notifications: Cage<Vec<Notification>>,
    pub toasts: Cage<Vec<Toast>>,
    pub preferences: Cage<HashMap<String, bool>>,
    pub commands: RefCell<VecDeque<Command>>,
    pub pending_key: RefCell<Option<f64>>,
    counter: std::cell::Cell<u64>,
    pub secrets: RefCell<HashMap<String, String>>,
    pub settings: RefCell<HashMap<String, String>>,
    pub cache: RefCell<HashMap<String, (f64, String)>>,
    pub metrics: RefCell<Vec<crate::utils::perf::ApiMetric>>,
    pub session: std::cell::Cell<u64>,
    pub refresh: RefCell<
        Option<(
            u64,
            futures::future::Shared<futures::future::LocalBoxFuture<'static, bool>>,
        )>,
    >,
}
thread_local! {
    static ROOTS:RefCell<HashMap<RuntimeId,Weak<Context>>>=RefCell::new(HashMap::new());
    static CURRENT:RefCell<Option<Weak<Context>>>=const {RefCell::new(None)};
}
pub fn install(scope: &Scope) -> Rc<Context> {
    let context = Rc::new(Context {
        language: scope.cage(crate::utils::i18n::current_language()),
        theme: scope.cage(storage::get_item("theme").unwrap_or_else(|| "system".into())),
        contrast: scope.cage(storage::get_item("high_contrast").as_deref() == Some("true")),
        dark: scope.cage(false),
        help: scope.cage(false),
        drawer: scope.cage(false),
        collapsed: scope.cage(false),
        identity: scope.cage((String::new(), String::new())),
        help_origin: RefCell::new(None),
        dialog_focus: RefCell::new(None),
        focus_request: scope.cage(None),
        notifications: scope.cage(Vec::new()),
        toasts: scope.cage(Vec::new()),
        preferences: scope.cage(
            storage::get_item("notification_preferences")
                .and_then(|v| serde_json::from_str(&v).ok())
                .unwrap_or_default(),
        ),
        commands: RefCell::new(VecDeque::new()),
        pending_key: RefCell::new(None),
        counter: std::cell::Cell::new(0),
        secrets: RefCell::new(HashMap::new()),
        settings: RefCell::new(HashMap::new()),
        cache: RefCell::new(HashMap::new()),
        metrics: RefCell::new(Vec::new()),
        session: std::cell::Cell::new(0),
        refresh: RefCell::new(None),
    });
    #[cfg(target_arch = "wasm32")]
    if let Some(origin) = web_sys::window().and_then(|w| w.location().origin().ok()) {
        context.settings.borrow_mut().insert(
            "pasion_url".into(),
            crate::utils::config::pasion_proxy_base(&origin),
        );
        context.settings.borrow_mut().insert(
            "palpo_admin_url".into(),
            storage::get_item("palpo_admin_url")
                .unwrap_or_else(|| format!("{origin}/_palpo/admin/v1")),
        );
    }
    let runtime = scope.id().runtime();
    ROOTS.with(|roots| roots.borrow_mut().insert(runtime, Rc::downgrade(&context)));
    scope
        .on_cleanup(move || {
            ROOTS.with(|roots| roots.borrow_mut().remove(&runtime));
        })
        .expect("live root");
    scope.retain(context.clone()).expect("retain app context");
    context
}
pub fn get(scope: &Scope) -> Option<Rc<Context>> {
    for_runtime(scope.id().runtime())
}
pub fn for_runtime(id: RuntimeId) -> Option<Rc<Context>> {
    ROOTS.with(|roots| roots.borrow().get(&id).and_then(Weak::upgrade))
}
pub fn current() -> Option<Rc<Context>> {
    CURRENT.with(|value| value.borrow().as_ref().and_then(Weak::upgrade))
}
pub fn enter<R>(context: &Rc<Context>, f: impl FnOnce() -> R) -> R {
    struct Restore(Option<Weak<Context>>);
    impl Drop for Restore {
        fn drop(&mut self) {
            CURRENT.with(|value| *value.borrow_mut() = self.0.take());
        }
    }
    let old = CURRENT.with(|value| value.replace(Some(Rc::downgrade(context))));
    let _restore = Restore(old);
    f()
}
impl Context {
    pub fn queue(&self, command: Command) {
        let mut queue = self.commands.borrow_mut();
        if matches!(command, Command::Reverify)
            && queue.iter().any(|v| matches!(v, Command::Reverify))
        {
            return;
        }
        if queue.len() < 128 {
            queue.push_back(command);
        }
    }
    pub fn notify(&self, kind: &str, message: String, failed: bool) {
        if !self
            .preferences
            .with(|v| v.get(kind).copied().unwrap_or(true))
        {
            return;
        }
        let Some(id) = self.counter.get().checked_add(1) else {
            return;
        };
        self.counter.set(id);
        self.notifications.revise(|items| {
            items.insert(
                0,
                Notification {
                    id,
                    kind: kind.into(),
                    message,
                    failed,
                    read: false,
                    timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                    severity: if failed {
                        "error"
                    } else if matches!(kind, "report_filed" | "federation_alert") {
                        "warning"
                    } else {
                        "info"
                    }
                    .into(),
                },
            );
            items.truncate(50);
        });
    }
    pub fn toast(&self, message: String, failed: bool) {
        let Some(id) = self.counter.get().checked_add(1) else {
            return;
        };
        self.counter.set(id);
        let expires = chrono::Utc::now().timestamp_millis() + if failed { 5000 } else { 3000 };
        self.toasts.revise(|items| {
            items.push(Toast {
                id,
                message,
                failed,
                expires,
            });
            if items.len() > 5 {
                items.drain(..items.len() - 5);
            }
        });
    }
    pub fn preference(&self, key: &str, value: bool) {
        self.preferences.revise(|prefs| {
            prefs.insert(key.to_owned(), value);
        });
        storage::set_item(
            "notification_preferences",
            &serde_json::to_string(&self.preferences.get()).expect("preferences serialize"),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glory::{
        render::{Props, memory::Memory},
        ui::App,
    };
    fn content(app: &App) -> String {
        let tree = app.tree();
        let Props::Text(text) = tree.node(tree.roots()[0]).unwrap().descriptor.view().props else {
            panic!("text");
        };
        text.content.clone()
    }
    #[test]
    fn independent_roots_language_and_secrets_do_not_leak() {
        let left = App::new(Memory::new);
        let right = App::new(Memory::new);
        let a = install(left.scope());
        let b = install(right.scope());
        a.language.revise(|v| *v = Language::En);
        b.language.revise(|v| *v = Language::En);
        left.mount_with(|| crate::ui::widgets::label("Sign in"))
            .unwrap();
        right
            .mount_with(|| crate::ui::widgets::label("Sign in"))
            .unwrap();
        a.language.revise(|v| *v = Language::ZhCn);
        left.flush().unwrap();
        right.flush().unwrap();
        assert_eq!(content(&left), "登录");
        assert_eq!(content(&right), "Sign in");
        enter(&a, || storage::set_item("access_token", "only-left"));
        assert_eq!(
            enter(&a, || storage::get_item("access_token")).as_deref(),
            Some("only-left")
        );
        assert_eq!(enter(&b, || storage::get_item("access_token")), None);
        let runtime = left.scope().id().runtime();
        drop(left);
        assert!(for_runtime(runtime).is_none());
    }
    #[test]
    fn notification_capacity_preferences_and_queue_are_bounded() {
        let app = App::new(Memory::new);
        let context = install(app.scope());
        context.preferences.revise(HashMap::clear);
        for index in 0..100 {
            context.notify("system_info", index.to_string(), false);
        }
        assert_eq!(context.notifications.get().len(), 50);
        assert_eq!(context.notifications.get()[0].message, "99");
        context.preference("system_info", false);
        context.notify("system_info", "ignored".into(), false);
        assert_eq!(context.notifications.get()[0].message, "99");
        for _ in 0..1000 {
            context.queue(Command::Reverify);
        }
        assert_eq!(context.commands.borrow().len(), 1);
    }
}
