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
    Close,
    CommandPalette,
    Settings,
    OpenUrl(String),
}

pub struct CanvasView {
    language: Language,
    visible: bool,
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    webview: Option<Entity<WebView>>,
    error: Option<CanvasError>,
    loading: bool,
    focus: FocusHandle,
    focus_pending: bool,
    dark: Arc<AtomicBool>,
}

impl CanvasView {
    pub fn new(
        root: &Path,
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
                            "w" => cx.emit(CanvasEvent::Close),
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
            language,
            visible: false,
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            webview: None,
            error: None,
            loading: true,
            focus: cx.focus_handle(),
            focus_pending: false,
            dark: Arc::new(AtomicBool::new(dark)),
        };
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        let loading_events = commands.clone();
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        let navigation_events = commands.clone();
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        let new_window_events = commands.clone();
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        let builder = wry::WebViewBuilder::new();
        #[cfg(target_os = "windows")]
        let builder = {
            use wry::WebViewBuilderExtWindows;
            builder.with_default_context_menus(false)
        };
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        let root = root.to_path_buf();
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        let directory = directory.to_path_buf();
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        let page_dark = tab.dark.clone();
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        let executor = cx.background_executor().clone();
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        match builder
            .with_asynchronous_custom_protocol(
                "locus-canvas".to_owned(),
                move |_, request, responder| {
                    let root = root.clone();
                    let directory = directory.clone();
                    let page_dark = page_dark.clone();
                    executor
                        .spawn(async move {
                            responder.respond(canvas_response(
                                &root,
                                &directory,
                                &request,
                                page_dark.load(Ordering::Relaxed),
                                language,
                            ));
                        })
                        .detach();
                },
            )
            .with_url(CANVAS_URL)
            .with_visible(false)
            .with_focused(false)
            .with_initialization_script(include_str!("page.js"))
            .with_ipc_handler(move |message| {
                if matches!(message.body().as_str(), "w" | "e" | "," | "focus") {
                    let _ = commands.try_send(message.into_body());
                }
            })
            .with_on_page_load_handler(move |event, _| {
                if matches!(event, wry::PageLoadEvent::Finished) {
                    let _ = loading_events.try_send("loaded".to_owned());
                }
            })
            .with_navigation_handler(move |url| {
                if is_canvas_url(&url) {
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
                tab.focus = webview.read(cx).focus_handle(cx);
                tab.webview = Some(webview);
            }
            Err(error) => tab.error = Some(CanvasError::Load(error.to_string())),
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = (root, directory, language, window, commands);
            tab.error = Some(CanvasError::Unsupported);
        }
        tab
    }

    pub fn request_focus(&mut self) {
        self.focus_pending = true;
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
            if visible && self.focus_pending {
                self.focus_pending = false;
                let handle = view.read(cx).handle();
                let _ = handle.raw().focus();
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
    if url.starts_with("http://locus-canvas.localhost") {
        return;
    }
    if let Some((scheme, address)) = url.split_once("://")
        && (scheme.eq_ignore_ascii_case("https") || scheme.eq_ignore_ascii_case("http"))
        && !address.is_empty()
    {
        let _ = events.try_send(format!("url:{url}"));
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
const CANVAS_URL: &str = "locus-canvas://localhost/canvas.html";

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn is_canvas_url(url: &str) -> bool {
    matches!(
        url.split('#').next(),
        Some(CANVAS_URL | "http://locus-canvas.localhost/canvas.html")
    )
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn canvas_response(
    root: &Path,
    directory: &Path,
    request: &wry::http::Request<Vec<u8>>,
    dark: bool,
    language: Language,
) -> wry::http::Response<Vec<u8>> {
    use wry::http::{Method, Response, StatusCode};
    let (status, source) = if request.method() != Method::GET
        || request.uri().path() != "/canvas.html"
    {
        (StatusCode::NOT_FOUND, String::new())
    } else {
        match locus_library::load_canvas(root, directory) {
            Ok(source) if !source.trim().is_empty() => (StatusCode::OK, source),
            result => {
                let message = match result {
                    Ok(_) => rust_i18n::t!("canvas.empty", locale = language.locale()).into_owned(),
                    Err(_) => {
                        rust_i18n::t!("canvas.read_error", locale = language.locale()).into_owned()
                    }
                };
                (
                    StatusCode::NOT_FOUND,
                    format!("<main class=\"p-4\"><p>{message}</p></main>"),
                )
            }
        }
    };
    Response::builder()
        .status(status)
        .header("Content-Type", "text/html; charset=utf-8")
        .header("Cache-Control", "no-store")
        .body(canvas_document(&source, dark).into_bytes())
        .expect("static canvas response headers are valid")
}

fn theme_script(dark: bool) -> String {
    let theme = if dark { "dark" } else { "light" };
    format!(
        "document.documentElement.classList.remove('dark','light');document.documentElement.classList.add('{theme}');document.documentElement.style.colorScheme='{theme}';"
    )
}

fn canvas_document(source: &str, dark: bool) -> String {
    let tailwind = include_str!("../assets/tailwindcss/browser-4.3.3.js");
    let theme = if dark { "dark" } else { "light" };
    format!(
        r#"<!doctype html><html class="{theme}" style="color-scheme:{theme}"><head>
<meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; img-src data: blob:; font-src data:; media-src data: blob:; connect-src 'none'; frame-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'">
<style>html,body{{margin:0;width:100%;height:100%;}}body{{overflow-wrap:anywhere;}}</style>
<style type="text/tailwindcss">@custom-variant dark (&:where(.dark, .dark *));</style>
<script>{tailwind}</script></head><body>{source}</body></html>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    #[test]
    fn refresh_reads_the_latest_file_from_the_original_note_and_recovers_from_removal() {
        use std::fs;
        use wry::http::{Request, StatusCode};
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("first");
        fs::create_dir(&directory).unwrap();
        fs::write(directory.join("index.md"), "First note").unwrap();
        let file = directory.join(locus_library::CANVAS_FILE);
        fs::write(&file, "<p>First version</p>").unwrap();
        let request = Request::builder().uri(CANVAS_URL).body(Vec::new()).unwrap();
        let response = canvas_response(root.path(), &directory, &request, false, Language::English);
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["Cache-Control"], "no-store");
        assert!(
            String::from_utf8(response.into_body())
                .unwrap()
                .contains("<p>First version</p>")
        );
        let other = root.path().join("other");
        fs::create_dir(&other).unwrap();
        fs::write(other.join("index.md"), "Other note").unwrap();
        fs::write(
            other.join(locus_library::CANVAS_FILE),
            "<p>Other canvas</p>",
        )
        .unwrap();
        fs::write(&file, "<p>Latest version</p>").unwrap();
        let response = canvas_response(root.path(), &directory, &request, true, Language::English);
        let html = String::from_utf8(response.into_body()).unwrap();
        assert!(html.contains("<p>Latest version</p>"));
        assert!(!html.contains("<p>Other canvas</p>"));
        assert!(html.contains("<html class=\"dark\""));
        fs::remove_file(&file).unwrap();
        assert_eq!(
            canvas_response(root.path(), &directory, &request, false, Language::English).status(),
            StatusCode::NOT_FOUND
        );
        fs::write(&file, "<p>Restored</p>").unwrap();
        assert_eq!(
            canvas_response(root.path(), &directory, &request, false, Language::English).status(),
            StatusCode::OK
        );
        for path in ["/index.md", "/../other/canvas.html", "/file.txt"] {
            let request = Request::builder().uri(path).body(Vec::new()).unwrap();
            assert_eq!(
                canvas_response(root.path(), &directory, &request, false, Language::English)
                    .status(),
                StatusCode::NOT_FOUND
            );
        }
        fs::write(&file, " ").unwrap();
        assert_eq!(
            canvas_response(root.path(), &directory, &request, false, Language::English).status(),
            StatusCode::NOT_FOUND
        );
    }

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    #[test]
    fn navigation_allows_page_reload_and_fragments_but_no_other_internal_page() {
        for url in [
            CANVAS_URL,
            "locus-canvas://localhost/canvas.html#section",
            "http://locus-canvas.localhost/canvas.html",
        ] {
            assert!(is_canvas_url(url));
        }
        for url in [
            "locus-canvas://localhost/index.md",
            "https://example.com",
            "about:blank",
        ] {
            assert!(!is_canvas_url(url));
        }
    }

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    #[test]
    fn external_navigation_forwards_web_links_only() {
        let (sender, receiver) = async_channel::bounded::<String>(16);
        for url in [
            "https://example.com/a?q=1#part",
            "http://example.com",
            "HTTPS://example.com",
        ] {
            forward_external_url(url, &sender);
            assert_eq!(receiver.try_recv().unwrap(), format!("url:{url}"));
        }
        for url in [
            "about:blank",
            "about:blank#part",
            "#part",
            "file:///tmp/test",
            "javascript:alert(1)",
            "data:text/html,test",
            "locus://standard",
            "https://",
        ] {
            forward_external_url(url, &sender);
            assert!(receiver.try_recv().is_err());
        }
    }

    #[test]
    fn document_contains_offline_tailwind_and_preserves_inline_programs() {
        let source = "<style>.test{color:red}</style><button class=\"p-4\">Run</button><script>window.value=42;</script>";
        let document = canvas_document(source, true);
        assert!(document.contains(source));
        assert!(document.contains("@custom-variant dark"));
        assert!(document.contains("<html class=\"dark\""));
        assert!(!document.contains("<script src="));
        assert!(document.contains("connect-src 'none'"));
    }
}
