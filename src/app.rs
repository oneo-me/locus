use std::time::{Duration, Instant};

use gpui::{
    Context, Div, Entity, FocusHandle, FontWeight, KeyDownEvent, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, Render, Role, Stateful, Window, div, prelude::*, px,
    transparent_black,
};

use crate::motion::WidthTween;
use crate::settings::{Language, Preferences, Settings, SettingsEvent};
use crate::state::{
    InterfaceState, LEFT_SIDEBAR_DEFAULT_WIDTH, LEFT_SIDEBAR_MAX_WIDTH, LEFT_SIDEBAR_MIN_WIDTH,
    RIGHT_SIDEBAR_DEFAULT_WIDTH, RIGHT_SIDEBAR_MAX_WIDTH, RIGHT_SIDEBAR_MIN_WIDTH,
};
use crate::theme::Theme;
use crate::ui::{Icon, icon};
use crate::{CloseWindow, OpenSettings, ToggleFpsCounter, ToggleLeftSidebar, ToggleRightSidebar};

const TITLEBAR_HEIGHT: f32 = 48.0;
const TRAFFIC_LIGHT_CLEARANCE: f32 = 86.0;
const TITLEBAR_CONTROL_INSET: f32 = 11.0;
const TITLEBAR_CONTROL_SIZE: f32 = 26.0;
const TITLEBAR_CONTROL_GAP: f32 = 8.0;
const CONTENT_MIN_WIDTH: f32 = 360.0;

#[derive(Clone, Copy, Eq, PartialEq)]
enum PanelSide {
    Left,
    Right,
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
    root_focus: FocusHandle,
    left_sidebar_visible: bool,
    right_sidebar_visible: bool,
    left_sidebar_width: f32,
    right_sidebar_width: f32,
    rendered_left_width: f32,
    rendered_right_width: f32,
    left_sidebar_slide: Option<WidthTween>,
    right_sidebar_slide: Option<WidthTween>,
    panel_resize_drag: Option<PanelResizeDrag>,
    header_drag_armed: bool,
    left_toggle_focus: FocusHandle,
    right_toggle_focus: FocusHandle,
    settings_focus: FocusHandle,
    settings_open: bool,
    settings: Entity<Settings>,
    fps_counter_visible: bool,
    fps_frame_count: u32,
    fps_value: u32,
    fps_last_frame: Instant,
}

impl Locus {
    pub fn new(
        window: &mut Window,
        preferences: Preferences,
        load_error: Option<String>,
        interface_state: InterfaceState,
        cx: &mut Context<Self>,
    ) -> Self {
        let root_focus = cx.focus_handle();
        window.focus(&root_focus, cx);
        crate::theme::apply_preference(preferences.theme, window, cx);
        window.set_rem_size(px(preferences.ui_font_size));
        let settings = cx.new(|cx| Settings::new(preferences, load_error, root_focus.clone(), cx));
        cx.subscribe(
            &settings,
            |this, _, event: &SettingsEvent, cx| match event {
                SettingsEvent::Close => {
                    this.settings_open = false;
                    cx.notify();
                }
                SettingsEvent::LanguageChanged(language) => {
                    crate::set_app_menus(cx, *language);
                    cx.notify();
                }
            },
        )
        .detach();
        cx.observe_window_appearance(window, |this, window, cx| {
            let preference = this.settings.read(cx).preferences().theme;
            if preference == crate::theme::ThemePreference::System {
                crate::theme::apply_preference(preference, window, cx);
                cx.notify();
            }
        })
        .detach();

        Self {
            root_focus,
            left_sidebar_visible: interface_state.left_sidebar_visible,
            right_sidebar_visible: interface_state.right_sidebar_visible,
            left_sidebar_width: interface_state.left_sidebar_width,
            right_sidebar_width: interface_state.right_sidebar_width,
            rendered_left_width: if interface_state.left_sidebar_visible {
                interface_state.left_sidebar_width
            } else {
                0.0
            },
            rendered_right_width: if interface_state.right_sidebar_visible {
                interface_state.right_sidebar_width
            } else {
                0.0
            },
            left_sidebar_slide: None,
            right_sidebar_slide: None,
            panel_resize_drag: None,
            header_drag_armed: false,
            left_toggle_focus: cx.focus_handle(),
            right_toggle_focus: cx.focus_handle(),
            settings_focus: cx.focus_handle(),
            settings_open: false,
            settings,
            fps_counter_visible: interface_state.fps_counter_visible,
            fps_frame_count: 0,
            fps_value: 0,
            fps_last_frame: Instant::now(),
        }
    }

    fn settle_panels(&mut self, window: &Window) -> PanelFrame {
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
        self.save_interface_state();
        cx.notify();
    }

    fn interface_state(&self) -> InterfaceState {
        InterfaceState {
            left_sidebar_width: self.left_sidebar_width,
            right_sidebar_width: self.right_sidebar_width,
            left_sidebar_visible: self.left_sidebar_visible,
            right_sidebar_visible: self.right_sidebar_visible,
            fps_counter_visible: self.fps_counter_visible,
        }
    }

    fn save_interface_state(&self) {
        if let Err(error) = self.interface_state().save() {
            eprintln!("无法保存界面状态：{error}");
        }
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

    fn language(&self, cx: &Context<Self>) -> Language {
        self.settings.read(cx).preferences().language
    }

    fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.settings_open = true;
        self.settings
            .update(cx, |settings, cx| settings.focus(window, cx));
        cx.notify();
    }

    fn open_settings_action(
        &mut self,
        _: &OpenSettings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_settings(window, cx);
    }

    fn close_window_action(
        &mut self,
        _: &CloseWindow,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        crate::platform::close_main_window(window, cx);
    }

    fn toggle_fps_counter_action(
        &mut self,
        _: &ToggleFpsCounter,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.fps_counter_visible = !self.fps_counter_visible;
        self.fps_frame_count = 0;
        self.fps_value = 0;
        self.fps_last_frame = Instant::now();
        self.save_interface_state();
        cx.notify();
    }

    fn tick_fps(&mut self, window: &Window) {
        let now = Instant::now();
        self.fps_frame_count = self.fps_frame_count.saturating_add(1);
        let elapsed = now.duration_since(self.fps_last_frame);
        if elapsed >= Duration::from_secs(1) {
            self.fps_value = (self.fps_frame_count as f32 / elapsed.as_secs_f32()).round() as u32;
            self.fps_frame_count = 0;
            self.fps_last_frame = now;
        }
        window.request_animation_frame();
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

    fn render_panel_toggle(&self, side: PanelSide, cx: &mut Context<Self>) -> Stateful<Div> {
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

        div()
            .id(id)
            .role(Role::Button)
            .aria_label(label)
            .track_focus(focus)
            .tab_index(0)
            .tab_stop(true)
            .w(px(TITLEBAR_CONTROL_SIZE))
            .h(px(TITLEBAR_CONTROL_SIZE))
            .flex_none()
            .rounded(px(6.0))
            .flex()
            .items_center()
            .justify_center()
            .cursor_default()
            .focus_visible(|style| style.border_1().border_color(theme.focus))
            .hover(|style| style.bg(theme.control_hover))
            .active(|style| style.bg(theme.control_active))
            .child(icon(icon_kind, 14.0, theme.text_secondary))
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

    fn render_left_titlebar(&self, left_control_clearance: f32, cx: &mut Context<Self>) -> Div {
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

    fn render_content_titlebar(&self, left_control_clearance: f32, cx: &mut Context<Self>) -> Div {
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
                    .child(self.language(cx).text("未选择笔记", "No note selected")),
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

    fn render_fps_counter(&self, cx: &mut Context<Self>) -> Div {
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
            .child(div().size(px(TITLEBAR_CONTROL_SIZE)).flex_none())
    }

    fn render_settings_button(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let theme = Theme::current(cx);
        let label = self.language(cx).text("设置", "Settings");

        div()
            .id("open-settings")
            .role(Role::Button)
            .aria_label(label)
            .aria_keyshortcuts("Meta+,")
            .track_focus(&self.settings_focus)
            .tab_index(0)
            .tab_stop(true)
            .w(px(TITLEBAR_CONTROL_SIZE))
            .h(px(TITLEBAR_CONTROL_SIZE))
            .flex_none()
            .rounded(px(6.0))
            .flex()
            .items_center()
            .justify_center()
            .cursor_default()
            .focus_visible(|style| style.border_1().border_color(theme.focus))
            .hover(|style| style.bg(theme.control_hover))
            .active(|style| style.bg(theme.control_active))
            .child(icon(Icon::Settings, 14.0, theme.text_secondary))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    this.open_settings(window, cx);
                    cx.stop_propagation();
                }
            }))
            .on_click(cx.listener(|this, _, window, cx| {
                this.open_settings(window, cx);
            }))
    }

    fn render_panel_resize_handle(&self, side: PanelSide, cx: &mut Context<Self>) -> Stateful<Div> {
        let theme = Theme::current(cx);
        let active = self.panel_resize_drag.is_some_and(|drag| drag.side == side);

        let handle = match side {
            PanelSide::Left => div().right(px(-5.0)).w(px(10.0)),
            PanelSide::Right => div().left(px(-5.0)).w(px(10.0)),
        };
        let edge = div()
            .left(px(5.0))
            .w(px(1.0))
            .absolute()
            .top_0()
            .h_full()
            .bg(if active && side == PanelSide::Left {
                theme.resize_handle
            } else {
                transparent_black()
            })
            .when(!active && side == PanelSide::Left, |edge| {
                edge.group_hover("panel-resize-handle", |style| style.bg(theme.resize_handle))
            });
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
            .child(edge)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    if event.click_count == 2 {
                        this.reset_panel_width(side, cx);
                        cx.stop_propagation();
                        return;
                    }
                    let start_width = match side {
                        PanelSide::Left => {
                            this.left_sidebar_slide = None;
                            this.rendered_left_width
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

    fn reset_panel_width(&mut self, side: PanelSide, cx: &mut Context<Self>) {
        self.panel_resize_drag = None;
        match side {
            PanelSide::Left => {
                self.left_sidebar_width = LEFT_SIDEBAR_DEFAULT_WIDTH;
                self.left_sidebar_slide = None;
            }
            PanelSide::Right => {
                self.right_sidebar_width = RIGHT_SIDEBAR_DEFAULT_WIDTH;
                self.right_sidebar_slide = None;
            }
        }
        self.save_interface_state();
        cx.notify();
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
            self.save_interface_state();
            cx.notify();
        }
    }

    fn render_left_sidebar(
        &self,
        width: f32,
        show_resize: bool,
        left_control_clearance: f32,
        cx: &mut Context<Self>,
    ) -> Div {
        let theme = Theme::current(cx);
        let language = self.language(cx);
        let library = self
            .settings
            .read(cx)
            .preferences()
            .library_path
            .as_deref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .map(str::to_owned)
            .unwrap_or_else(|| {
                language
                    .text("尚未打开文档库", "No library selected")
                    .to_owned()
            });
        div()
            .relative()
            .w(px(width))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .bg(transparent_black())
            .child(self.render_left_titlebar(left_control_clearance, cx))
            .child(
                div()
                    .min_h_0()
                    .flex_1()
                    .p_4()
                    .text_sm()
                    .text_color(theme.text_secondary)
                    .child(library),
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

    fn render_content(
        &self,
        left_border: bool,
        left_control_clearance: f32,
        cx: &mut Context<Self>,
    ) -> Div {
        let theme = Theme::current(cx);

        div()
            .relative()
            .min_w_0()
            .h_full()
            .flex_1()
            .flex()
            .flex_col()
            .bg(theme.surface)
            .child(self.render_content_titlebar(left_control_clearance, cx))
            .child(div().min_h_0().flex_1())
            .when(left_border, |content| {
                content.child(
                    div()
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left_0()
                        .w(px(1.0))
                        .bg(theme.border),
                )
            })
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
        let panels = self.settle_panels(window);
        let left_toggle_offset = left_toggle_offset(window.is_fullscreen());
        let left_control_clearance =
            left_toggle_offset + TITLEBAR_CONTROL_SIZE + TITLEBAR_CONTROL_GAP;
        if panels.sliding() {
            window.request_animation_frame();
        }
        if self.fps_counter_visible {
            self.tick_fps(window);
        }

        let workspace = div()
            .size_full()
            .relative()
            .flex()
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
                            left_control_clearance,
                            cx,
                        )),
                )
            })
            .child(self.render_content(panels.left > 0.0, left_control_clearance, cx))
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
                    .top(px(TITLEBAR_CONTROL_INSET))
                    .left(px(left_toggle_offset))
                    .child(self.render_panel_toggle(PanelSide::Left, cx)),
            )
            .child(
                div()
                    .absolute()
                    .top(px(TITLEBAR_CONTROL_INSET))
                    .right(px(TITLEBAR_CONTROL_INSET))
                    .child(self.render_panel_toggle(PanelSide::Right, cx)),
            );

        div()
            .key_context("Locus")
            .track_focus(&self.root_focus)
            .on_action(cx.listener(Self::close_window_action))
            .on_action(cx.listener(Self::open_settings_action))
            .on_action(cx.listener(Self::toggle_fps_counter_action))
            .on_action(cx.listener(Self::toggle_left_sidebar_action))
            .on_action(cx.listener(Self::toggle_right_sidebar_action))
            .on_mouse_move(cx.listener(Self::resize_panel_mouse_move))
            .capture_any_mouse_up(cx.listener(Self::finish_panel_resize))
            .size_full()
            .relative()
            .flex()
            .bg(theme.window_background)
            .font_family(".SystemUIFont")
            .text_color(theme.text)
            .when(self.settings_open, |root| root.child(self.settings.clone()))
            .when(!self.settings_open, |root| root.child(workspace))
    }
}

fn left_toggle_offset(fullscreen: bool) -> f32 {
    if cfg!(target_os = "macos") && !fullscreen {
        TRAFFIC_LIGHT_CLEARANCE
    } else {
        TITLEBAR_CONTROL_INSET
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

    #[test]
    fn fullscreen_left_toggle_uses_the_window_inset() {
        assert_eq!(left_toggle_offset(true), TITLEBAR_CONTROL_INSET);
    }
}
