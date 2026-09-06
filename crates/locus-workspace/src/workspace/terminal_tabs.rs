use super::*;
use gpui::Focusable as _;
use gpui_component::{
    Disableable as _, Sizable as _,
    button::{Button, ButtonVariants as _},
    tab::{Tab, TabBar},
};

pub(super) const TERMINAL_TAB_BAR_HEIGHT: f32 = 36.0;

pub(super) struct TerminalTab {
    pub(super) id: u64,
    pub(super) terminal: Entity<TerminalView>,
    _title_subscription: Subscription,
}

fn terminal_tab_label(language: Language, id: u64) -> String {
    language.text_with("workspace.terminal.tab", &[("id", id.to_string())])
}

fn terminal_tab_title(language: Language, id: u64, terminal_title: Option<&str>) -> String {
    terminal_title
        .map(str::to_owned)
        .unwrap_or_else(|| terminal_tab_label(language, id))
}

fn close_terminal_label(language: Language, id: u64) -> String {
    language.text_with("workspace.terminal.close_tab", &[("id", id.to_string())])
}

fn adjacent_tab_after_close(remaining_tabs: usize, closed_index: usize) -> Option<usize> {
    (remaining_tabs > 0).then(|| closed_index.min(remaining_tabs - 1))
}

fn terminal_tab_scroll_item(tab_index: usize) -> usize {
    tab_index + 1
}

fn right_sidebar_tab_after_terminal_close(
    remaining_tabs: usize,
    current: RightSidebarTab,
) -> RightSidebarTab {
    if remaining_tabs == 0 {
        RightSidebarTab::KnowledgeGraph
    } else {
        current
    }
}

impl Locus {
    fn create_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let preferences = self.settings.read(cx).preferences().clone();
        let Some(path) = preferences.library_path.clone() else {
            return;
        };

        let id = self.next_terminal_id;
        self.next_terminal_id += 1;
        let configuration = terminal_configuration(&preferences);
        let terminal = cx.new(|cx| TerminalView::new(path, configuration, cx));
        let title_subscription = cx.subscribe(&terminal, |_, _, _: &TerminalEvent, cx| cx.notify());
        let focus = terminal.read(cx).focus_handle(cx);
        self.terminal_tabs.push(TerminalTab {
            id,
            terminal,
            _title_subscription: title_subscription,
        });
        self.selected_right_sidebar_tab = RightSidebarTab::Terminal;
        self.selected_terminal_id = Some(id);
        self.scroll_to_terminal_tab(id);
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

    fn select_terminal(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.terminal_tabs.get(index) else {
            return;
        };
        let focus = tab.terminal.read(cx).focus_handle(cx);
        self.selected_right_sidebar_tab = RightSidebarTab::Terminal;
        self.selected_terminal_id = Some(tab.id);
        self.scroll_to_terminal_tab(tab.id);
        window.focus(&focus, cx);
        cx.notify();
    }

    fn close_terminal(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.terminal_tabs.iter().position(|tab| tab.id == id) else {
            return;
        };
        let was_selected = self.selected_terminal_id == Some(id);
        self.terminal_tabs.remove(index);

        if was_selected {
            if let Some(selected_index) = adjacent_tab_after_close(self.terminal_tabs.len(), index)
            {
                let tab = &self.terminal_tabs[selected_index];
                let focus = tab.terminal.read(cx).focus_handle(cx);
                self.selected_terminal_id = Some(tab.id);
                self.scroll_to_terminal_tab(tab.id);
                window.focus(&focus, cx);
            } else {
                self.selected_terminal_id = None;
                window.focus(&self.root_focus, cx);
            }
        }
        self.selected_right_sidebar_tab = right_sidebar_tab_after_terminal_close(
            self.terminal_tabs.len(),
            self.selected_right_sidebar_tab,
        );
        cx.notify();
    }

    pub(super) fn close_terminal_tab_action(
        &mut self,
        _: &CloseTerminalTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(id) = self.selected_terminal_id {
            self.close_terminal(id, window, cx);
        }
    }

    pub(super) fn restart_terminals(&mut self, cx: &mut Context<Self>) {
        let preferences = self.settings.read(cx).preferences().clone();
        let Some(path) = preferences.library_path.clone() else {
            self.terminal_tabs.clear();
            self.selected_terminal_id = None;
            self.terminals_need_restart = false;
            cx.notify();
            return;
        };
        let configuration = terminal_configuration(&preferences);
        for tab in &mut self.terminal_tabs {
            let path = path.clone();
            let configuration = configuration.clone();
            let terminal = cx.new(|cx| TerminalView::new(path, configuration, cx));
            let title_subscription =
                cx.subscribe(&terminal, |_, _, _: &TerminalEvent, cx| cx.notify());
            tab.terminal = terminal;
            tab._title_subscription = title_subscription;
        }
        self.terminals_need_restart = false;
        cx.notify();
    }

    fn render_terminal_tab_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let language = self.language(cx);
        let selected_index = self
            .selected_terminal_id
            .and_then(|id| self.terminal_tabs.iter().position(|tab| tab.id == id));
        let tabs = self
            .terminal_tabs
            .iter()
            .map(|tab| {
                let id = tab.id;
                let close_label = close_terminal_label(language, id);
                let label = terminal_tab_title(language, id, tab.terminal.read(cx).title());
                Tab::new().px_2().label(label).suffix(
                    Button::new(format!("close-terminal-{id}"))
                        .ghost()
                        .xsmall()
                        .icon(Icon::Close)
                        .accessibility_label(close_label.clone())
                        .tooltip(close_label)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            cx.stop_propagation();
                            this.close_terminal(id, window, cx);
                        })),
                )
            })
            .collect::<Vec<_>>();
        let add_label = language.text("workspace.terminal.new");

        TabBar::new("terminal-tabs")
            .w_full()
            .h(px(TERMINAL_TAB_BAR_HEIGHT))
            .underline()
            .track_scroll(&self.terminal_tab_scroll_handle)
            .when_some(selected_index, |tabs, index| tabs.selected_index(index))
            .on_click(cx.listener(|this, index: &usize, window, cx| {
                this.select_terminal(*index, window, cx);
            }))
            .children(tabs)
            .suffix(
                Button::new("new-terminal")
                    .ghost()
                    .xsmall()
                    .mr_2()
                    .icon(Icon::Plus)
                    .disabled(self.current_library_path(cx).is_none())
                    .accessibility_label(add_label.clone())
                    .tooltip(add_label)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.create_terminal(window, cx);
                    })),
            )
    }

    fn scroll_to_terminal_tab(&self, id: u64) {
        if let Some(index) = self.terminal_tabs.iter().position(|tab| tab.id == id) {
            self.terminal_tab_scroll_handle
                .scroll_to_item(terminal_tab_scroll_item(index));
        }
    }

    fn render_terminal_placeholder(&self, cx: &mut Context<Self>) -> Div {
        let language = self.language(cx);
        let theme = Theme::global(cx).clone();
        let has_library = self.current_library_path(cx).is_some();
        let add_label = language.text("workspace.terminal.new");

        div()
            .min_h_0()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .px(px(24.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_3()
                    .child(
                        Button::new("empty-new-terminal")
                            .primary()
                            .icon(Icon::Plus)
                            .label(add_label)
                            .disabled(!has_library)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.create_terminal(window, cx);
                            })),
                    )
                    .child(div().text_sm().text_color(theme.muted_foreground).child(
                        if has_library {
                            language.text("workspace.terminal.shortcut_hint")
                        } else {
                            language.text("workspace.terminal.library_required")
                        },
                    )),
            )
    }

    pub(super) fn render_terminal_panel(&self, cx: &mut Context<Self>) -> Div {
        if self.terminal_tabs.is_empty() {
            return self.render_terminal_placeholder(cx);
        }

        let terminal = self
            .selected_terminal_id
            .and_then(|id| self.terminal_tabs.iter().find(|tab| tab.id == id))
            .map(|tab| tab.terminal.clone());

        div()
            .min_h_0()
            .flex_1()
            .flex()
            .flex_col()
            .child(self.render_terminal_tab_bar(cx))
            .child(
                div()
                    .min_h_0()
                    .flex_1()
                    .when_some(terminal, |body, terminal| body.child(terminal)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RightSidebarTab, adjacent_tab_after_close, right_sidebar_tab_after_terminal_close,
        terminal_tab_scroll_item, terminal_tab_title,
    };
    use locus_preferences::localization::Language;

    #[test]
    fn closing_a_tab_selects_the_right_neighbor_then_the_left_neighbor() {
        assert_eq!(adjacent_tab_after_close(2, 0), Some(0));
        assert_eq!(adjacent_tab_after_close(2, 1), Some(1));
        assert_eq!(adjacent_tab_after_close(2, 2), Some(1));
        assert_eq!(adjacent_tab_after_close(0, 0), None);
    }

    #[test]
    fn terminal_tab_scroll_item_skips_the_underline_indicator() {
        assert_eq!(terminal_tab_scroll_item(0), 1);
        assert_eq!(terminal_tab_scroll_item(11), 12);
    }

    #[test]
    fn last_terminal_reveals_the_knowledge_graph_tab() {
        assert_eq!(
            right_sidebar_tab_after_terminal_close(1, RightSidebarTab::Terminal),
            RightSidebarTab::Terminal
        );
        assert_eq!(
            right_sidebar_tab_after_terminal_close(0, RightSidebarTab::Terminal),
            RightSidebarTab::KnowledgeGraph
        );
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
