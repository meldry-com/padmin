use crate::utils::error::HttpError;
use glory::prelude::*;
use glory::ui::{Input, Scope, Text, View, events};
use sapid::Button;
use std::future::Future;
use std::result::Result;

pub struct LocalText {
    inner: Option<Text>,
    value: String,
    name: Option<String>,
}
impl LocalText {
    pub fn name(mut self, value: &str) -> Self {
        self.name = Some(value.into());
        self
    }
    pub fn class(mut self, value: impl Into<glory::Lotus<String>>) -> Self {
        self.inner = Some(self.inner.take().unwrap().class(value));
        self
    }
    pub fn kind(mut self, value: glory::render::TextKind) -> Self {
        self.inner = Some(self.inner.take().unwrap().kind(value));
        self
    }
}
impl glory::ui::Widget for LocalText {
    fn build(&mut self, scope: &Scope) -> glory::ui::Result {
        let value = self.value.clone();
        let content = localized(scope, value);
        let mut inner = self.inner.take().unwrap().content(content);
        if let Some(name) = self.name.take() {
            inner = inner.name(localized(scope, name));
        }
        inner.build(scope)
    }
}
pub struct LocalButton {
    inner: Option<Button>,
    caption: String,
    callback: Option<Box<dyn Fn()>>,
}
impl LocalButton {
    pub fn name(mut self, value: impl Into<glory::Lotus<String>>) -> Self {
        self.inner = Some(self.inner.take().unwrap().name(value));
        self
    }
    pub fn class(mut self, value: impl Into<glory::Lotus<String>>) -> Self {
        self.inner = Some(self.inner.take().unwrap().class(value));
        self
    }
    pub fn enabled(mut self, value: impl Into<glory::Lotus<bool>>) -> Self {
        self.inner = Some(self.inner.take().unwrap().enabled(value));
        self
    }
}
impl glory::ui::Widget for LocalButton {
    fn build(&mut self, scope: &Scope) -> glory::ui::Result {
        let caption = localized(scope, self.caption.clone());
        let callback = self.callback.take().unwrap();
        let context = super::context::get(scope);
        let mut inner = self.inner.take().unwrap().label(caption).on_press(move || {
            if let Some(ctx) = context.as_ref() {
                super::context::enter(ctx, || callback());
            } else {
                callback();
            }
        });
        inner.build(scope)
    }
}
pub struct LocalInput {
    inner: Option<Input>,
    caption: String,
    source: Cage<String>,
    placeholder: String,
    hint: String,
    full_width: bool,
}
impl LocalInput {
    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }
    pub fn placeholder(mut self, value: &str) -> Self {
        self.placeholder = value.into();
        self
    }
    pub fn hint(mut self, value: &str) -> Self {
        self.hint = value.into();
        self
    }
    pub fn class(mut self, value: impl Into<glory::Lotus<String>>) -> Self {
        self.inner = Some(self.inner.take().unwrap().class(value));
        self
    }
    pub fn purpose(mut self, value: impl Into<glory::Lotus<glory::render::Purpose>>) -> Self {
        self.inner = Some(self.inner.take().unwrap().purpose(value));
        self
    }
    pub fn multiline(mut self, value: bool) -> Self {
        self.inner = Some(self.inner.take().unwrap().multiline(value));
        self
    }
    pub fn enabled(mut self, value: impl Into<glory::Lotus<bool>>) -> Self {
        self.inner = Some(self.inner.take().unwrap().enabled(value));
        self
    }
    pub fn on(
        mut self,
        event: glory::render::EventKind,
        callback: impl FnMut(glory::render::Event) + 'static,
    ) -> Self {
        self.inner = Some(self.inner.take().unwrap().on(event, callback));
        self
    }
}
impl glory::ui::Widget for LocalInput {
    fn build(&mut self, scope: &Scope) -> glory::ui::Result {
        let content = localized(scope, self.caption.clone());
        let placeholder = localized(scope, self.placeholder.clone());
        let mut inner = view()
            .class(if self.full_width {
                "field full-width"
            } else {
                "field"
            })
            .fill((
                text().class("field-label").content(content.clone()),
                view().class("input-frame").fill((
                    self.inner.take().unwrap().name(content),
                    when(self.source.map(|value| value.is_empty()), move || {
                        text()
                            .class("input-placeholder")
                            .content(placeholder.clone())
                            .selectable(false)
                    }),
                )),
                label(self.hint.clone()).class("field-hint"),
            ));
        inner.build(scope)
    }
}
pub fn localized(scope: &Scope, value: String) -> glory::Lotus<String> {
    match super::context::get(scope) {
        Some(ctx) => {
            let language = ctx.language;
            scope
                .bond(move || crate::utils::i18n::translate_text(&value, language.get()))
                .into()
        }
        None => value.into(),
    }
}
pub fn page(title: &str) -> View {
    let mut page = view().class(if title == "Create user" {
        "page form-page"
    } else if title.starts_with("User ")
        || title.starts_with("Room ")
        || title.starts_with("Account ")
        || title.starts_with("Pasion account ")
        || title.starts_with("Known user ")
    {
        "page detail-page"
    } else {
        "page"
    });
    if title == "Create user" {
        page = page.fill(view().class("form-breadcrumb").fill((
            link().target("/users").label("Users"),
            label("/"),
            label("Create User"),
        )));
    }
    page.fill(
        label(if title == "Create user" {
            "Create New User"
        } else {
            title
        })
        .name(title)
        .kind(glory::render::TextKind::Heading(1)),
    )
}
pub fn modal(key: &str, title: &str, open: Cage<bool>, body: View) -> sapid::Dialog {
    let mut body = Some(body);
    sapid::dialog(key)
        .unstyled()
        .class("operation-dialog")
        .label(title.to_owned())
        .title(title.to_owned())
        .show_trigger(false)
        .open(open)
        .on_open_change(move |value| open.revise(|slot| *slot = value))
        .content_policy(sapid::ContentPolicy::KeepMounted)
        .content(move || body.take().expect("dialog body retained while closed"))
}
pub fn label(value: impl Into<String>) -> LocalText {
    LocalText {
        inner: Some(text()),
        value: value.into(),
        name: None,
    }
}
pub fn field(scope: &Scope, name: &str) -> (Cage<String>, LocalInput) {
    let value = scope.cage(String::new());
    (
        value,
        LocalInput {
            caption: name.into(),
            source: value,
            placeholder: String::new(),
            hint: String::new(),
            full_width: false,
            inner: Some(
                input()
                    .value(value)
                    .on(events::value_changed, move |event| {
                        if let glory::render::Event::ValueChanged { value: next, .. } = event {
                            value.revise(|v| *v = next);
                        }
                    }),
            ),
        },
    )
}
pub fn action(name: &str, callback: impl Fn() + 'static) -> LocalButton {
    let class = match name {
        "English" => "control-language language-en",
        "中文" => "control-language language-zh",
        "Light" => "control-theme theme-light",
        "Dark" => "control-theme theme-dark",
        "System" => "control-theme theme-system",
        "High contrast" => "control-contrast",
        "Keyboard shortcuts" => "control-shortcuts",
        "Sign out" => "control-logout",
        "Create user" | "Create room" => "btn-primary",
        _ => "",
    };
    LocalButton {
        inner: Some(sapid::button().unstyled().class(class)),
        caption: name.into(),
        callback: Some(Box::new(callback)),
    }
}
pub fn status(message: Cage<String>) -> Text {
    if let Some(ctx) = super::context::for_runtime(message.runtime()) {
        let language = ctx.language;
        text().class("status").content(
            message.map(move |value| crate::utils::i18n::translate_text(value, language.get())),
        )
    } else {
        text().class("status").content(message)
    }
}
pub fn spawn(scope: &Scope, future: impl Future<Output = ()> + 'static) {
    let context = super::context::get(scope).map(|ctx| std::rc::Rc::downgrade(&ctx));
    let mut future = Box::pin(future);
    scope
        .spawn(std::future::poll_fn(move |cx| {
            if let Some(ctx) = context.as_ref().and_then(std::rc::Weak::upgrade) {
                super::context::enter(&ctx, || future.as_mut().poll(cx))
            } else {
                future.as_mut().poll(cx)
            }
        }))
        .expect("live page task owner");
}
pub fn submit(
    scope: &Scope,
    busy: Cage<bool>,
    message: Cage<String>,
    future: impl Future<Output = Result<String, HttpError>> + 'static,
) {
    submit_with_toast(scope, busy, message, future, true);
}
pub fn load_quiet(
    scope: &Scope,
    busy: Cage<bool>,
    message: Cage<String>,
    future: impl Future<Output = Result<String, HttpError>> + 'static,
) {
    submit_with_toast(scope, busy, message, future, false);
}
fn submit_with_toast(
    scope: &Scope,
    busy: Cage<bool>,
    message: Cage<String>,
    future: impl Future<Output = Result<String, HttpError>> + 'static,
    announce_success: bool,
) {
    if busy.get() {
        return;
    }
    busy.revise(|v| *v = true);
    message.revise(|v| *v = "Working…".into());
    spawn(scope, async move {
        let result = future.await;
        let failed = result.is_err();
        let text = result.unwrap_or_else(|e| e.to_string());
        if let Some(ctx) = super::context::current().filter(|_| announce_success || failed) {
            ctx.toast(text.clone(), failed);
        }
        let _ = message.try_revise(|v| *v = text);
        let _ = busy.try_revise(|v| *v = false);
    });
}
pub fn confirm(message: &str) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| {
                w.confirm_with_message(
                    &super::context::current()
                        .map(|ctx| crate::utils::i18n::translate_text(message, ctx.language.get()))
                        .unwrap_or_else(|| message.into()),
                )
                .ok()
            })
            .unwrap_or(false)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = message;
        false
    }
}
pub async fn copy(value: &str) -> Result<(), HttpError> {
    #[cfg(target_arch = "wasm32")]
    {
        wasm_bindgen_futures::JsFuture::from(
            web_sys::window()
                .ok_or_else(|| HttpError::message("Browser unavailable"))?
                .navigator()
                .clipboard()
                .write_text(value),
        )
        .await
        .map_err(|_| HttpError::message("Clipboard permission denied"))?;
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = value;
        Err(HttpError::message("Browser required"))
    }
}
pub fn download_text(filename: &str, content: &str, mime: &str) -> Result<(), HttpError> {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        let parts = js_sys::Array::of1(&content.into());
        let options = web_sys::BlobPropertyBag::new();
        options.set_type(mime);
        let blob = web_sys::Blob::new_with_str_sequence_and_options(&parts, &options)
            .map_err(|_| HttpError::message("Unable to create download"))?;
        let url = web_sys::Url::create_object_url_with_blob(&blob)
            .map_err(|_| HttpError::message("Unable to create download URL"))?;
        let result = (|| {
            let doc = web_sys::window()
                .and_then(|w| w.document())
                .ok_or_else(|| HttpError::message("Document unavailable"))?;
            let a = doc
                .create_element("a")
                .map_err(|_| HttpError::message("Download unavailable"))?;
            a.set_attribute("href", &url)
                .map_err(|_| HttpError::message("Download URL failed"))?;
            a.set_attribute("download", filename)
                .map_err(|_| HttpError::message("Download filename failed"))?;
            a.unchecked_into::<web_sys::HtmlElement>().click();
            Ok(())
        })();
        let _ = web_sys::Url::revoke_object_url(&url);
        result
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (filename, content, mime);
        Err(HttpError::message("Browser required"))
    }
}
