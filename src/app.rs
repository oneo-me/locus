use gpui::{
    Context, Div, FocusHandle, FontWeight, KeyDownEvent, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, Render, Role, Stateful, Window, WindowHandle, div, prelude::*,
    px, transparent_black,
};

use crate::motion::WidthTween;
use crate::settings::{self, Settings};
use crate::theme::Theme;
use crate::ui::{Icon, icon};
use crate::{OpenSettings, ToggleLeftSidebar, ToggleRightSidebar};

const TITLEBAR_HEIGHT: f32 = 48.0;
const TRAFFIC_LIGHT_CLEARANCE: f32 = 86.0;
pub(crate) const LEFT_SIDEBAR_WIDTH: f32 = 252.0;
pub(crate) const RIGHT_SIDEBAR_WIDTH: f32 = 460.0;
const LEFT_SIDEBAR_MIN_WIDTH: f32 = 180.0;
const LEFT_SIDEBAR_MAX_WIDTH: f32 = 420.0;
const RIGHT_SIDEBAR_MIN_WIDTH: f32 = 280.0;
const RIGHT_SIDEBAR_MAX_WIDTH: f32 = 1000.0;
const CONTENT_MIN_WIDTH: f32 = 360.0;

#[derive(Clone, Copy, Eq, PartialEq)]
enum PanelSide {
    Left,
    Right,
}

#[derive(Clone, Copy)]
enum ToggleLocation {
    Panel,
    Content,
}

#[derive(Clone, Copy)]
struct PanelResizeDrag {
    side: PanelSide,
    start_mouse_x: f32,
    start_width: f32,
}

#[derive(Clone, Copy)]
struct PanelFrame {
    left_content: f32,
    right_content: f32,
    left: f32,
    right: f32,
    left_sliding: bool,
    right_sliding: bool,
}

impl PanelFrame {
    fn sliding(self) -> bool {
        self.left_sliding || self.right_sliding
    }
}

pub struct Locus {
    left_sidebar_visible: bool,
    right_sidebar_visible: bool,
    left_sidebar_width: f32,
    right_sidebar_width: f32,
    rendered_left_width: f32,
    rendered_right_width: f32,
    left_sidebar_slide: Option<WidthTween>,
    right_sidebar_slide: Option<WidthTween>,
    applied_material_dark: Option<bool>,
    panel_resize_drag: Option<PanelResizeDrag>,
    header_drag_armed: bool,
    left_toggle_focus: FocusHandle,
    right_toggle_focus: FocusHandle,
    left_restore_focus: FocusHandle,
    settings_focus: FocusHandle,
    settings_window: Option<WindowHandle<Settings>>,
}

impl Locus {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            left_sidebar_visible: true,
            right_sidebar_visible: true,
            left_sidebar_width: LEFT_SIDEBAR_WIDTH,
            right_sidebar_width: RIGHT_SIDEBAR_WIDTH,
            rendered_left_width: LEFT_SIDEBAR_WIDTH,
            rendered_right_width: RIGHT_SIDEBAR_WIDTH,
            left_sidebar_slide: None,
            right_sidebar_slide: None,
            applied_material_dark: None,
            panel_resize_drag: None,
            header_drag_armed: false,
            left_toggle_focus: cx.focus_handle(),
            right_toggle_focus: cx.focus_handle(),
            left_restore_focus: cx.focus_handle(),
            settings_focus: cx.focus_handle(),
            settings_window: None,
        }
    }

    fn settle_panels(&mut self, window: &Window, dark: bool) -> PanelFrame {
        let (left_content, right_content) = fitted_panel_widths(
            f32::from(window.viewport_size().width),
            self.left_sidebar_visible || self.left_sidebar_slide.is_some(),
            self.right_sidebar_visible || self.right_sidebar_slide.is_some(),
            self.left_sidebar_width,
            self.right_sidebar_width,
        );
        let left = slide_width(
            &mut self.left_sidebar_slide,
            if self.left_sidebar_visible {
                left_content
            } else {
                0.0
            },
        );
        let right = slide_width(
            &mut self.right_sidebar_slide,
            if self.right_sidebar_visible {
                right_content
            } else {
                0.0
            },
        );

        self.rendered_left_width = left;
        self.rendered_right_width = right;

        if self.applied_material_dark != Some(dark) {
            crate::platform::set_sidebar_material_color(dark)
                .expect("failed to update macOS sidebar material color");
            self.applied_material_dark = Some(dark);
        }

        PanelFrame {
            left_content,
            right_content,
            left,
            right,
            left_sliding: self.left_sidebar_slide.is_some(),
            right_sliding: self.right_sidebar_slide.is_some(),
        }
    }

    fn toggle_panel(&mut self, side: PanelSide, cx: &mut Context<Self>) {
        match side {
            PanelSide::Left => {
                self.left_sidebar_visible = !self.left_sidebar_visible;
                self.left_sidebar_slide =
                    (!cx.reduce_motion()).then(|| WidthTween::new(self.rendered_left_width));
            }
            PanelSide::Right => {
                self.right_sidebar_visible = !self.right_sidebar_visible;
                self.right_sidebar_slide =
                    (!cx.reduce_motion()).then(|| WidthTween::new(self.rendered_right_width));
            }
        }
        cx.notify();
    }

    fn toggle_left_sidebar_action(
        &mut self,
        _: &ToggleLeftSidebar,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_panel(PanelSide::Left, cx);
    }

    fn toggle_right_sidebar_action(
        &mut self,
        _: &ToggleRightSidebar,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_panel(PanelSide::Right, cx);
    }

    fn open_settings(&mut self, cx: &mut Context<Self>) {
        if let Some(window) = self.settings_window
            && window
                .update(cx, |_, window, _| window.activate_window())
                .is_ok()
        {
            cx.activate(true);
            return;
        }

        self.settings_window = Some(settings::open_settings_window(cx));
        cx.activate(true);
    }

    fn open_settings_action(&mut self, _: &OpenSettings, _: &mut Window, cx: &mut Context<Self>) {
        self.open_settings(cx);
    }

    fn window_drag_region(&self, id: &'static str, cx: &mut Context<Self>) -> Stateful<Div> {
        div()
            .id(id)
            .h_full()
            .on_click(|event, window, _| {
                if event.click_count() == 2 {
                    window.titlebar_double_click();
                }
            })
            .on_mouse_down_out(cx.listener(|this, _, _, _| {
                this.header_drag_armed = false;
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, _| {
                    this.header_drag_armed = true;
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| {
                    this.header_drag_armed = false;
                }),
            )
            .on_mouse_move(cx.listener(|this, _, window, _| {
                if this.header_drag_armed {
                    this.header_drag_armed = false;
                    window.start_window_move();
                }
            }))
    }

    fn render_panel_toggle(
        &self,
        side: PanelSide,
        location: ToggleLocation,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let theme = Theme::current(cx);
        let (focus, id, label, icon_kind) = match (side, location) {
            (PanelSide::Left, ToggleLocation::Panel) => (
                &self.left_toggle_focus,
                "hide-left-sidebar",
                "隐藏左侧栏",
                Icon::PanelLeft,
            ),
            (PanelSide::Left, ToggleLocation::Content) => (
                &self.left_restore_focus,
                "show-left-sidebar",
                "显示左侧栏",
                Icon::PanelLeft,
            ),
            (PanelSide::Right, _) => (
                &self.right_toggle_focus,
                "toggle-right-sidebar",
                if self.right_sidebar_visible {
                    "隐藏右侧栏"
                } else {
                    "显示右侧栏"
                },
                Icon::PanelRight,
            ),
        };

        div()
            .id(id)
            .role(Role::Button)
            .aria_label(label)
            .track_focus(focus)
            .tab_index(0)
            .tab_stop(true)
            .w(px(26.0))
            .h(px(26.0))
            .flex_none()
            .rounded(px(6.0))
            .flex()
            .items_center()
            .justify_center()
            .cursor_default()
            .focus_visible(|style| style.border_1().border_color(theme.focus))
            .hover(|style| style.bg(theme.control_hover))
            .active(|style| style.bg(theme.control_active))
            .child(icon(icon_kind, 14.0, theme.muted_text))
            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                cx.stop_propagation();
            })
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    this.toggle_panel(side, cx);
                    cx.stop_propagation();
                }
            }))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.toggle_panel(side, cx);
                cx.stop_propagation();
            }))
    }

    fn render_left_titlebar(&self, cx: &mut Context<Self>) -> Div {
        div()
            .h(px(TITLEBAR_HEIGHT))
            .flex_none()
            .flex()
            .items_center()
            .child(
                self.window_drag_region("left-traffic-light-region", cx)
                    .w(px(TRAFFIC_LIGHT_CLEARANCE))
                    .flex_none(),
            )
            .child(self.render_panel_toggle(PanelSide::Left, ToggleLocation::Panel, cx))
            .child(
                self.window_drag_region("left-title-region", cx)
                    .ml(px(8.0))
                    .min_w_0()
                    .flex_1(),
            )
    }

    fn render_content_titlebar(&self, cx: &mut Context<Self>) -> Div {
        div()
            .h(px(TITLEBAR_HEIGHT))
            .flex_none()
            .flex()
            .items_center()
            .gap(px(8.0))
            .pl(if self.left_sidebar_visible {
                px(14.0 + (TRAFFIC_LIGHT_CLEARANCE - self.rendered_left_width).max(0.0))
            } else {
                px(0.0)
            })
            .pr(px(14.0))
            .when(!self.left_sidebar_visible, |header| {
                header
                    .child(
                        self.window_drag_region("content-traffic-light-region", cx)
                            .w(px(TRAFFIC_LIGHT_CLEARANCE - 8.0))
                            .flex_none(),
                    )
                    .child(self.render_panel_toggle(PanelSide::Left, ToggleLocation::Content, cx))
            })
            .child(
                self.window_drag_region("content-title-region", cx)
                    .min_w_0()
                    .flex_shrink(1.0)
                    .flex()
                    .items_center()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("未选择笔记"),
            )
            .child(
                self.window_drag_region("content-center-drag-region", cx)
                    .flex_1(),
            )
            .when(!self.right_sidebar_visible, |header| {
                header.child(div().w(px(26.0)).h(px(26.0)).flex_none())
            })
    }

    fn render_right_titlebar(&self, cx: &mut Context<Self>) -> Div {
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
            .child(div().w(px(26.0)).h(px(26.0)).flex_none())
    }

    fn render_settings_button(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let theme = Theme::current(cx);

        div()
            .id("open-settings")
            .role(Role::Button)
            .aria_label("设置")
            .aria_keyshortcuts("Meta+,")
            .track_focus(&self.settings_focus)
            .tab_index(0)
            .tab_stop(true)
            .w(px(26.0))
            .h(px(26.0))
            .flex_none()
            .rounded(px(6.0))
            .flex()
            .items_center()
            .justify_center()
            .cursor_default()
            .focus_visible(|style| style.border_1().border_color(theme.focus))
            .hover(|style| style.bg(theme.control_hover))
            .active(|style| style.bg(theme.control_active))
            .child(icon(Icon::Settings, 14.0, theme.muted_text))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    this.open_settings(cx);
                    cx.stop_propagation();
                }
            }))
            .on_click(cx.listener(|this, _, _, cx| {
                this.open_settings(cx);
            }))
    }

    fn render_panel_resize_handle(&self, side: PanelSide, cx: &mut Context<Self>) -> Stateful<Div> {
        let theme = Theme::current(cx);
        let active = self.panel_resize_drag.is_some_and(|drag| drag.side == side);

        let (handle, edge) = match side {
            PanelSide::Left => (
                div().right(px(-5.0)).w(px(10.0)),
                div().left(px(5.0)).w(px(1.0)),
            ),
            PanelSide::Right => (
                div().left(px(-7.0)).w(px(8.0)),
                div().left(px(5.0)).w(px(2.0)),
            ),
        };
        handle
            .id(match side {
                PanelSide::Left => "left-sidebar-resize-handle",
                PanelSide::Right => "right-sidebar-resize-handle",
            })
            .absolute()
            .top_0()
            .bottom_0()
            .cursor_col_resize()
            .group("panel-resize-handle")
            .child(
                edge.absolute()
                    .top_0()
                    .h_full()
                    .bg(if active {
                        theme.resize_handle
                    } else {
                        transparent_black()
                    })
                    .group_hover("panel-resize-handle", |style| style.bg(theme.resize_handle)),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    let start_width = match side {
                        PanelSide::Left => {
                            this.left_sidebar_slide = None;
                            let width = this.rendered_left_width;
                            let dark = Theme::current(cx).is_dark;
                            crate::platform::set_sidebar_material(width, dark)
                                .expect("failed to resize macOS sidebar material");
                            this.applied_material_dark = Some(dark);
                            width
                        }
                        PanelSide::Right => {
                            this.right_sidebar_slide = None;
                            this.rendered_right_width
                        }
                    };
                    this.panel_resize_drag = Some(PanelResizeDrag {
                        side,
                        start_mouse_x: f32::from(event.position.x),
                        start_width,
                    });
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
    }

    fn resize_panel_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(drag) = self.panel_resize_drag else {
            return;
        };
        let viewport_width = f32::from(window.viewport_size().width);
        let delta = f32::from(event.position.x) - drag.start_mouse_x;

        match drag.side {
            PanelSide::Left => {
                let maximum = LEFT_SIDEBAR_MAX_WIDTH
                    .min(viewport_width - CONTENT_MIN_WIDTH - self.rendered_right_width)
                    .max(LEFT_SIDEBAR_MIN_WIDTH);
                self.left_sidebar_width =
                    (drag.start_width + delta).clamp(LEFT_SIDEBAR_MIN_WIDTH, maximum);
                let dark = Theme::current(cx).is_dark;
                crate::platform::set_sidebar_material(self.left_sidebar_width, dark)
                    .expect("failed to resize macOS sidebar material");
                self.applied_material_dark = Some(dark);
            }
            PanelSide::Right => {
                let maximum = RIGHT_SIDEBAR_MAX_WIDTH
                    .min(viewport_width - CONTENT_MIN_WIDTH - self.rendered_left_width)
                    .max(RIGHT_SIDEBAR_MIN_WIDTH);
                self.right_sidebar_width =
                    (drag.start_width - delta).clamp(RIGHT_SIDEBAR_MIN_WIDTH, maximum);
            }
        }
        cx.notify();
    }

    fn finish_panel_resize(
        &mut self,
        event: &MouseUpEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.button == MouseButton::Left && self.panel_resize_drag.take().is_some() {
            cx.notify();
        }
    }

    fn render_left_sidebar(&self, width: f32, show_resize: bool, cx: &mut Context<Self>) -> Div {
        let theme = Theme::current(cx);
        let is_resizing = self
            .panel_resize_drag
            .is_some_and(|drag| drag.side == PanelSide::Left);

        div()
            .relative()
            .w(px(width))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .bg(if is_resizing {
                theme.sidebar_drag_background
            } else {
                transparent_black()
            })
            .child(self.render_left_titlebar(cx))
            .child(
                div()
                    .min_h_0()
                    .flex_1()
                    .p_4()
                    .text_sm()
                    .text_color(theme.muted_text)
                    .child("尚未打开文档库"),
            )
            .child(
                div()
                    .h(px(40.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .px(px(10.0))
                    .child(self.render_settings_button(cx)),
            )
            .when(show_resize, |panel| {
                panel.child(self.render_panel_resize_handle(PanelSide::Left, cx))
            })
    }

    fn render_content(&self, left_border: bool, cx: &mut Context<Self>) -> Div {
        let theme = Theme::current(cx);

        div()
            .relative()
            .min_w_0()
            .h_full()
            .flex_1()
            .flex()
            .flex_col()
            .when(left_border, |content| {
                content.border_l_1().border_color(theme.border)
            })
            .bg(theme.surface)
            .child(self.render_content_titlebar(cx))
            .child(div().min_h_0().flex_1())
    }

    fn render_right_sidebar(&self, width: f32, show_resize: bool, cx: &mut Context<Self>) -> Div {
        let theme = Theme::current(cx);

        div()
            .relative()
            .w(px(width))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .min_w_0()
            .border_l_1()
            .border_color(theme.border_strong)
            .bg(theme.surface)
            .child(self.render_right_titlebar(cx))
            .child(div().min_h_0().flex_1())
            .when(show_resize, |panel| {
                panel.child(self.render_panel_resize_handle(PanelSide::Right, cx))
            })
    }
}

impl Render for Locus {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::current(cx);
        let panels = self.settle_panels(window, theme.is_dark);
        if panels.sliding() {
            window.request_animation_frame();
        }

        div()
            .key_context("Locus")
            .on_action(cx.listener(Self::open_settings_action))
            .on_action(cx.listener(Self::toggle_left_sidebar_action))
            .on_action(cx.listener(Self::toggle_right_sidebar_action))
            .on_mouse_move(cx.listener(Self::resize_panel_mouse_move))
            .capture_any_mouse_up(cx.listener(Self::finish_panel_resize))
            .size_full()
            .relative()
            .flex()
            .font_family(".SystemUIFont")
            .text_color(theme.text)
            .when(panels.left > 0.0, |root| {
                root.child(
                    div()
                        .h_full()
                        .flex_none()
                        .w(px(panels.left))
                        .when(panels.left_sliding, |panel| panel.overflow_hidden())
                        .child(self.render_left_sidebar(
                            panels.left_content,
                            self.left_sidebar_visible && !panels.left_sliding,
                            cx,
                        )),
                )
            })
            .child(self.render_content(panels.left > 0.0, cx))
            .when(panels.right > 0.0, |root| {
                root.child(
                    div()
                        .h_full()
                        .flex_none()
                        .w(px(panels.right))
                        .flex()
                        .relative()
                        .when(panels.right_sliding, |panel| panel.overflow_hidden())
                        .child(
                            div()
                                .absolute()
                                .top_0()
                                .right_0()
                                .h_full()
                                .w(px(panels.right_content))
                                .child(self.render_right_sidebar(
                                    panels.right_content,
                                    self.right_sidebar_visible && !panels.right_sliding,
                                    cx,
                                )),
                        ),
                )
            })
            .child(
                div()
                    .absolute()
                    .top(px((TITLEBAR_HEIGHT - 26.0) / 2.0))
                    .right(px((TITLEBAR_HEIGHT - 26.0) / 2.0))
                    .child(self.render_panel_toggle(PanelSide::Right, ToggleLocation::Panel, cx)),
            )
    }
}

fn slide_width(slide: &mut Option<WidthTween>, target: f32) -> f32 {
    match slide.and_then(|slide| slide.width_toward(target)) {
        Some(width) => width,
        None => {
            *slide = None;
            target
        }
    }
}

fn fitted_panel_widths(
    viewport_width: f32,
    left_visible: bool,
    right_visible: bool,
    preferred_left: f32,
    preferred_right: f32,
) -> (f32, f32) {
    let left_min = if left_visible {
        LEFT_SIDEBAR_MIN_WIDTH
    } else {
        0.0
    };
    let right_min = if right_visible {
        RIGHT_SIDEBAR_MIN_WIDTH
    } else {
        0.0
    };
    let mut left = if left_visible {
        preferred_left.clamp(LEFT_SIDEBAR_MIN_WIDTH, LEFT_SIDEBAR_MAX_WIDTH)
    } else {
        0.0
    };
    let mut right = if right_visible {
        preferred_right.clamp(RIGHT_SIDEBAR_MIN_WIDTH, RIGHT_SIDEBAR_MAX_WIDTH)
    } else {
        0.0
    };

    let available = (viewport_width - CONTENT_MIN_WIDTH).max(0.0);
    let mut overflow = (left + right - available).max(0.0);
    let right_reduction = overflow.min((right - right_min).max(0.0));
    right -= right_reduction;
    overflow -= right_reduction;
    let left_reduction = overflow.min((left - left_min).max(0.0));
    left -= left_reduction;
    overflow -= left_reduction;

    if overflow > 0.0 {
        let right_reduction = overflow.min(right);
        right -= right_reduction;
        overflow -= right_reduction;
        left = (left - overflow).max(0.0);
    }

    (left, right)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_widths_preserve_the_content_minimum() {
        let (left, right) = fitted_panel_widths(880.0, true, true, 420.0, 480.0);

        assert_eq!(left, 240.0);
        assert_eq!(right, 280.0);
        assert_eq!(880.0 - left - right, CONTENT_MIN_WIDTH);
    }

    #[test]
    fn hidden_panels_do_not_consume_width() {
        assert_eq!(
            fitted_panel_widths(880.0, false, true, 420.0, 480.0),
            (0.0, 480.0)
        );
    }
}
