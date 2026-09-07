use super::*;
use gpui_kit::Focusable;
use locus_ui::CanvasMetadata;

#[cfg(any(target_os = "macos", target_os = "windows"))]
use gpui_wry::WebView;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum CanvasError {
    Load(String),
    Missing,
    Empty,
    Ambiguous,
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    Unsupported,
}

impl CanvasError {
    fn label(&self, language: Language) -> String {
        let key = match self {
            Self::Load(detail) => return detail.clone(),
            Self::Missing => "workspace.canvas.missing",
            Self::Empty => "workspace.canvas.empty",
            Self::Ambiguous => "workspace.canvas.ambiguous",
            #[cfg(not(any(target_os = "macos", target_os = "windows")))]
            Self::Unsupported => "workspace.canvas.unsupported",
        };
        rust_i18n::t!(key, locale = language.locale()).into_owned()
    }
}

pub(super) struct CanvasTab {
    pub(super) path: String,
    pub(super) title: String,
    pub(super) source: String,
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    webview: Option<Entity<WebView>>,
    error: Option<CanvasError>,
    loading: bool,
    focus: FocusHandle,
    focus_pending: bool,
    pub(super) reload_generation: u64,
    dark: bool,
}

impl CanvasTab {
    pub(super) fn new(
        path: &str,
        title: &str,
        source: &str,
        commands: async_channel::Sender<String>,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let dark = Theme::global(cx).is_dark();
        let mut tab = Self {
            path: path.to_owned(),
            title: title.to_owned(),
            source: source.to_owned(),
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            webview: None,
            error: None,
            loading: true,
            focus: cx.focus_handle(),
            focus_pending: false,
            reload_generation: 0,
            dark,
        };
        if source.trim().is_empty() {
            tab.error = Some(CanvasError::Empty);
            return tab;
        }
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
            builder
                .with_default_context_menus(false)
                .with_browser_accelerator_keys(false)
        };
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        match builder
            .with_html(canvas_document(source, dark))
            .with_visible(false)
            .with_focused(false)
            .with_initialization_script(include_str!("canvas.js"))
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
                forward_external_url(&url, &navigation_events);
                url == "about:blank"
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
            let _ = (window, commands);
            tab.error = Some(CanvasError::Unsupported);
        }
        tab
    }

    pub(super) fn focus_handle(&self) -> FocusHandle {
        self.focus.clone()
    }

    pub(super) fn request_focus(&mut self) {
        self.focus_pending = true;
    }

    pub(super) fn has_error(&self) -> bool {
        self.error.is_some()
    }

    pub(super) fn finish_loading(&mut self) {
        self.loading = false;
    }

    pub(super) fn label(&self) -> String {
        CanvasMetadata::parse(&self.title)
            .expect("canvas metadata is validated before opening")
            .title()
            .to_owned()
    }

    pub(super) fn matches(&self, path: &str, header: &str, source: &str) -> bool {
        self.path == path
            && self.source == source
            && CanvasMetadata::parse(header)
                .is_some_and(|metadata| metadata.title() == self.label())
    }

    pub(super) fn refresh(
        &mut self,
        document: Result<MarkdownDocument, CanvasError>,
        cx: &mut App,
    ) {
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        if self.webview.is_none() {
            return;
        }
        let source = document.and_then(|document| {
            resolve_canvas_source(&document, &self.title, &self.source).map(str::to_owned)
        });
        match source {
            Ok(source) => {
                if source != self.source || self.error.is_some() {
                    #[cfg(any(target_os = "macos", target_os = "windows"))]
                    if let Some(view) = &self.webview {
                        self.loading = true;
                        let handle = view.read(cx).handle();
                        if let Err(error) =
                            handle.raw().load_html(&canvas_document(&source, self.dark))
                        {
                            self.error = Some(CanvasError::Load(error.to_string()));
                            return;
                        }
                    }
                    self.source = source;
                    self.error = None;
                }
            }
            Err(error) => self.error = Some(error),
        }
    }

    pub(super) fn synchronize(&mut self, visible: bool, dark: bool, cx: &mut App) {
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
            if self.dark != dark {
                let handle = view.read(cx).handle();
                let _ = handle.raw().evaluate_script(&theme_script(dark));
            }
            if visible && self.focus_pending {
                self.focus_pending = false;
                let handle = view.read(cx).handle();
                let _ = handle.raw().focus();
            }
        }
        self.dark = dark;
    }

    pub(super) fn render(&self, language: Language) -> AnyElement {
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
            .child(rust_i18n::t!("workspace.canvas.error", locale = language.locale()).into_owned())
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
fn forward_external_url(url: &str, events: &async_channel::Sender<String>) {
    if let Some((scheme, address)) = url.split_once("://")
        && (scheme.eq_ignore_ascii_case("https") || scheme.eq_ignore_ascii_case("http"))
        && !address.is_empty()
    {
        let _ = events.try_send(format!("url:{url}"));
    }
}

fn resolve_canvas_source<'a>(
    document: &'a MarkdownDocument,
    title: &str,
    source: &str,
) -> Result<&'a str, CanvasError> {
    let candidates = document
        .code_blocks()
        .into_iter()
        .filter(|(language, candidate, _)| {
            *language == "canvas"
                && CanvasMetadata::parse(candidate)
                    .zip(CanvasMetadata::parse(title))
                    .is_some_and(|(candidate, current)| candidate.title() == current.title())
        })
        .map(|(_, _, source)| source)
        .collect::<Vec<_>>();
    if let Some(unchanged) = candidates.iter().find(|candidate| **candidate == source) {
        return Ok(unchanged);
    }
    match candidates.as_slice() {
        [source] if source.trim().is_empty() => Err(CanvasError::Empty),
        [source] => Ok(source),
        [] => Err(CanvasError::Missing),
        _ => Err(CanvasError::Ambiguous),
    }
}

fn theme_script(dark: bool) -> String {
    let theme = if dark { "dark" } else { "light" };
    format!(
        "document.documentElement.classList.remove('dark','light');document.documentElement.classList.add('{theme}');document.documentElement.style.colorScheme='{theme}';"
    )
}

fn canvas_document(source: &str, dark: bool) -> String {
    let tailwind = include_str!("../../assets/tailwindcss/browser-4.3.3.js");
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
    fn external_navigation_forwards_web_links_only() {
        let (sender, receiver) = async_channel::bounded(16);
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
    fn refresh_follows_a_named_canvas_and_rejects_removed_or_ambiguous_sources() {
        let document = MarkdownDocument::parse(
            "```canvas Demo | Example\n<button>New</button>\n```\n\n```canvas Other | Other example\nOther\n```",
        );
        assert_eq!(
            resolve_canvas_source(&document, "Demo | Example", "Old"),
            Ok("<button>New</button>")
        );
        assert_eq!(
            resolve_canvas_source(&document, "Gone", "Old"),
            Err(CanvasError::Missing)
        );
        let duplicate = MarkdownDocument::parse(
            "```canvas Demo | Example\nOne\n```\n\n```canvas Demo | Example\nTwo\n```",
        );
        assert_eq!(
            resolve_canvas_source(&duplicate, "Demo | Example", "One"),
            Ok("One")
        );
        assert_eq!(
            resolve_canvas_source(&duplicate, "Demo | Example", "Old"),
            Err(CanvasError::Ambiguous)
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
        assert!(document.contains("connect-src 'none'"));
    }
}
