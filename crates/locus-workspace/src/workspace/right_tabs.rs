use super::*;
use crate::OpenCommandPalette;
use gpui_kit::Focusable as _;
use gpui_kit::component::WindowExt as _;
use gpui_kit::component::{
    Sizable as _,
    button::{Button, ButtonVariants as _},
    tab::{Tab, TabBar},
};
use locus_canvas::{CanvasEvent, CanvasView};
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
    Canvas {
        path: String,
        view: Entity<CanvasView>,
        _subscription: Subscription,
    },
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
            RightTabContent::Canvas { view, .. } => view.read(cx).focus_handle(cx),
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

fn canvas_navigation_changes<'a>(
    tabs: impl Iterator<Item = (u64, Option<&'a str>)>,
    selected: Option<&'a str>,
    canvas_available: bool,
    auto_open: bool,
) -> (Vec<u64>, Option<&'a str>) {
    let mut current_open = false;
    let mut close = Vec::new();
    for (id, owner) in tabs {
        if let Some(owner) = owner {
            if Some(owner) == selected {
                current_open = true;
            } else {
                close.push(id);
            }
        }
    }
    let open = selected.filter(|_| auto_open && canvas_available && !current_open);
    (close, open)
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
    pub(super) fn reload_focused_canvas(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.app_shell.workspace().visible(PanelSide::Right)
            || self.app_shell.is_settings_open()
        {
            return false;
        }
        for tab in &self.right_tabs {
            if Some(tab.id) == self.selected_right_tab_id
                && let RightTabContent::Canvas { view, .. } = &tab.content
                && view.read(cx).focus_handle(cx).is_focused(window)
            {
                view.update(cx, |view, cx| view.reload(cx));
                return true;
            }
        }
        false
    }

    pub(super) fn schedule_note_canvas_selection(&self, cx: &mut Context<Self>) {
        let view = cx.weak_entity();
        let window = self.window_handle;
        let selected = self.selected_library_node.clone();
        let library = self.current_library_path(cx).map(Path::to_path_buf);
        cx.defer(move |cx| {
            let _ = window.update(cx, |_, window, cx| {
                let _ = view.update(cx, |this, cx| {
                    if this.selected_library_node == selected
                        && this.current_library_path(cx) == library.as_deref()
                    {
                        this.apply_note_canvas_selection(window, cx);
                    }
                });
            });
        });
    }

    pub(super) fn apply_note_canvas_selection(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let could_reload = self.can_reload_current_note();
        let language = self.language(cx);
        let selected = self.selected_library_node.clone();
        let (close, open) = canvas_navigation_changes(
            self.right_tabs.iter().map(|tab| {
                (
                    tab.id,
                    match &tab.content {
                        RightTabContent::Canvas { path, .. } => Some(path.as_str()),
                        RightTabContent::Terminal { .. } => None,
                    },
                )
            }),
            selected.as_deref(),
            selected
                .as_deref()
                .and_then(|path| self.library_tree.node(path))
                .is_some_and(|node| node.canvas_path.is_some()),
            self.settings.read(cx).preferences().auto_open_canvas,
        );
        let open = open.map(str::to_owned);
        self.right_tabs.retain(|tab| {
            let RightTabContent::Canvas { view, .. } = &tab.content else {
                return true;
            };
            if !close.contains(&tab.id) {
                return true;
            }
            if view.read(cx).focus_handle(cx).is_focused(window) {
                window.focus(&self.root_focus, cx);
            }
            view.update(cx, |view, cx| view.synchronize(false, language, cx));
            false
        });
        if !self
            .right_tabs
            .iter()
            .any(|tab| Some(tab.id) == self.selected_right_tab_id)
        {
            self.selected_right_tab_id = self.right_tabs.last().map(|tab| tab.id);
        }
        if let Some(path) = open {
            self.open_canvas(&path, false, window, cx);
        }
        if !close.is_empty()
            && self.right_tabs.is_empty()
            && self.app_shell.workspace().visible(PanelSide::Right)
        {
            self.toggle_panel(PanelSide::Right, window, cx);
        }
        self.update_app_menus_if_reload_state_changed(could_reload, cx);
        cx.notify();
    }

    pub(super) fn open_canvas(
        &mut self,
        path: &str,
        focus: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(node) = self
            .library_tree
            .node(path)
            .filter(|node| node.canvas_path.is_some())
            .cloned()
        else {
            return;
        };
        let Some(root) = self.current_library_path(cx).map(Path::to_path_buf) else {
            return;
        };
        if !self.app_shell.workspace().visible(PanelSide::Right) {
            self.toggle_panel(PanelSide::Right, window, cx);
        }
        if let Some(index) = self.right_tabs.iter().position(|tab| {
            matches!(&tab.content,
            RightTabContent::Canvas { path: owner, .. } if owner == path)
        }) {
            if matches!(&self.right_tabs[index].content, RightTabContent::Canvas { view, .. } if view.read(cx).has_error())
            {
                self.right_tabs.remove(index);
            } else {
                if focus {
                    self.select_right_tab(index, window, cx);
                }
                return;
            }
        }
        let id = self.next_right_tab_id;
        self.next_right_tab_id += 1;
        let language = self.language(cx);
        let view = cx.new(|cx| CanvasView::new(&root, &node.directory, language, window, cx));
        let subscription = cx.subscribe_in(&view, window, move |this, _, event, window, cx| {
            if this.selected_right_tab_id != Some(id)
                || this.app_shell.is_settings_open()
                || !this.app_shell.workspace().visible(PanelSide::Right)
            {
                return;
            }
            match event {
                CanvasEvent::Close => this.close_right_tab(id, window, cx),
                CanvasEvent::CommandPalette => {
                    this.open_command_palette_action(&OpenCommandPalette, window, cx)
                }
                CanvasEvent::Settings => window.dispatch_action(Box::new(OpenSettings), cx),
                CanvasEvent::OpenUrl(url) => cx.open_url(url),
            }
        });
        self.right_tabs.push(RightTab {
            id,
            content: RightTabContent::Canvas {
                path: node.path,
                view,
                _subscription: subscription,
            },
        });
        if focus {
            self.select_right_tab(self.right_tabs.len() - 1, window, cx);
        } else {
            self.selected_right_tab_id = Some(id);
            self.scroll_to_right_tab(id);
            cx.notify();
        }
    }

    pub(super) fn synchronize_canvases(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let visible = !self.app_shell.is_settings_open()
            && self.app_shell.workspace().visible(PanelSide::Right)
            && !self.app_shell.workspace().transitioning(PanelSide::Right)
            && self.command_palette_state == super::command_palette::CommandPaletteState::Hidden
            && !window.has_active_dialog(cx)
            && !window.has_active_sheet(cx);
        let language = self.language(cx);
        for tab in &mut self.right_tabs {
            if let RightTabContent::Canvas { view, .. } = &mut tab.content {
                view.update(cx, |view, cx| {
                    view.synchronize(
                        visible && self.selected_right_tab_id == Some(tab.id),
                        language,
                        cx,
                    )
                });
            }
        }
    }

    fn create_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let could_reload_current_note = self.can_reload_current_note();
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
        self.update_app_menus_if_reload_state_changed(could_reload_current_note, cx);
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
        let could_reload_current_note = self.can_reload_current_note();
        let Some(tab) = self.right_tabs.get_mut(index) else {
            return;
        };
        if let RightTabContent::Canvas { view, .. } = &mut tab.content {
            view.update(cx, |view, _| view.request_focus());
        }
        let focus = tab.focus_handle(cx);
        let id = tab.id;
        self.selected_right_tab_id = Some(id);
        self.scroll_to_right_tab(id);
        window.focus(&focus, cx);
        self.update_app_menus_if_reload_state_changed(could_reload_current_note, cx);
        cx.notify();
    }

    fn close_right_tab(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let could_reload_current_note = self.can_reload_current_note();
        let Some(index) = self.right_tabs.iter().position(|tab| tab.id == id) else {
            return;
        };
        if let RightTabContent::Canvas { view, .. } = &self.right_tabs[index].content {
            let language = self.language(cx);
            view.update(cx, |view, cx| view.synchronize(false, language, cx));
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
            if let RightTabContent::Canvas { view, .. } = &mut tab.content {
                view.update(cx, |view, _| view.request_focus());
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
        self.update_app_menus_if_reload_state_changed(could_reload_current_note, cx);
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
                    RightTabContent::Canvas { .. } => (
                        t!("workspace.canvas.title", locale = language.locale()).into_owned(),
                        t!("workspace.canvas.close", locale = language.locale()).into_owned(),
                        Icon::Globe,
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
                RightTabContent::Canvas { view, .. } => view.clone().into_any_element(),
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
    use super::{
        canvas_navigation_changes, right_tab_scroll_item, selected_tab_after_close,
        terminal_tab_title,
    };
    use oneo_gpui::Language;

    #[test]
    fn navigation_closes_only_departing_canvases_and_opens_when_enabled() {
        let tabs = [(1, None), (2, Some("first")), (3, None)];
        assert_eq!(
            canvas_navigation_changes(tabs.into_iter(), Some("second"), true, true),
            (vec![2], Some("second"))
        );
        assert_eq!(
            canvas_navigation_changes(tabs.into_iter(), Some("second"), true, false),
            (vec![2], None)
        );
        assert_eq!(
            canvas_navigation_changes(tabs.into_iter(), Some("second"), false, true),
            (vec![2], None)
        );
        assert_eq!(
            canvas_navigation_changes(tabs.into_iter(), None, false, true),
            (vec![2], None)
        );
        assert_eq!(
            canvas_navigation_changes(tabs.into_iter(), Some("first"), true, true),
            (vec![], None)
        );
    }

    #[test]
    fn startup_and_reentry_open_a_fresh_canvas_only_when_enabled() {
        for enabled in [false, true] {
            assert_eq!(
                canvas_navigation_changes(std::iter::empty(), Some("first"), true, enabled),
                (vec![], enabled.then_some("first"))
            );
        }
    }

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
