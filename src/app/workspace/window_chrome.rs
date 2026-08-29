use super::*;

impl Locus {
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
        let theme = Theme::current(cx);
        let language = self.language(cx);
        let (focus, id, label, icon_kind) = match side {
            PanelSide::Left => (
                &self.left_toggle_focus,
                "toggle-left-sidebar",
                if self.left_sidebar_visible {
                    language.text("隐藏左侧栏", "Hide left sidebar")
                } else {
                    language.text("显示左侧栏", "Show left sidebar")
                },
                Icon::PanelLeft,
            ),
            PanelSide::Right => (
                &self.right_toggle_focus,
                "toggle-right-sidebar",
                if self.right_sidebar_visible {
                    language.text("隐藏右侧栏", "Hide right sidebar")
                } else {
                    language.text("显示右侧栏", "Show right sidebar")
                },
                Icon::PanelRight,
            ),
        };

        toolbar_icon_button(id, label, focus, icon_kind, theme)
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_activation(cx, move |this, _, cx| this.toggle_panel(side, cx))
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
            .pl(px(
                (left_control_clearance - self.rendered_left_width).max(14.0)
            ))
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
            .when(self.fps_counter_visible, |header| {
                header.child(self.render_fps_counter(cx))
            })
            .when(!self.right_sidebar_visible, |header| {
                header.child(div().size(px(TITLEBAR_CONTROL_SIZE)).flex_none())
            })
    }

    pub(super) fn render_fps_counter(&self, cx: &mut Context<Self>) -> Div {
        let theme = Theme::current(cx);
        div()
            .h(px(26.0))
            .flex_none()
            .px(px(6.0))
            .flex()
            .items_center()
            .gap(px(5.0))
            .text_xs()
            .text_color(theme.text_tertiary)
            .child(
                div()
                    .size(px(6.0))
                    .rounded_full()
                    .bg(if self.fps_value > 0 {
                        theme.focus
                    } else {
                        theme.text_tertiary
                    }),
            )
            .child(format!("{} FPS", self.fps_value))
    }

    pub(super) fn render_right_titlebar(&self, cx: &mut Context<Self>) -> Div {
        div()
            .h(px(TITLEBAR_HEIGHT))
            .flex_none()
            .flex()
            .items_center()
            .pl(px(10.0))
            .pr(px(14.0))
            .child(
                self.window_drag_region("right-title-region", cx)
                    .min_w_0()
                    .flex_1(),
            )
            .child(div().size(px(TITLEBAR_CONTROL_SIZE)).flex_none())
    }

    pub(super) fn render_settings_button(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let theme = Theme::current(cx);
        let label = self.language(cx).text("设置", "Settings");
        toolbar_icon_button(
            "open-settings",
            label,
            &self.settings_focus,
            Icon::Settings,
            theme,
        )
        .aria_keyshortcuts("Meta+,")
        .on_activation(cx, |this, window, cx| this.open_settings(window, cx))
    }
}
