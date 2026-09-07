use super::canvas::CanvasError;
use super::*;
use crate::OpenCommandPalette;
use gpui_kit::Focusable as _;
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::{
    Sizable as _,
    button::{Button, ButtonVariants as _},
    tab::{Tab, TabBar},
};
use rust_i18n::t;

const RIGHT_TAB_MAX_WIDTH_REM: f32 = 16.0;

pub(super) struct RightTab {
    pub(super) id: u64,
    pub(super) content: RightTabContent,
}

pub(super) enum RightTabContent {
    Terminal {
        terminal: Entity<TerminalView>,
        _title_subscription: Subscription,
    },
    Canvas(super::canvas::CanvasTab),
}

impl RightTab {
    pub(super) fn terminal(&self) -> Option<&Entity<TerminalView>> {
        match &self.content {
            RightTabContent::Terminal { terminal, .. } => Some(terminal),
            _ => None,
        }
    }

    fn focus_handle(&self, cx: &App) -> FocusHandle {
        match &self.content {
            RightTabContent::Terminal { terminal, .. } => terminal.read(cx).focus_handle(cx),
            RightTabContent::Canvas(canvas) => canvas.focus_handle(),
        }
    }
}

fn terminal_tab_label(language: Language, id: u64) -> String {
    t!(
        "workspace.terminal.tab",
        id = id.to_string(),
        locale = language.locale()
    )
    .into_owned()
}

fn terminal_tab_title(language: Language, id: u64, terminal_title: Option<&str>) -> String {
    terminal_title
        .map(str::to_owned)
        .unwrap_or_else(|| terminal_tab_label(language, id))
}

fn close_terminal_label(language: Language, id: u64) -> String {
    t!(
        "workspace.terminal.close_tab",
        id = id.to_string(),
        locale = language.locale()
    )
    .into_owned()
}

fn selected_tab_after_close(
    selected: Option<u64>,
    closed: u64,
    neighbor: Option<u64>,
) -> Option<u64> {
    if selected == Some(closed) {
        neighbor
    } else {
        selected
    }
}

fn right_tab_scroll_item(tab_index: usize) -> usize {
    tab_index + 1
}

fn right_tab(label: String, icon: Icon, close: Button) -> Tab {
    Tab::new()
        .min_h(px(TITLEBAR_HEIGHT))
        .px_2()
        .aria_label(label.clone())
        .label(label)
        .prefix(gpui_kit::component::Icon::new(icon))
        .suffix(close)
}

impl Locus {
    pub(super) fn reload_canvases(&mut self, cx: &mut Context<Self>) {
        let mut requests = Vec::new();
        for tab in &mut self.right_tabs {
            if let RightTabContent::Canvas(canvas) = &mut tab.content {
                canvas.reload_generation = canvas.reload_generation.wrapping_add(1);
                let file = self
                    .library_tree
                    .node(&canvas.path)
                    .and_then(|node| node.document_path.clone());
                requests.push((tab.id, canvas.reload_generation, file));
            }
        }
        if requests.is_empty() {
            return;
        }
        cx.spawn(async move |this, cx| {
            let results = cx
                .background_spawn(async move {
                    requests
                        .into_iter()
                        .map(|(id, generation, file)| {
                            let document = file
                                .ok_or(CanvasError::Missing)
                                .and_then(|file| {
                                    load_note(&file)
                                        .map_err(|error| CanvasError::Load(error.to_string()))
                                })
                                .map(|source| MarkdownDocument::parse(&source));
                            (id, generation, document)
                        })
                        .collect::<Vec<_>>()
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                for (id, generation, document) in results {
                    if let Some(tab) = this.right_tabs.iter_mut().find(|tab| tab.id == id)
                        && let RightTabContent::Canvas(canvas) = &mut tab.content
                        && canvas.reload_generation == generation
                    {
                        canvas.refresh(document, cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn open_canvas(
        &mut self,
        path: &str,
        title: &str,
        source: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.app_shell.workspace().visible(PanelSide::Right) {
            self.toggle_panel(PanelSide::Right, window, cx);
        }
        if let Some(index) = self.right_tabs.iter().position(|tab| {
            matches!(&tab.content,
            RightTabContent::Canvas(canvas) if canvas.matches(path, title, source))
        }) {
            if matches!(&self.right_tabs[index].content, RightTabContent::Canvas(canvas) if canvas.has_error())
            {
                self.right_tabs.remove(index);
            } else {
                self.select_right_tab(index, window, cx);
                return;
            }
        }
        let id = self.next_right_tab_id;
        self.next_right_tab_id += 1;
        let (sender, receiver) = async_channel::bounded(16);
        let canvas = super::canvas::CanvasTab::new(path, title, source, sender, window, cx);
        cx.spawn_in(window, async move |this, cx| {
            while let Ok(command) = receiver.recv().await {
                if this
                    .update_in(cx, |this, window, cx| {
                        if command == "loaded" {
                            if let Some(tab) = this.right_tabs.iter_mut().find(|tab| tab.id == id)
                                && let RightTabContent::Canvas(canvas) = &mut tab.content
                            {
                                canvas.finish_loading();
                                cx.notify();
                            }
                            return;
                        }
                        if this.selected_right_tab_id != Some(id)
                            || this.app_shell.is_settings_open()
                            || !this.app_shell.workspace().visible(PanelSide::Right)
                        {
                            return;
                        }
                        if let Some(url) = command.strip_prefix("url:") {
                            cx.open_url(url);
                            return;
                        }
                        match command.as_str() {
                            "w" => this.close_right_tab(id, window, cx),
                            "e" => {
                                this.open_command_palette_action(&OpenCommandPalette, window, cx)
                            }
                            "," => window.dispatch_action(Box::new(OpenSettings), cx),
                            "focus" => {
                                if let Some(tab) = this.right_tabs.iter().find(|tab| tab.id == id) {
                                    window.focus(&tab.focus_handle(cx), cx);
                                }
                            }
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
        self.right_tabs.push(RightTab {
            id,
            content: RightTabContent::Canvas(canvas),
        });
        self.select_right_tab(self.right_tabs.len() - 1, window, cx);
    }

    pub(super) fn synchronize_canvases(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let visible = !self.app_shell.is_settings_open()
            && self.app_shell.workspace().visible(PanelSide::Right)
            && !self.app_shell.workspace().transitioning(PanelSide::Right)
            && self.command_palette_state == super::command_palette::CommandPaletteState::Hidden
            && !window.has_active_dialog(cx)
            && !window.has_active_sheet(cx);
        let dark = Theme::global(cx).is_dark();
        for tab in &mut self.right_tabs {
            if let RightTabContent::Canvas(canvas) = &mut tab.content {
                canvas.synchronize(
                    visible && self.selected_right_tab_id == Some(tab.id),
                    dark,
                    cx,
                );
            }
        }
    }

    fn create_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let preferences = self.settings.read(cx).preferences().clone();
        let Some(path) = preferences.library_path.clone() else {
            return;
        };

        let id = self.next_right_tab_id;
        self.next_right_tab_id += 1;
        let configuration = terminal_configuration(&preferences);
        let terminal = cx.new(|cx| TerminalView::new(path, configuration, cx));
        let title_subscription = cx.subscribe(&terminal, |this, _, _: &TerminalEvent, cx| {
            if let Some(id) = this.selected_right_tab_id {
                this.scroll_to_right_tab(id);
            }
            cx.notify();
        });
        let focus = terminal.read(cx).focus_handle(cx);
        self.right_tabs.push(RightTab {
            id,
            content: RightTabContent::Terminal {
                terminal,
                _title_subscription: title_subscription,
            },
        });
        self.selected_right_tab_id = Some(id);
        self.scroll_to_right_tab(id);
        window.focus(&focus, cx);
        cx.notify();
    }

    pub(super) fn new_terminal_action(
        &mut self,
        _: &NewTerminal,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.app_shell.is_settings_open() {
            self.app_shell.close_settings();
            if self.terminals_need_restart {
                self.restart_terminals(cx);
            }
        }
        if !self.app_shell.workspace().visible(PanelSide::Right) {
            self.toggle_panel(PanelSide::Right, window, cx);
        }
        self.create_terminal(window, cx);
    }

    fn select_right_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.right_tabs.get_mut(index) else {
            return;
        };
        if let RightTabContent::Canvas(canvas) = &mut tab.content {
            canvas.request_focus();
        }
        let focus = tab.focus_handle(cx);
        let id = tab.id;
        self.selected_right_tab_id = Some(id);
        self.scroll_to_right_tab(id);
        window.focus(&focus, cx);
        cx.notify();
    }

    fn close_right_tab(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.right_tabs.iter().position(|tab| tab.id == id) else {
            return;
        };
        if let RightTabContent::Canvas(canvas) = &mut self.right_tabs[index].content {
            canvas.synchronize(false, Theme::global(cx).is_dark(), cx);
        }
        self.right_tabs.remove(index);
        let neighbor = self
            .right_tabs
            .get(index)
            .or_else(|| self.right_tabs.last())
            .map(|tab| tab.id);
        self.selected_right_tab_id =
            selected_tab_after_close(self.selected_right_tab_id, id, neighbor);

        if let Some(selected) = self.selected_right_tab_id {
            let tab = self
                .right_tabs
                .iter_mut()
                .find(|tab| tab.id == selected)
                .expect("selected right tab exists");
            if let RightTabContent::Canvas(canvas) = &mut tab.content {
                canvas.request_focus();
            }
            let focus = tab.focus_handle(cx);
            self.scroll_to_right_tab(selected);
            window.focus(&focus, cx);
        } else {
            window.focus(&self.root_focus, cx);
            if self.app_shell.workspace().visible(PanelSide::Right) {
                self.toggle_panel(PanelSide::Right, window, cx);
            }
        }
        cx.notify();
    }

    pub(super) fn close_right_tab_action(
        &mut self,
        _: &CloseWindow,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(id) = self.selected_right_tab_id {
            self.close_right_tab(id, window, cx);
        }
    }

    pub(super) fn restart_terminals(&mut self, cx: &mut Context<Self>) {
        let preferences = self.settings.read(cx).preferences().clone();
        let Some(path) = preferences.library_path.clone() else {
            self.right_tabs.clear();
            self.selected_right_tab_id = None;
            self.terminals_need_restart = false;
            cx.notify();
            return;
        };
        let configuration = terminal_configuration(&preferences);
        for tab in &mut self.right_tabs {
            if tab.terminal().is_none() {
                continue;
            }
            let path = path.clone();
            let configuration = configuration.clone();
            let terminal = cx.new(|cx| TerminalView::new(path, configuration, cx));
            let title_subscription = cx.subscribe(&terminal, |this, _, _: &TerminalEvent, cx| {
                if let Some(id) = this.selected_right_tab_id {
                    this.scroll_to_right_tab(id);
                }
                cx.notify();
            });
            tab.content = RightTabContent::Terminal {
                terminal,
                _title_subscription: title_subscription,
            };
        }
        self.terminals_need_restart = false;
        cx.notify();
    }

    pub(super) fn render_right_tab_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let language = self.language(cx);
        let selected_index = self
            .selected_right_tab_id
            .and_then(|id| self.right_tabs.iter().position(|tab| tab.id == id));
        let tabs = self
            .right_tabs
            .iter()
            .map(|tab| {
                let id = tab.id;
                let (label, close_label, icon) = match &tab.content {
                    RightTabContent::Terminal { terminal, .. } => (
                        terminal_tab_title(language, id, terminal.read(cx).title()),
                        close_terminal_label(language, id),
                        Icon::SquareTerminal,
                    ),
                    RightTabContent::Canvas(canvas) => (
                        canvas.label(),
                        t!(
                            "workspace.canvas.close",
                            title = canvas.label(),
                            locale = language.locale()
                        )
                        .into_owned(),
                        Icon::File,
                    ),
                };
                right_tab(
                    label,
                    icon,
                    Button::new(format!("close-right-tab-{id}"))
                        .ghost()
                        .xsmall()
                        .icon(Icon::Close)
                        .accessibility_label(close_label.clone())
                        .tooltip(close_label)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            cx.stop_propagation();
                            this.close_right_tab(id, window, cx);
                        })),
                )
            })
            .collect::<Vec<_>>();

        TabBar::new("right-sidebar-tabs")
            .w_full()
            .min_w_0()
            .h_full()
            .underline()
            .max_width(Theme::global(cx).font_size * RIGHT_TAB_MAX_WIDTH_REM)
            .track_scroll(&self.right_tab_scroll_handle)
            .when_some(selected_index, |tabs, index| tabs.selected_index(index))
            .on_click(cx.listener(|this, index: &usize, window, cx| {
                this.select_right_tab(*index, window, cx);
            }))
            .children(tabs)
    }

    fn scroll_to_right_tab(&self, id: u64) {
        if let Some(index) = self.right_tabs.iter().position(|tab| tab.id == id) {
            self.right_tab_scroll_handle
                .scroll_to_item(right_tab_scroll_item(index));
        }
    }

    fn render_right_sidebar_placeholder(&self, cx: &mut Context<Self>) -> Div {
        div()
            .min_h_0()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .p_6()
            .text_sm()
            .text_color(Theme::global(cx).muted_foreground)
            .child(
                t!(
                    "workspace.sidebar.empty",
                    locale = self.language(cx).locale()
                )
                .into_owned(),
            )
    }

    pub(super) fn render_right_panel(&self, cx: &mut Context<Self>) -> Div {
        if self.right_tabs.is_empty() {
            return self.render_right_sidebar_placeholder(cx);
        }

        let content = self
            .selected_right_tab_id
            .and_then(|id| self.right_tabs.iter().find(|tab| tab.id == id))
            .map(|tab| match &tab.content {
                RightTabContent::Terminal { terminal, .. } => terminal.clone().into_any_element(),
                RightTabContent::Canvas(canvas) => canvas.render(self.language(cx)),
            });

        right_panel_body(content)
    }
}

fn right_panel_body(content: Option<AnyElement>) -> Div {
    div().min_h_0().flex_1().flex().flex_col().child(
        div()
            .min_h_0()
            .flex_1()
            .when_some(content, |body, content| body.child(content)),
    )
}

#[cfg(test)]
mod tests {
    use super::{right_tab_scroll_item, selected_tab_after_close, terminal_tab_title};
    use oneo_gpui::Language;

    #[test]
    fn closing_the_selected_tab_selects_its_neighbor() {
        assert_eq!(selected_tab_after_close(Some(2), 2, Some(3)), Some(3));
        assert_eq!(selected_tab_after_close(Some(3), 3, Some(1)), Some(1));
    }

    #[test]
    fn closing_a_background_tab_preserves_the_selected_session() {
        assert_eq!(selected_tab_after_close(Some(3), 1, Some(2)), Some(3));
    }

    #[test]
    fn closing_the_last_tab_clears_the_selection() {
        assert_eq!(selected_tab_after_close(Some(7), 7, None), None);
    }

    #[test]
    fn right_tab_scroll_item_skips_the_selection_indicator() {
        assert_eq!(right_tab_scroll_item(0), 1);
        assert_eq!(right_tab_scroll_item(11), 12);
    }

    #[test]
    fn terminal_tab_title_prefers_the_session_title() {
        assert_eq!(
            terminal_tab_title(Language::English, 3, Some("π - locus")),
            "π - locus"
        );
        assert_eq!(terminal_tab_title(Language::English, 3, None), "Terminal 3");
    }
}

#[cfg(test)]
mod layout_tests {
    use super::*;
    use gpui_kit::{TestAppContext, VisualTestContext};

    struct ContentPreview {
        height: f32,
    }

    impl Render for ContentPreview {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(520.)).h(px(self.height)).child(
                oneo_gpui::WorkspacePanel::new(right_panel_body(Some(
                    div()
                        .size_full()
                        .debug_selector(|| "webview-slot".to_owned())
                        .into_any_element(),
                )))
                .header(div().size_full())
                .into_element()
                .debug_selector(|| "preview-panel".to_owned()),
            )
        }
    }

    #[gpui_kit::test]
    fn webview_slot_fills_the_panel_below_the_tabs(cx: &mut TestAppContext) {
        cx.update(gpui_kit::component::init);
        let (view, cx) = cx.add_window_view(|_, _| ContentPreview { height: 900. });
        for height in [900., 640., 1100.] {
            view.update(cx, |view, cx| {
                view.height = height;
                cx.notify();
            });
            draw(cx);
            let panel = cx.debug_bounds("preview-panel").unwrap();
            let content = cx.debug_bounds("webview-slot").unwrap();
            assert_eq!(content.bottom(), panel.bottom());
            assert_eq!(content.size.width, panel.size.width);
            assert_eq!(content.size.height, panel.size.height - px(TITLEBAR_HEIGHT));
        }
    }

    struct TabPreview {
        title: String,
    }

    impl Render for TabPreview {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            window.set_rem_size(Theme::global(cx).font_size);
            div()
                .id("preview-header")
                .debug_selector(|| "tab-header".to_owned())
                .w_full()
                .h(px(TITLEBAR_HEIGHT))
                .child(
                    TabBar::new("preview-tabs")
                        .w_full()
                        .h_full()
                        .underline()
                        .selected_index(0)
                        .max_width(Theme::global(cx).font_size * RIGHT_TAB_MAX_WIDTH_REM)
                        .child(
                            right_tab(
                                self.title.clone(),
                                Icon::SquareTerminal,
                                Button::new("preview-close")
                                    .ghost()
                                    .xsmall()
                                    .icon(Icon::Close)
                                    .debug_selector(|| "tab-close".to_owned()),
                            )
                            .debug_selector(|| "terminal-tab".to_owned()),
                        ),
                )
        }
    }

    fn draw(cx: &mut VisualTestContext) {
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }

    #[gpui_kit::test]
    fn terminal_titles_resize_without_clipping_the_close_button(cx: &mut TestAppContext) {
        cx.update(gpui_kit::component::init);
        let (preview, cx) = cx.add_window_view(|_, _| TabPreview {
            title: "zsh".to_owned(),
        });
        draw(cx);
        let original = cx.debug_bounds("terminal-tab").expect("tab is rendered");
        let close = cx
            .debug_bounds("tab-close")
            .expect("close button is rendered");
        let header = cx.debug_bounds("tab-header").expect("header is rendered");
        assert_eq!(original.origin, header.origin);
        assert_eq!(original.bottom(), header.bottom());
        let inset = original.right() - close.right();
        assert!(inset > px(0.));
        for title in [
            "π - Notes",
            "Running a command with a much longer terminal title",
            "zsh",
        ] {
            preview.update(cx, |preview, cx| {
                preview.title = title.to_owned();
                cx.notify();
            });
            draw(cx);
            let tab = cx.debug_bounds("terminal-tab").unwrap();
            let close = cx.debug_bounds("tab-close").unwrap();
            assert_eq!(tab.size.height, original.size.height);
            assert_eq!(tab.right() - close.right(), inset);
            if title == "zsh" {
                assert_eq!(tab.size.width, original.size.width);
            } else {
                assert!(tab.size.width > original.size.width);
            }
            let max_width = cx.read(|cx| Theme::global(cx).font_size * RIGHT_TAB_MAX_WIDTH_REM);
            assert!(tab.size.width <= max_width);
        }
    }
    #[gpui_kit::test]
    fn selected_underline_is_painted_on_the_header_bottom(cx: &mut TestAppContext) {
        cx.update(gpui_kit::component::init);
        let (preview, cx) = cx.add_window_view(|_, _| TabPreview {
            title: "π - Notes".to_owned(),
        });
        draw(cx);
        preview.update(cx, |_, cx| cx.notify());
        draw(cx);
        let header = cx.debug_bounds("tab-header").expect("header is rendered");
        cx.update(|window, cx| {
            let background: gpui_kit::Background = Theme::global(cx).primary.into();
            let underline = window
                .painted_quads()
                .into_iter()
                .find(|quad| {
                    quad.background == background
                        && quad.bounds.size.height.as_f32() == 2.0 * window.scale_factor()
                })
                .expect("selected underline is painted");
            assert_eq!(
                underline.bounds.bottom().as_f32(),
                f32::from(header.bottom()) * window.scale_factor()
            );
        });
    }
}
