use gpui_kit::component::Theme;
use gpui_kit::{
    AnyElement, App, Context, Entity, EventEmitter, FocusHandle, Focusable, Render, Window, div,
    prelude::*,
};
use oneo_gpui::Language;
use std::path::Path;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

#[cfg(target_os = "macos")]
use oneo_gpui_webview::{NewWindowResponse, PageLoadEvent, WebView, WebViewBuilder};

#[derive(Clone, Debug, Eq, PartialEq)]
enum CanvasError {
    Load(String),
    #[cfg(not(target_os = "macos"))]
    Unsupported,
}

impl CanvasError {
    fn label(&self, _language: Language) -> String {
        match self {
            Self::Load(detail) => detail.clone(),
            #[cfg(not(target_os = "macos"))]
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
    root: std::path::PathBuf,
    directory: std::path::PathBuf,
    dependencies: Arc<Mutex<super::export::Dependencies>>,
    language: Language,
    visible: bool,
    #[cfg(target_os = "macos")]
    webview: Option<Entity<WebView>>,
    error: Option<CanvasError>,
    loading: bool,
    focus: FocusHandle,
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
                            #[cfg(target_os = "macos")]
                            if let Some(view) = &this.webview {
                                let _ = view.read(cx).evaluate_script(&theme_script(
                                    this.dark.load(Ordering::Relaxed),
                                ));
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
            root: root.to_path_buf(),
            directory: directory.to_path_buf(),
            dependencies: Arc::new(Mutex::new(super::export::Dependencies::default())),
            language,
            visible: false,
            #[cfg(target_os = "macos")]
            webview: None,
            error: None,
            loading: true,
            focus: cx.focus_handle(),
            dark: Arc::new(AtomicBool::new(dark)),
        };
        tab.connect(root, directory, commands, window, cx);
        tab
    }

    fn connect(
        &mut self,
        root: &Path,
        directory: &Path,
        commands: async_channel::Sender<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        #[cfg(target_os = "macos")]
        {
            let loading_events = commands.clone();
            let navigation_events = commands.clone();
            let new_window_events = commands.clone();
            let root = root.to_path_buf();
            let directory = directory.to_path_buf();
            let page_dark = self.dark.clone();
            let language = self.language;
            let executor = cx.background_executor().clone();
            let dependencies = self.dependencies.clone();
            let builder = WebViewBuilder::new();
            let terminal_message = format!("terminal:{}", uuid::Uuid::new_v4());
            let page_script =
                include_str!("page.js").replace("__LOCUS_TERMINAL_MESSAGE__", &terminal_message);
            let builder = builder
                .with_asynchronous_custom_protocol(
                    "locus-canvas".to_owned(),
                    move |_, request, responder| {
                        let root = root.clone();
                        let directory = directory.clone();
                        let page_dark = page_dark.clone();
                        let dependencies = dependencies.clone();
                        executor
                            .spawn(async move {
                                let response = canvas_response(
                                    &root,
                                    &directory,
                                    &request,
                                    page_dark.load(Ordering::Relaxed),
                                    language,
                                );
                                if response.status().is_success() {
                                    let mut dependencies = dependencies.lock().unwrap();
                                    if request.uri().path() == "/canvas.html" {
                                        *dependencies = super::export::Dependencies::default();
                                    } else {
                                        let path = request
                                            .uri()
                                            .path_and_query()
                                            .map(|value| value.as_str())
                                            .unwrap_or_default();
                                        let name = path.trim_start_matches('/');
                                        let name =
                                            if name.starts_with(super::resources::LIBRARY_PREFIX) {
                                                name.to_owned()
                                            } else {
                                                percent_encoding::percent_decode_str(name)
                                                    .decode_utf8()
                                                    .unwrap_or_default()
                                                    .into_owned()
                                            };
                                        if request
                                            .headers()
                                            .get("sec-fetch-dest")
                                            .is_some_and(|value| value == "style")
                                        {
                                            dependencies.stylesheets.insert(name.clone());
                                        }
                                        dependencies.resources.insert(name);
                                    }
                                }
                                responder.respond(response);
                            })
                            .detach();
                    },
                )
                .with_url(CANVAS_URL)
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
                    if matches!(event, PageLoadEvent::Finished) {
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
                    NewWindowResponse::Deny
                })
                .with_download_started_handler(|_, _| false);
            match WebView::new(builder, window, cx) {
                Ok(view) => {
                    self.focus = view.focus_handle(cx);
                    self.webview = Some(cx.new(|_| view));
                }
                Err(error) => {
                    self.loading = false;
                    self.error = Some(CanvasError::Load(error.to_string()));
                }
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (root, directory, commands, window, cx);
            self.loading = false;
            self.error = Some(CanvasError::Unsupported);
        }
    }

    pub fn prepare_download(
        &self,
        cx: &App,
    ) -> gpui_kit::Task<std::io::Result<super::CanvasDownload>> {
        let root = self.root.clone();
        let directory = self.directory.clone();
        let dependencies = self.dependencies.lock().unwrap().clone();
        let dark = self.dark.load(Ordering::Relaxed);
        cx.background_executor().spawn(async move {
            super::export::prepare_download(&root, &directory, dark, dependencies)
        })
    }

    pub fn has_error(&self) -> bool {
        self.error.is_some()
    }

    pub fn reload(&mut self, cx: &mut Context<Self>) {
        #[cfg(target_os = "macos")]
        if let Some(view) = &self.webview {
            if let Err(error) = view.read(cx).reload() {
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
        #[cfg(target_os = "macos")]
        if let Some(view) = &self.webview {
            view.update(cx, |view, cx| {
                if view.is_visible() != visible {
                    view.set_visible(visible);
                    cx.notify();
                }
            });
            if self.dark.load(Ordering::Relaxed) != dark {
                let _ = view.read(cx).evaluate_script(&theme_script(dark));
            }
        }
        self.dark.store(dark, Ordering::Relaxed);
    }

    fn render_content(&self, language: Language, cx: &App) -> AnyElement {
        if self.loading && self.error.is_none() {
            return div()
                .bg(Theme::global(cx).background)
                .size_full()
                .track_focus(&self.focus)
                .flex()
                .items_center()
                .justify_center()
                .child(gpui_kit::component::spinner::Spinner::new())
                .into_any_element();
        }
        #[cfg(target_os = "macos")]
        if let Some(view) = &self.webview
            && self.error.is_none()
        {
            return view.clone().into_any_element();
        }
        div()
            .bg(Theme::global(cx).background)
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

#[cfg(target_os = "macos")]
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
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_content(self.language, cx)
    }
}

#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
const CANVAS_URL: &str = "locus-canvas://localhost/canvas.html";

#[cfg(target_os = "macos")]
fn is_canvas_url(url: &str) -> bool {
    matches!(
        url.split('#').next(),
        Some(CANVAS_URL | "http://locus-canvas.localhost/canvas.html")
    )
}

#[cfg(target_os = "macos")]
fn canvas_response(
    root: &Path,
    directory: &Path,
    request: &wry::http::Request<Vec<u8>>,
    dark: bool,
    language: Language,
) -> wry::http::Response<Vec<u8>> {
    use wry::http::{Method, Response, StatusCode};
    if request.method() == Method::GET && request.uri().path() != "/canvas.html" {
        let name = request
            .uri()
            .path_and_query()
            .map(|value| value.as_str())
            .unwrap_or_default()
            .trim_start_matches('/');
        let decoded;
        let name = if name.starts_with(super::resources::LIBRARY_PREFIX) {
            name
        } else {
            decoded = percent_encoding::percent_decode_str(name)
                .decode_utf8()
                .unwrap_or_default();
            &decoded
        };
        let resource = super::resources::read_resource(root, directory, name)
            .ok()
            .map(|bytes| {
                let mime = super::resources::resource_mime(name);
                let bytes = if matches!(
                    mime.as_str(),
                    "text/css"
                        | "text/javascript"
                        | "application/javascript"
                        | "text/html"
                        | "image/svg+xml"
                ) {
                    std::str::from_utf8(&bytes)
                        .map(|text| super::resources::rewrite_resources(text).into_bytes())
                        .unwrap_or(bytes)
                } else {
                    bytes
                };
                (bytes, mime)
            });
        let (status, bytes, mime) = match resource {
            Some((bytes, mime)) => (StatusCode::OK, bytes, mime),
            None => (StatusCode::NOT_FOUND, Vec::new(), "text/plain".to_owned()),
        };
        return Response::builder()
            .status(status)
            .header("Content-Type", mime)
            .header("Cache-Control", "no-store")
            .header("X-Content-Type-Options", "nosniff")
            .header("Content-Security-Policy", CANVAS_POLICY)
            .body(bytes)
            .expect("canvas resource response headers are valid");
    }
    let (status, document) = if request.method() != Method::GET {
        (StatusCode::NOT_FOUND, String::new())
    } else {
        match locus_library::resolve_canvas(root, directory)
            .and_then(|canvas| resolved_document(root, &canvas, dark))
        {
            Ok(document) => (StatusCode::OK, document),
            Err(error) => {
                let message = rust_i18n::t!("canvas.read_error", locale = language.locale());
                let detail = error
                    .to_string()
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;");
                (
                    StatusCode::NOT_FOUND,
                    canvas_document(
                        &format!("<main class=\"p-4\"><p>{message}</p><pre>{detail}</pre></main>"),
                        dark,
                    ),
                )
            }
        }
    };
    Response::builder()
        .status(status)
        .header("Content-Type", "text/html; charset=utf-8")
        .header("Cache-Control", "no-store")
        .body(document.into_bytes())
        .expect("static canvas response headers are valid")
}

fn theme_script(dark: bool) -> String {
    let theme = if dark { "dark" } else { "light" };
    format!(
        "(() => {{ const apply = () => {{ document.documentElement.classList.remove('dark','light');document.documentElement.classList.add('{theme}');document.documentElement.style.colorScheme='{theme}'; }}; if(document.documentElement) apply(); else document.addEventListener('DOMContentLoaded',apply,{{once:true}}); }})();"
    )
}

const CANVAS_POLICY: &str = "default-src 'none'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' https: data: blob:; font-src 'self' data:; media-src 'self' data: blob:; connect-src 'self' https:; worker-src 'self' blob:; frame-src 'none'; object-src 'none'; base-uri 'self'; form-action 'none'";

pub(crate) fn canvas_document(source: &str, dark: bool) -> String {
    canvas_document_with_data(source, dark, "./", &serde_json::Value::Null, "", "")
}

pub(crate) fn resolved_document(
    root: &Path,
    canvas: &locus_library::CanvasSource,
    dark: bool,
) -> std::io::Result<String> {
    let base = super::resources::canvas_base(root, canvas.directory())?;
    Ok(canvas_document_with_data(
        canvas.html(),
        dark,
        &base,
        canvas.data(),
        "",
        &canvas_entry(),
    ))
}

pub(crate) fn canvas_entry() -> String {
    let lifecycle = include_str!("main.js");
    format!(
        "<script>{lifecycle}\nwindow.addEventListener('load', () => runCanvas(() => import(new URL('main.js', document.baseURI).href)), {{once:true}});</script>"
    )
}

pub(crate) fn canvas_document_with_data(
    source: &str,
    dark: bool,
    base: &str,
    data: &serde_json::Value,
    bootstrap: &str,
    entry: &str,
) -> String {
    let source = super::resources::rewrite_resources(source);
    let data = serde_json::to_string(data).unwrap().replace('<', "\\u003c");
    let tailwind = include_str!("../assets/tailwindcss/browser-4.3.3.js");
    let runtime = include_str!("runtime.js");
    let theme = if dark { "dark" } else { "light" };
    format!(
        r#"<!doctype html><html class="{theme}" style="color-scheme:{theme}"><head>
<base href="{base}"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<meta http-equiv="Content-Security-Policy" content="{CANVAS_POLICY}">
<style>html,body{{margin:0;width:100%;height:100%;}}body{{overflow-wrap:anywhere;}}</style>
<style type="text/tailwindcss">@custom-variant dark (&:where(.dark, .dark *));</style>
{bootstrap}<script>{tailwind}</script><script>{runtime}</script></head><body>{source}<script type="application/json" id="locus-canvas-data">{data}</script>{entry}</body></html>"#
    )
}

#[cfg(test)]
mod tests {
    fn write_fixture(
        path: impl AsRef<std::path::Path>,
        contents: impl AsRef<[u8]>,
    ) -> std::io::Result<()> {
        let path = path.as_ref();
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(path, contents)
    }

    use super::*;

    #[cfg(target_os = "macos")]
    #[test]
    fn terminal_ipc_requires_the_private_shortcut_message() {
        let token = "terminal:private-token";
        for message in ["`", "terminal", "terminal:another-token", "codex", ""] {
            assert_eq!(canvas_command(message, token), None);
        }
        assert_eq!(canvas_command(token, token), Some("`"));
        for message in ["w", "e", ",", "focus"] {
            assert_eq!(canvas_command(message, token), Some(message));
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn template_entry_uses_its_own_resource_base_and_passes_data() {
        use wry::http::{Request, StatusCode};
        let root = tempfile::tempdir().unwrap();
        let caller = root.path().join("caller");
        write_fixture(caller.join("index.md"), "Caller").unwrap();
        write_fixture(
            caller.join("canvas/index.json"),
            r#"{"template":"locus://template","data":{"answer":"</script>"}}"#,
        )
        .unwrap();
        let template = root.path().join("template");
        write_fixture(template.join("index.md"), "Template").unwrap();
        write_fixture(template.join("canvas/index.html"), "<output></output>").unwrap();
        write_fixture(
            template.join("canvas/main.js"),
            "export async function main(data) {}",
        )
        .unwrap();
        let request = Request::builder().uri(CANVAS_URL).body(Vec::new()).unwrap();
        let response = canvas_response(root.path(), &caller, &request, false, Language::English);
        assert_eq!(response.status(), StatusCode::OK);
        let html = std::str::from_utf8(response.body()).unwrap();
        assert!(html.contains("<base href=\"/_locus/resource/template/canvas/\">"));
        assert!(html.contains(r#"{"answer":"\u003c/script>"}"#));
        assert!(!html.contains("loadCanvas"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn refresh_reads_the_latest_file_from_the_original_note_and_recovers_from_removal() {
        use std::fs;
        use wry::http::{Request, StatusCode};
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("first");
        fs::create_dir(&directory).unwrap();
        write_fixture(directory.join("index.md"), "First note").unwrap();
        let file = directory.join(locus_library::CANVAS_FILE);
        write_fixture(&file, "<p>First version</p>").unwrap();
        write_fixture(
            directory.join("canvas/main.js"),
            "export async function main(data) {}",
        )
        .unwrap();
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
        write_fixture(other.join("index.md"), "Other note").unwrap();
        write_fixture(
            other.join(locus_library::CANVAS_FILE),
            "<p>Other canvas</p>",
        )
        .unwrap();
        write_fixture(&file, "<p>Latest version</p>").unwrap();
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
        write_fixture(&file, "<p>Restored</p>").unwrap();
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
        write_fixture(&file, " ").unwrap();
        assert_eq!(
            canvas_response(root.path(), &directory, &request, false, Language::English).status(),
            StatusCode::NOT_FOUND
        );
    }

    #[cfg(target_os = "macos")]
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

    #[cfg(target_os = "macos")]
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

    #[cfg(target_os = "macos")]
    #[test]
    fn relative_images_are_served_as_bytes_with_decoded_names() {
        use wry::http::{Request, StatusCode};
        let root = tempfile::tempdir().unwrap();
        write_fixture(root.path().join("index.md"), "Note").unwrap();
        write_fixture(root.path().join("canvas/照片 1.png"), [0, 128, 255]).unwrap();
        let request = Request::builder()
            .uri(
                "/".to_owned()
                    + &percent_encoding::utf8_percent_encode(
                        "照片 1.png",
                        percent_encoding::NON_ALPHANUMERIC,
                    )
                    .to_string()
                    + "?v=1",
            )
            .body(Vec::new())
            .unwrap();
        let response =
            canvas_response(root.path(), root.path(), &request, false, Language::English);
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["Content-Type"], "image/png");
        assert_eq!(response.headers()["Content-Security-Policy"], CANVAS_POLICY);
        assert_eq!(response.headers()["Cache-Control"], "no-store");
        assert_eq!(response.body(), &[0, 128, 255]);
        for uri in [
            "/%2e%2e%2fphoto.png",
            "/%2fphoto.png",
            "/%5cphoto.png",
            "/%FF.png",
            "/%69ndex.md",
            "/.secret",
            "/missing.png",
        ] {
            let request = Request::builder().uri(uri).body(Vec::new()).unwrap();
            assert_eq!(
                canvas_response(root.path(), root.path(), &request, false, Language::English)
                    .status(),
                StatusCode::NOT_FOUND,
                "{uri}"
            );
        }
        let request = Request::builder()
            .method("POST")
            .uri("/photo.png")
            .body(Vec::new())
            .unwrap();
        assert_eq!(
            canvas_response(root.path(), root.path(), &request, false, Language::English).status(),
            StatusCode::NOT_FOUND
        );
    }

    #[test]
    fn document_contains_offline_tailwind_and_preserves_inline_programs() {
        let source = "<style>.test{color:red}</style><button class=\"p-4\">Run</button><script>window.value=42;</script>";
        let document = canvas_document(source, true);
        assert!(document.contains(source));
        assert!(document.contains("@custom-variant dark"));
        assert!(document.contains("<html class=\"dark\""));
        assert!(!document.contains("<script src="));
        assert!(document.contains("connect-src 'self' https:"));
        assert!(document.contains("worker-src 'self' blob:"));
        assert!(document.contains("img-src 'self' https: data: blob:"));
        assert!(document.contains("frame-src 'none'"));
    }
}
