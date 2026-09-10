use super::*;
use gpui_kit::Role;
use gpui_kit::component::{Disableable as _, button::Button};
use locus_mcp::McpServerState;
use rust_i18n::t;

impl Locus {
    pub(super) fn toggle_panel(&mut self, cx: &mut Context<Self>) {
        self.app_shell.workspace_mut().toggle(cx.reduce_motion());
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

        oneo_gpui::toolbar_icon_button(id, label, icon_kind, &theme)
            .when(!enabled, |button| button.opacity(0.35))
            .disabled(!enabled)
            .on_click(cx.listener(move |this, _, _, cx| match direction {
                NavigationDirection::Back => this.navigate_back(cx),
                NavigationDirection::Forward => this.navigate_forward(cx),
            }))
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
                self.app_shell.workspace().rendered_width(),
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
    }

    pub(super) fn render_mcp_status(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let theme = Theme::global(cx).clone();
        let language = self.language(cx);
        let state = self.mcp_server.state();
        let (status, color) = match &state {
            McpServerState::Stopped => (
                t!("mcp.status.stopped", locale = language.locale()).into_owned(),
                theme.muted_foreground,
            ),
            McpServerState::Running => (
                t!("mcp.status.running", locale = language.locale()).into_owned(),
                theme.success,
            ),
            McpServerState::Failed { .. } => (
                t!("mcp.status.failed", locale = language.locale()).into_owned(),
                theme.danger,
            ),
        };
        let label = format!("MCP · {status}");
        div()
            .id("mcp-status")
            .role(Role::Status)
            .aria_label(label.clone())
            .tooltip(move |window, cx| {
                gpui_kit::component::tooltip::Tooltip::new(label.clone()).build(window, cx)
            })
            .flex()
            .items_center()
            .justify_center()
            .size(gpui_kit::rems(1.75))
            .text_color(color)
            .child(gpui_kit::component::Icon::new(Icon::Network).size(gpui_kit::rems(0.875)))
    }
}
