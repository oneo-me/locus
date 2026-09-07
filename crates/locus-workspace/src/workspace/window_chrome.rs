use super::*;
use gpui_component::{
    Disableable as _, Sizable as _,
    button::{Button, ButtonVariants as _},
};
use locus_mcp::McpServerState;
use rust_i18n::t;

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
                    t!("workspace.sidebar.hide_left", locale = language.locale()).into_owned()
                } else {
                    t!("workspace.sidebar.show_left", locale = language.locale()).into_owned()
                },
            ),
            PanelSide::Right => (
                "toggle-right-sidebar",
                if self.app_shell.workspace().visible(PanelSide::Right) {
                    t!("workspace.sidebar.hide_right", locale = language.locale()).into_owned()
                } else {
                    t!("workspace.sidebar.show_right", locale = language.locale()).into_owned()
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
                t!("workspace.navigation.back", locale = language.locale()).into_owned(),
                Icon::ArrowLeft,
                self.library_navigation.can_go_back(),
            ),
            NavigationDirection::Forward => (
                "navigate-forward",
                t!("workspace.navigation.forward", locale = language.locale()).into_owned(),
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
                                t!(
                                    "workspace.note.no_selection",
                                    locale = self.language(cx).locale()
                                )
                                .into_owned()
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
        div().h(px(TITLEBAR_HEIGHT)).flex_none().child(
            self.window_drag_region("right-title-region", cx)
                .w_full()
                .child(self.render_right_tab_bar(cx)),
        )
    }

    pub(super) fn render_settings_button(
        &self,
        cx: &mut Context<Self>,
    ) -> gpui_component::button::Button {
        let label = t!(
            "workspace.settings.open",
            locale = self.language(cx).locale()
        )
        .into_owned();
        toolbar_icon_button("open-settings", label, Icon::Settings, Theme::global(cx))
            .size(px(28.0))
            .on_click(cx.listener(|this, _, window, cx| this.open_settings(window, cx)))
    }

    pub(super) fn render_mcp_status(&self, cx: &mut Context<Self>) -> Button {
        let theme = Theme::global(cx).clone();
        let language = self.language(cx);
        let state = self.mcp_server.state();
        let (status, color) = match &state {
            McpServerState::Stopped { .. } => (
                t!("mcp.status.stopped", locale = language.locale()).into_owned(),
                theme.muted_foreground,
            ),
            McpServerState::Running { .. } => (
                t!("mcp.status.running", locale = language.locale()).into_owned(),
                theme.success,
            ),
            McpServerState::Failed { .. } => (
                t!("mcp.status.failed", locale = language.locale()).into_owned(),
                theme.danger,
            ),
        };
        let endpoint = state.endpoint().to_owned();
        let label = if self.mcp_status_copied {
            t!("workspace.mcp.endpoint_copied", locale = language.locale()).into_owned()
        } else {
            format!(
                "{status} · {}",
                t!("workspace.mcp.copy_endpoint", locale = language.locale()).into_owned()
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
