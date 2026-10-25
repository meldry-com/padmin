use std::cell::RefCell;
use wasm_bindgen::prelude::*;
struct Mounted {
    _app: glory::web::WebApp,
    pop: Closure<dyn FnMut(web_sys::Event)>,
    key: Closure<dyn FnMut(web_sys::KeyboardEvent)>,
    media: web_sys::MediaQueryList,
    changed: Closure<dyn FnMut(web_sys::Event)>,
    context: std::rc::Rc<crate::ui::context::Context>,
    root: web_sys::Element,
    click: Closure<dyn FnMut(web_sys::MouseEvent)>,
    storage: Closure<dyn FnMut(web_sys::Event)>,
}
thread_local! { static APP: RefCell<Option<Mounted>> = const {RefCell::new(None)}; }
impl Drop for Mounted {
    fn drop(&mut self) {
        if let Some(w) = web_sys::window() {
            let _ = w
                .remove_event_listener_with_callback("popstate", self.pop.as_ref().unchecked_ref());
            let _ =
                w.remove_event_listener_with_callback("keydown", self.key.as_ref().unchecked_ref());
            let _ = self.media.remove_event_listener_with_callback(
                "change",
                self.changed.as_ref().unchecked_ref(),
            );
            let _ = self
                .root
                .remove_event_listener_with_callback("click", self.click.as_ref().unchecked_ref());
            let _ = w.remove_event_listener_with_callback(
                "storage",
                self.storage.as_ref().unchecked_ref(),
            );
        }
    }
}
#[wasm_bindgen]
pub fn unmount() {
    let old = APP.with(|app| app.borrow_mut().take());
    drop(old);
}
#[wasm_bindgen(start)]
pub fn mount() -> Result<(), JsValue> {
    use wasm_bindgen::JsCast;
    unmount();
    let w = web_sys::window().ok_or("missing window")?;
    let root = w
        .document()
        .and_then(|d| d.get_element_by_id("app"))
        .ok_or("missing app root")?;
    let route = crate::router::parse(
        &w.location().pathname()?,
        &crate::utils::config::base_path(),
    );
    // Preserve encoded URL parameters for the pure parser.
    let initial = w
        .location()
        .pathname()?
        .strip_prefix(&crate::utils::config::base_path())
        .unwrap_or("/")
        .to_owned();
    let initial = if route.path == "/*" {
        "/*".into()
    } else {
        initial
    };
    let app = glory::web::launch_with(root.clone(), |scope| crate::ui::app(scope, initial))
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let context = crate::ui::context::get(app.app().scope()).ok_or("missing app context")?;
    let weak = std::rc::Rc::downgrade(&context);
    let pop = Closure::wrap(Box::new(move |_: web_sys::Event| {
        if let Some(w) = web_sys::window() {
            let path = w.location().pathname().unwrap_or_default();
            let prefix = crate::utils::config::base_path();
            if let Some(ctx) = weak.upgrade() {
                ctx.queue(crate::ui::context::Command::Pop(
                    path.strip_prefix(&prefix)
                        .filter(|p| prefix.is_empty() || p.is_empty() || p.starts_with('/'))
                        .unwrap_or("/*")
                        .to_owned(),
                ));
            }
        }
    }) as Box<dyn FnMut(_)>);
    w.add_event_listener_with_callback("popstate", pop.as_ref().unchecked_ref())?;
    let weak = std::rc::Rc::downgrade(&context);
    let key = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
        if event.ctrl_key() || event.alt_key() || event.meta_key() || event.is_composing() {
            return;
        }
        if event.key() == "Tab"
            && web_sys::window()
                .and_then(|w| w.document())
                .and_then(|doc| doc.query_selector(".shortcut-help").ok().flatten())
                .is_some()
        {
            event.prevent_default();
        }
        if let Some(active) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.active_element())
        {
            if matches!(active.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
                || active.get_attribute("contenteditable").as_deref() == Some("true")
            {
                return;
            }
        }
        if let Some(ctx) = weak.upgrade() {
            ctx.queue(crate::ui::context::Command::Key(
                if event.key() == "Tab" && event.shift_key() {
                    "Shift+Tab".into()
                } else {
                    event.key()
                },
                js_sys::Date::now(),
            ));
        }
    }) as Box<dyn FnMut(_)>);
    w.add_event_listener_with_callback("keydown", key.as_ref().unchecked_ref())?;
    let media = w
        .match_media("(prefers-color-scheme: dark)")?
        .ok_or("matchMedia unavailable")?;
    context.queue(crate::ui::context::Command::SystemTheme(media.matches()));
    let weak = std::rc::Rc::downgrade(&context);
    let query = media.clone();
    let changed = Closure::wrap(Box::new(move |_: web_sys::Event| {
        if let Some(ctx) = weak.upgrade() {
            ctx.queue(crate::ui::context::Command::SystemTheme(query.matches()));
        }
    }) as Box<dyn FnMut(_)>);
    media.add_event_listener_with_callback("change", changed.as_ref().unchecked_ref())?;
    let weak = std::rc::Rc::downgrade(&context);
    let link_root = root.clone();
    let click = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
        if event.button() != 0
            || event.ctrl_key()
            || event.meta_key()
            || event.shift_key()
            || event.alt_key()
        {
            return;
        }
        let Some(anchor) = event
            .target()
            .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
            .and_then(|target| target.closest("a[href]").ok().flatten())
        else {
            return;
        };
        if !link_root.contains(Some(&anchor)) {
            return;
        }
        let Some(href) = anchor
            .get_attribute("href")
            .filter(|href| href.starts_with('/') && !href.starts_with("//"))
        else {
            return;
        };
        let prefix = crate::utils::config::base_path();
        let Some(path) = href
            .strip_prefix(&prefix)
            .filter(|p| prefix.is_empty() || p.starts_with('/'))
        else {
            return;
        };
        event.prevent_default();
        if let Some(ctx) = weak.upgrade() {
            ctx.queue(crate::ui::context::Command::Navigate(path.into()));
        }
    }) as Box<dyn FnMut(_)>);
    root.add_event_listener_with_callback("click", click.as_ref().unchecked_ref())?;
    let weak = std::rc::Rc::downgrade(&context);
    let storage = Closure::wrap(Box::new(move |event: web_sys::Event| {
        let key = js_sys::Reflect::get(event.as_ref(), &"key".into())
            .ok()
            .and_then(|v| v.as_string());
        if key.as_deref() == Some("padmin.logout") {
            if let Some(ctx) = weak.upgrade() {
                ctx.queue(crate::ui::context::Command::Retire);
            }
        }
    }) as Box<dyn FnMut(_)>);
    w.add_event_listener_with_callback("storage", storage.as_ref().unchecked_ref())?;
    APP.with(|slot| {
        *slot.borrow_mut() = Some(Mounted {
            _app: app,
            pop,
            key,
            media,
            changed,
            context,
            root,
            click,
            storage,
        })
    });
    Ok(())
}
use wasm_bindgen::JsCast;

pub(crate) fn queue(command: crate::ui::context::Command) {
    APP.with(|slot| {
        if let Some(app) = slot.borrow().as_ref() {
            app.context.queue(command);
        }
    });
}
/// Read-only acceptance diagnostics: no auth bypass or mutation backdoor.
#[wasm_bindgen]
pub fn stats() -> String {
    APP.with(|slot|{let app=slot.borrow();let Some(app)=app.as_ref()else{return "null".into();};let ui=app._app.app().stats();let rt=app._app.app().runtime_stats();serde_json::json!({"nodes":ui.nodes,"owners":rt.owners,"tasks":rt.tasks,"edges":rt.edges,"handlers":ui.handlers,"queries":ui.queries,"queued":rt.queued,"notifications":app.context.notifications.with(|v|v.len())}).to_string()})
}
