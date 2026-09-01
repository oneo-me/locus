use super::*;
use crate::mcp::McpServerState;
use gpui_component::{
    Disableable as _, Icon as ComponentIcon, Sizable as _,
    button::{Button, ButtonVariants as _},
    popover::Popover,
    switch::Switch,
    tab::{Tab, TabBar},
};

impl Locus {
    pub(super) fn toggle_panel(
        &mut self,
        side: PanelSide,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.app_shell
            .workspace_mut()
            .toggle(side, window, cx.reduce_motion(), WINDOW_MIN_WIDTH);
        self.save_interface_state(cx);
        cx.notify();
    }

    pub(super) fn window_drag_region(
        &self,
        id: &'static str,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        div()
            .id(id)
            .h_full()
            .on_click(|event, window, _| {
                if event.click_count() == 2 {
                    window.titlebar_double_click();
                }
            })
            .on_mouse_down_out(cx.listener(|this, _, _, _| this.header_drag_armed = false))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.header_drag_armed = true),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| this.header_drag_armed = false),
            )
            .on_mouse_move(cx.listener(|this, _, window, _| {
                if this.header_drag_armed {
                    this.header_drag_armed = false;
                    window.start_window_move();
                }
            }))
    }

    pub(super) fn render_panel_toggle(
        &self,
        side: PanelSide,
        cx: &mut Context<Self>,
    ) -> gpui_component::button::Button {
        let theme = Theme::global(cx).clone();
        let language = self.language(cx);
        let (id, label) = match side {
            PanelSide::Left => (
                "toggle-left-sidebar",
                if self.app_shell.workspace().visible(PanelSide::Left) {
                    language.text("workspace.sidebar.hide_left")
                } else {
                    language.text("workspace.sidebar.show_left")
                },
            ),
            PanelSide::Right => (
                "toggle-right-sidebar",
                if self.app_shell.workspace().visible(PanelSide::Right) {
                    language.text("workspace.sidebar.hide_right")
                } else {
                    language.text("workspace.sidebar.show_right")
                },
            ),
        };

        panel_toggle_button(id, label, side, &theme)
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(move |this, _, window, cx| this.toggle_panel(side, window, cx)))
    }

    pub(super) fn render_history_button(
        &self,
        direction: NavigationDirection,
        cx: &mut Context<Self>,
    ) -> Button {
        let theme = Theme::global(cx).clone();
        let language = self.language(cx);
        let (id, label, icon_kind, enabled) = match direction {
            NavigationDirection::Back => (
                "navigate-back",
                language.text("workspace.navigation.back"),
                Icon::ArrowLeft,
                self.library_navigation.can_go_back(),
            ),
            NavigationDirection::Forward => (
                "navigate-forward",
                language.text("workspace.navigation.forward"),
                Icon::ArrowRight,
                self.library_navigation.can_go_forward(),
            ),
        };

        Button::new(id)
            .ghost()
            .small()
            .size(px(TITLEBAR_CONTROL_SIZE))
            .icon(icon_kind)
            .text_color(theme.muted_foreground)
            .when(!enabled, |button| button.opacity(0.35))
            .disabled(!enabled)
            .accessibility_label(label.clone())
            .tooltip(label)
            .on_click(cx.listener(move |this, _, _, cx| match direction {
                NavigationDirection::Back => this.navigate_back(cx),
                NavigationDirection::Forward => this.navigate_forward(cx),
            }))
    }

    pub(super) fn render_left_titlebar(
        &self,
        left_control_clearance: f32,
        cx: &mut Context<Self>,
    ) -> Div {
        div()
            .h(px(TITLEBAR_HEIGHT))
            .flex_none()
            .flex()
            .items_center()
            .child(
                self.window_drag_region("left-control-clearance-region", cx)
                    .w(px(left_control_clearance))
                    .flex_none(),
            )
            .child(
                self.window_drag_region("left-title-region", cx)
                    .min_w_0()
                    .flex_1(),
            )
    }

    pub(super) fn render_content_titlebar(
        &self,
        left_control_clearance: f32,
        cx: &mut Context<Self>,
    ) -> Div {
        div()
            .h(px(TITLEBAR_HEIGHT))
            .flex_none()
            .flex()
            .items_center()
            .gap(px(8.0))
            .pl(px(titlebar_content_left_padding(
                left_control_clearance,
                self.app_shell.workspace().rendered_width(PanelSide::Left),
                14.0,
            )))
            .pr(px(14.0))
            .child(
                self.window_drag_region("content-title-region", cx)
                    .min_w_0()
                    .flex_shrink(1.0)
                    .flex()
                    .items_center()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(
                        self.selected_library_node
                            .as_deref()
                            .and_then(|path| self.library_tree.node(path))
                            .map(|node| node.title.clone())
                            .unwrap_or_else(|| {
                                self.language(cx).text("workspace.note.no_selection")
                            }),
                    ),
            )
            .child(
                self.window_drag_region("content-center-drag-region", cx)
                    .flex_1(),
            )
            .child(self.fps_counter.clone())
            .child(div().size(px(TITLEBAR_CONTROL_SIZE)).flex_none())
    }

    pub(super) fn render_right_titlebar(&self, cx: &mut Context<Self>) -> Div {
        let theme = Theme::global(cx).clone();
        let terminal_label = self.language(cx).text("workspace.terminal.title");
        let graph_label = self.language(cx).text("workspace.graph.title");
        let selected_index = self.selected_right_sidebar_tab.index();
        div()
            .h(px(TITLEBAR_HEIGHT))
            .flex_none()
            .border_b_1()
            .border_color(theme.border)
            .flex()
            .items_center()
            .pl(px(10.0))
            .pr(px(14.0))
            .child(
                self.window_drag_region("right-title-region", cx)
                    .min_w_0()
                    .flex_1()
                    .flex()
                    .items_center()
                    .child(
                        TabBar::new("right-sidebar-tabs")
                            .pill()
                            .selected_index(selected_index)
                            .max_width(px(260.0))
                            .on_click(cx.listener(|this, index: &usize, window, cx| {
                                this.select_right_sidebar_tab(*index, window, cx);
                            }))
                            .child(
                                Tab::new().aria_label(graph_label.clone()).child(
                                    div()
                                        .min_w_0()
                                        .flex()
                                        .items_center()
                                        .gap(px(6.0))
                                        .child(ComponentIcon::new(Icon::Network))
                                        .child(
                                            div().min_w_0().flex_1().truncate().child(graph_label),
                                        ),
                                ),
                            )
                            .child(
                                Tab::new().aria_label(terminal_label.clone()).child(
                                    div()
                                        .min_w_0()
                                        .flex()
                                        .items_center()
                                        .gap(px(6.0))
                                        .child(ComponentIcon::new(Icon::SquareTerminal))
                                        .child(
                                            div()
                                                .min_w_0()
                                                .flex_1()
                                                .truncate()
                                                .child(terminal_label),
                                        ),
                                ),
                            ),
                    ),
            )
            .when(
                self.selected_right_sidebar_tab == RightSidebarTab::KnowledgeGraph,
                |header| {
                    let settings_label = self.language(cx).text("workspace.graph.settings");
                    let attachments_label =
                        self.language(cx).text("workspace.graph.show_attachments");
                    let attachments_description = self
                        .language(cx)
                        .text("workspace.graph.show_attachments_description");
                    let checked = self.settings.read(cx).preferences().show_graph_attachments;
                    header.child(
                        Popover::new("knowledge-graph-settings")
                            .w(px(260.0))
                            .trigger(
                                Button::new("open-knowledge-graph-settings")
                                    .ghost()
                                    .xsmall()
                                    .icon(Icon::Ellipsis)
                                    .accessibility_label(settings_label.clone())
                                    .tooltip(settings_label),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_start()
                                    .justify_between()
                                    .gap_3()
                                    .child(
                                        div()
                                            .min_w_0()
                                            .flex_1()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .child(div().text_sm().child(attachments_label.clone()))
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(theme.muted_foreground)
                                                    .child(attachments_description),
                                            ),
                                    )
                                    .child(
                                        Switch::new("show-knowledge-graph-attachments")
                                            .small()
                                            .checked(checked)
                                            .accessibility_label(attachments_label)
                                            .on_click(cx.listener(
                                                |this, checked: &bool, _, cx| {
                                                    this.settings.update(cx, |settings, cx| {
                                                        settings.set_show_graph_attachments(
                                                            *checked, cx,
                                                        );
                                                    });
                                                    this.knowledge_graph.update(cx, |graph, cx| {
                                                        graph.set_show_attachments(*checked, cx);
                                                    });
                                                    cx.notify();
                                                },
                                            )),
                                    ),
                            ),
                    )
                },
            )
    }

    pub(super) fn render_settings_button(
        &self,
        cx: &mut Context<Self>,
    ) -> gpui_component::button::Button {
        let label = self.language(cx).text("workspace.settings.open");
        toolbar_icon_button("open-settings", label, Icon::Settings, Theme::global(cx))
            .size(px(28.0))
            .on_click(cx.listener(|this, _, window, cx| this.open_settings(window, cx)))
    }

    pub(super) fn render_mcp_status(&self, cx: &mut Context<Self>) -> Button {
        let theme = Theme::global(cx).clone();
        let language = self.language(cx);
        let state = self.mcp_server.state();
        let (status, color) = match &state {
            McpServerState::Stopped { .. } => {
                (language.text("mcp.status.stopped"), theme.muted_foreground)
            }
            McpServerState::Running { .. } => (language.text("mcp.status.running"), theme.success),
            McpServerState::Failed { .. } => (language.text("mcp.status.failed"), theme.danger),
        };
        let endpoint = state.endpoint().to_owned();
        let label = if self.mcp_status_copied {
            language.text("workspace.mcp.endpoint_copied")
        } else {
            format!(
                "{status} · {}",
                language.text("workspace.mcp.copy_endpoint")
            )
        };
        let (icon_kind, icon_color) = if self.mcp_status_copied {
            (Icon::Check, theme.success)
        } else {
            (Icon::Network, color)
        };

        Button::new("copy-mcp-endpoint")
            .ghost()
            .xsmall()
            .size(px(28.0))
            .icon(icon_kind)
            .text_color(icon_color)
            .accessibility_label(label.clone())
            .tooltip(label)
            .on_click(cx.listener(move |this, _, _, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(endpoint.clone()));
                this.mcp_status_copied = true;
                this.mcp_status_copy_generation = this.mcp_status_copy_generation.wrapping_add(1);
                let generation = this.mcp_status_copy_generation;
                cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(COPY_FEEDBACK_DURATION).await;
                    let _ = this.update(cx, |this, cx| {
                        if this.mcp_status_copy_generation == generation {
                            this.mcp_status_copied = false;
                            cx.notify();
                        }
                    });
                })
                .detach();
                cx.notify();
            }))
    }
}
