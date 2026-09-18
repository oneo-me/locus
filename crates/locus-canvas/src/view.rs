use gpui_kit::component::Theme;
use gpui_kit::{
    AnyElement, App, Context, Entity, EventEmitter, FocusHandle, Focusable, Render, Window, div,
    prelude::*,
};
use oneo_gpui::Language;
use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[cfg(any(target_os = "macos", target_os = "windows"))]
use gpui_wry::WebView;

#[derive(Clone, Debug, Eq, PartialEq)]
enum CanvasError {
    Load(String),
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    Unsupported,
}

impl CanvasError {
    fn label(&self, _language: Language) -> String {
        match self {
            Self::Load(detail) => detail.clone(),
            #[cfg(not(any(target_os = "macos", target_os = "windows")))]
            Self::Unsupported => {
                rust_i18n::t!("canvas.unsupported", locale = _language.locale()).into_owned()
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum CanvasEvent {
    CloseWindow,
    OpenTerminal,
    CommandPalette,
    Settings,
    OpenUrl(String),
}

pub struct CanvasView {
    runtime: Arc<super::CanvasRuntime>,
    language: Language,
    visible: bool,
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    webview: Option<Entity<WebView>>,
    error: Option<CanvasError>,
    loading: bool,
    focus: FocusHandle,
    dark: Arc<AtomicBool>,
    _monitor: Option<gpui_kit::Task<()>>,
}

impl CanvasView {
    pub fn new(
        runtime: Arc<super::CanvasRuntime>,
        directory: &Path,
        language: Language,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let (commands, receiver) = async_channel::bounded::<String>(16);
        cx.spawn_in(window, async move |this, cx| {
            while let Ok(command) = receiver.recv().await {
                if this
                    .update_in(cx, |this, window, cx| {
                        if command == "loaded" {
                            this.loading = false;
                            #[cfg(any(target_os = "macos", target_os = "windows"))]
                            if let Some(view) = &this.webview {
                                let _ = view.read(cx).handle().raw().evaluate_script(
                                    &theme_script(this.dark.load(Ordering::Relaxed)),
                                );
                            }
                            cx.notify();
                            return;
                        }
                        if !this.visible {
                            return;
                        }
                        if let Some(url) = command.strip_prefix("url:") {
                            cx.emit(CanvasEvent::OpenUrl(url.to_owned()));
                            return;
                        }
                        match command.as_str() {
                            "w" => cx.emit(CanvasEvent::CloseWindow),
                            "`" => cx.emit(CanvasEvent::OpenTerminal),
                            "e" => cx.emit(CanvasEvent::CommandPalette),
                            "," => cx.emit(CanvasEvent::Settings),
                            "focus" => window.focus(&this.focus, cx),
                            _ => {}
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        let dark = Theme::global(cx).is_dark();
        let mut tab = Self {
            runtime,
            language,
            visible: false,
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            webview: None,
            error: None,
            loading: true,
            focus: cx.focus_handle(),
            dark: Arc::new(AtomicBool::new(dark)),
            _monitor: None,
        };
        let runtime = tab.runtime.clone();
        let directory = directory.to_path_buf();
        cx.spawn_in(window, async move |this, cx| {
            let result = cx
                .background_spawn(async move { runtime.page_url(&directory) })
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                match result {
                    Ok(url) => this.connect(
                        &format!("{url}?lang={}", this.language.locale()),
                        commands,
                        window,
                        cx,
                    ),
                    Err(error) => {
                        this.loading = false;
                        this.error = Some(CanvasError::Load(error.to_string()));
                    }
                }
                cx.notify();
            });
        })
        .detach();
        tab._monitor = Some(cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(1))
                    .await;
                let alive = this.update(cx, |this, cx| {
                    if this.runtime.has_exited() {
                        this.loading = false;
                        this.error = Some(CanvasError::Load(
                            rust_i18n::t!(
                                "canvas.service_stopped",
                                locale = this.language.locale()
                            )
                            .into_owned(),
                        ));
                        cx.notify();
                        false
                    } else {
                        true
                    }
                });
                if !matches!(alive, Ok(true)) {
                    break;
                }
            }
        }));
        tab
    }

    fn connect(
        &mut self,
        url: &str,
        commands: async_channel::Sender<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            let loading_events = commands.clone();
            let navigation_events = commands.clone();
            let new_window_events = commands.clone();
            let page_url = url.to_owned();
            let builder = wry::WebViewBuilder::new();
            #[cfg(target_os = "windows")]
            let builder = {
                use wry::WebViewBuilderExtWindows;
                builder.with_default_context_menus(false)
            };
            let terminal_message = format!("terminal:{}", uuid::Uuid::new_v4());
            let page_script =
                include_str!("page.js").replace("__LOCUS_TERMINAL_MESSAGE__", &terminal_message);
            match builder
                .with_url(url)
                .with_visible(false)
                .with_focused(false)
                .with_initialization_script(format!(
                    "{}{}",
                    theme_script(self.dark.load(Ordering::Relaxed)),
                    page_script
                ))
                .with_ipc_handler(move |message| {
                    if let Some(command) = canvas_command(message.body(), &terminal_message) {
                        let _ = commands.try_send(command.to_owned());
                    }
                })
                .with_on_page_load_handler(move |event, _| {
                    if matches!(event, wry::PageLoadEvent::Finished) {
                        let _ = loading_events.try_send("loaded".to_owned());
                    }
                })
                .with_navigation_handler(move |url| {
                    if url.split('#').next() == Some(page_url.as_str()) {
                        return true;
                    }
                    forward_external_url(&url, &navigation_events);
                    false
                })
                .with_new_window_req_handler(move |url, _| {
                    forward_external_url(&url, &new_window_events);
                    wry::NewWindowResponse::Deny
                })
                .with_download_started_handler(|_, _| false)
                .build_as_child(window)
            {
                Ok(raw) => {
                    let webview = cx.new(|cx| {
                        let mut view = WebView::new(raw, window, cx);
                        view.hide();
                        view
                    });
                    self.focus = webview.read(cx).focus_handle(cx);
                    self.webview = Some(webview);
                }
                Err(error) => {
                    self.loading = false;
                    self.error = Some(CanvasError::Load(error.to_string()));
                }
            }
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = (url, commands, window, cx);
            self.loading = false;
            self.error = Some(CanvasError::Unsupported);
        }
    }

    pub fn has_error(&self) -> bool {
        self.error.is_some()
    }

    pub fn reload(&mut self, cx: &mut Context<Self>) {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        if let Some(view) = &self.webview {
            if let Err(error) = view.read(cx).handle().raw().reload() {
                self.error = Some(CanvasError::Load(error.to_string()));
            } else {
                self.error = None;
            }
            cx.notify();
        }
    }

    pub fn synchronize(&mut self, visible: bool, language: Language, cx: &mut Context<Self>) {
        let dark = Theme::global(cx).is_dark();
        self.visible = visible;
        if self.language != language {
            self.language = language;
            cx.notify();
        }
        let visible = visible && self.error.is_none() && !self.loading;
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        if let Some(view) = &self.webview {
            view.update(cx, |view, cx| {
                if view.visible() != visible {
                    if visible {
                        view.show();
                    } else {
                        view.hide();
                    }
                    cx.notify();
                }
            });
            if self.dark.load(Ordering::Relaxed) != dark {
                let handle = view.read(cx).handle();
                let _ = handle.raw().evaluate_script(&theme_script(dark));
            }
        }
        self.dark.store(dark, Ordering::Relaxed);
    }

    fn render_content(&self, language: Language) -> AnyElement {
        if self.loading && self.error.is_none() {
            return div()
                .size_full()
                .track_focus(&self.focus)
                .flex()
                .items_center()
                .justify_center()
                .child(gpui_kit::component::spinner::Spinner::new())
                .into_any_element();
        }
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        if let Some(view) = &self.webview
            && self.error.is_none()
        {
            return view.clone().into_any_element();
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .justify_center()
            .p_4()
            .gap_2()
            .track_focus(&self.focus)
            .child(rust_i18n::t!("canvas.error", locale = language.locale()).into_owned())
            .child(
                self.error
                    .as_ref()
                    .map(|error| error.label(language))
                    .unwrap_or_default(),
            )
            .into_any_element()
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn canvas_command<'a>(message: &'a str, terminal_message: &str) -> Option<&'a str> {
    match message {
        "w" | "e" | "," | "focus" => Some(message),
        _ if message == terminal_message => Some("`"),
        _ => None,
    }
}

impl EventEmitter<CanvasEvent> for CanvasView {}

impl Focusable for CanvasView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for CanvasView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.render_content(self.language)
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn forward_external_url(url: &str, events: &async_channel::Sender<String>) {
    if url.starts_with("http://127.0.0.1:") {
        return;
    }
    if let Some((scheme, address)) = url.split_once("://")
        && (scheme.eq_ignore_ascii_case("https") || scheme.eq_ignore_ascii_case("http"))
        && !address.is_empty()
    {
        let _ = events.try_send(format!("url:{url}"));
    }
}

fn theme_script(dark: bool) -> String {
    let theme = if dark { "dark" } else { "light" };
    format!(
        "(() => {{ const apply = () => {{ document.documentElement.classList.remove('dark','light');document.documentElement.classList.add('{theme}');document.documentElement.style.colorScheme='{theme}'; }}; if(document.documentElement) apply(); else document.addEventListener('DOMContentLoaded',apply,{{once:true}}); }})();"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_commands_require_the_terminal_token() {
        assert_eq!(canvas_command("`", "terminal:token"), None);
        assert_eq!(
            canvas_command("terminal:token", "terminal:token"),
            Some("`")
        );
        assert_eq!(canvas_command("focus", "terminal:token"), Some("focus"));
    }
}
