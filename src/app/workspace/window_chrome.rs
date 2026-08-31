use super::*;
use crate::mcp::McpServerState;
use gpui_component::{
    Icon as ComponentIcon,
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
        self.save_interface_state();
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
    ) -> Stateful<Div> {
        let theme = Theme::global(cx).clone();
        let language = self.language(cx);
        let (focus, id, label) = match side {
            PanelSide::Left => (
                &self.left_toggle_focus,
                "toggle-left-sidebar",
                if self.app_shell.workspace().visible(PanelSide::Left) {
                    language.text("隐藏左侧栏", "Hide left sidebar")
                } else {
                    language.text("显示左侧栏", "Show left sidebar")
                },
            ),
            PanelSide::Right => (
                &self.right_toggle_focus,
                "toggle-right-sidebar",
                if self.app_shell.workspace().visible(PanelSide::Right) {
                    language.text("隐藏右侧栏", "Hide right sidebar")
                } else {
                    language.text("显示右侧栏", "Show right sidebar")
                },
            ),
        };

        panel_toggle_button(id, label, focus, side, &theme)
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_activation(cx, move |this, window, cx| {
                this.toggle_panel(side, window, cx)
            })
    }

    pub(super) fn render_history_button(
        &self,
        direction: NavigationDirection,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let theme = Theme::global(cx).clone();
        let language = self.language(cx);
        let (id, label, focus, icon_kind, enabled) = match direction {
            NavigationDirection::Back => (
                "navigate-back",
                language.text("后退", "Back"),
                &self.navigation_back_focus,
                Icon::ArrowLeft,
                self.library_navigation.can_go_back(),
            ),
            NavigationDirection::Forward => (
                "navigate-forward",
                language.text("前进", "Forward"),
                &self.navigation_forward_focus,
                Icon::ArrowRight,
                self.library_navigation.can_go_forward(),
            ),
        };

        div()
            .id(id)
            .role(Role::Button)
            .aria_label(label)
            .size(px(TITLEBAR_CONTROL_SIZE))
            .flex_none()
            .rounded(px(6.0))
            .flex()
            .items_center()
            .justify_center()
            .cursor_default()
            .when(!enabled, |button| button.opacity(0.35))
            .when(enabled, |button| {
                button
                    .track_focus(focus)
                    .tab_index(0)
                    .tab_stop(true)
                    .focus_visible(|style| style.border_1().border_color(theme.ring))
                    .hover(|style| style.bg(theme.accent))
                    .active(|style| style.bg(theme.accent))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_activation(cx, move |this, _, cx| match direction {
                        NavigationDirection::Back => this.navigate_back(cx),
                        NavigationDirection::Forward => this.navigate_forward(cx),
                    })
            })
            .child(icon(icon_kind, 14.0, theme.muted_foreground))
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
                                self.language(cx)
                                    .text("未选择笔记", "No note selected")
                                    .to_owned()
                            }),
                    ),
            )
            .child(
                self.window_drag_region("content-center-drag-region", cx)
                    .flex_1(),
            )
            .when(self.app_shell.fps_counter().visible(), |header| {
                header.child(self.render_fps_counter(cx))
            })
            .child(div().size(px(TITLEBAR_CONTROL_SIZE)).flex_none())
    }

    pub(super) fn render_fps_counter(&self, cx: &mut Context<Self>) -> Div {
        oneo_gpui_app_shell::fps_counter(self.app_shell.fps_counter(), Theme::global(cx))
    }

    pub(super) fn render_right_titlebar(&self, cx: &mut Context<Self>) -> Div {
        let theme = Theme::global(cx).clone();
        let label = self.language(cx).text("终端", "Terminal");
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
                            .selected_index(0)
                            .max_width(px(176.0))
                            .child(
                                Tab::new().aria_label(label).child(
                                    div()
                                        .min_w_0()
                                        .flex()
                                        .items_center()
                                        .gap(px(6.0))
                                        .child(ComponentIcon::new(Icon::SquareTerminal))
                                        .child(div().min_w_0().flex_1().truncate().child(label)),
                                ),
                            ),
                    ),
            )
    }

    pub(super) fn render_settings_button(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let theme = Theme::global(cx).clone();
        let label = self.language(cx).text("设置", "Settings");
        toolbar_icon_button(
            "open-settings",
            label,
            &self.settings_focus,
            Icon::Settings,
            &theme,
        )
        .size(px(28.0))
        .aria_keyshortcuts("Meta+,")
        .on_activation(cx, |this, window, cx| this.open_settings(window, cx))
    }

    pub(super) fn render_mcp_status(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let theme = Theme::global(cx).clone();
        let language = self.language(cx);
        let state = self.mcp_server.state();
        let (status, color) = match &state {
            McpServerState::Stopped { .. } => {
                (language.text("已停止", "Stopped"), theme.muted_foreground)
            }
            McpServerState::Running { .. } => (language.text("运行中", "Running"), theme.success),
            McpServerState::Failed { .. } => {
                (language.text("启动失败", "Failed to start"), theme.danger)
            }
        };
        let endpoint = state.endpoint().to_owned();
        let label = if self.mcp_status_copied {
            language.text("MCP 地址已复制", "MCP URL copied").to_owned()
        } else {
            format!(
                "{status} · {}",
                language.text("复制 MCP 地址", "Copy MCP URL")
            )
        };
        let (icon_kind, icon_color) = if self.mcp_status_copied {
            (Icon::Check, theme.success)
        } else {
            (Icon::Network, color)
        };

        div()
            .id("copy-mcp-endpoint")
            .role(Role::Button)
            .aria_label(label)
            .track_focus(&self.mcp_status_focus)
            .tab_index(0)
            .tab_stop(true)
            .size(px(28.0))
            .flex_none()
            .rounded(px(6.0))
            .flex()
            .items_center()
            .justify_center()
            .cursor_default()
            .focus_visible(|style| style.border_1().border_color(theme.ring))
            .hover(|style| style.bg(theme.accent))
            .active(|style| style.bg(theme.accent))
            .child(icon(icon_kind, 14.0, icon_color))
            .on_activation(cx, move |this, _, cx| {
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
            })
    }
}
