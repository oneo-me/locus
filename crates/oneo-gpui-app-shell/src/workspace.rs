use std::time::{Duration, Instant};

use super::FpsCounterState;
use gpui::{
    AnyElement, Div, Hsla, Stateful, Window, div, ease_out_quint, prelude::*, px, transparent_black,
};
use gpui_component::{IconName, Theme};

use crate::toolbar_icon_button;

const PANEL_SLIDE_DURATION: Duration = Duration::from_millis(200);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PanelSide {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug)]
pub struct PanelSize {
    pub min: f32,
    pub max: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct WorkspaceShellConfig {
    pub content_min_width: f32,
    pub left: PanelSize,
    pub right: PanelSize,
}

#[derive(Clone, Copy, Debug)]
pub struct PanelFrame {
    pub left_content: f32,
    pub right_content: f32,
    pub left: f32,
    pub right: f32,
    pub left_sliding: bool,
    pub right_sliding: bool,
}

impl PanelFrame {
    pub fn sliding(self) -> bool {
        self.left_sliding || self.right_sliding
    }
}

#[derive(Clone, Copy)]
struct PanelSlide {
    from: f32,
    started: Instant,
}

#[derive(Clone, Copy, Debug)]
struct PanelWindowResize {
    window_width: f32,
    panel_width: f32,
    panel_content_width: f32,
    minimum_window_width: f32,
    enabled: bool,
}

#[derive(Clone, Copy, Debug)]
struct PanelWindowFrame {
    pub panel_width: f32,
    pub window_resize_complete: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppShellMode {
    Workspace,
    Settings,
}

#[derive(Clone, Copy, Debug)]
pub struct WorkspaceShellSnapshot {
    pub left_visible: bool,
    pub right_visible: bool,
    pub left_width: f32,
    pub right_width: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct WorkspaceShellLayout {
    pub frame: PanelFrame,
    pub left_transition_finished: bool,
    pub right_transition_finished: bool,
}

#[derive(Clone, Copy, Debug)]
struct PanelResizeDrag {
    side: PanelSide,
    start_mouse_x: f32,
    start_width: f32,
}

pub struct AppShellState {
    mode: AppShellMode,
    workspace: WorkspaceShellState,
    fps_counter: FpsCounterState,
}

pub struct WorkspaceShellState {
    config: WorkspaceShellConfig,
    left_visible: bool,
    right_visible: bool,
    left_width: f32,
    right_width: f32,
    rendered_left_width: f32,
    rendered_right_width: f32,
    left_slide: Option<PanelSlide>,
    right_slide: Option<PanelSlide>,
    right_window_resize: Option<PanelWindowResize>,
    resize_drag: Option<PanelResizeDrag>,
}

impl AppShellState {
    pub fn new(workspace: WorkspaceShellState) -> Self {
        Self {
            mode: AppShellMode::Workspace,
            workspace,
            fps_counter: FpsCounterState::new(false),
        }
    }

    pub fn with_fps_counter_visible(mut self, visible: bool) -> Self {
        self.fps_counter = FpsCounterState::new(visible);
        self
    }

    pub fn mode(&self) -> AppShellMode {
        self.mode
    }

    pub fn is_settings_open(&self) -> bool {
        self.mode == AppShellMode::Settings
    }

    pub fn open_settings(&mut self) {
        self.mode = AppShellMode::Settings;
    }

    pub fn close_settings(&mut self) {
        self.mode = AppShellMode::Workspace;
    }

    pub fn workspace(&self) -> &WorkspaceShellState {
        &self.workspace
    }

    pub fn workspace_mut(&mut self) -> &mut WorkspaceShellState {
        &mut self.workspace
    }

    pub fn fps_counter(&self) -> &FpsCounterState {
        &self.fps_counter
    }

    pub fn fps_counter_mut(&mut self) -> &mut FpsCounterState {
        &mut self.fps_counter
    }
}

impl WorkspaceShellState {
    pub fn new(
        config: WorkspaceShellConfig,
        left_visible: bool,
        right_visible: bool,
        left_width: f32,
        right_width: f32,
    ) -> Self {
        let left_width = left_width.clamp(config.left.min, config.left.max);
        let right_width = right_width.clamp(config.right.min, config.right.max);
        Self {
            config,
            left_visible,
            right_visible,
            left_width,
            right_width,
            rendered_left_width: if left_visible { left_width } else { 0.0 },
            rendered_right_width: if right_visible { right_width } else { 0.0 },
            left_slide: None,
            right_slide: None,
            right_window_resize: None,
            resize_drag: None,
        }
    }

    pub fn snapshot(&self) -> WorkspaceShellSnapshot {
        WorkspaceShellSnapshot {
            left_visible: self.left_visible,
            right_visible: self.right_visible,
            left_width: self.left_width,
            right_width: self.right_width,
        }
    }

    pub fn visible(&self, side: PanelSide) -> bool {
        match side {
            PanelSide::Left => self.left_visible,
            PanelSide::Right => self.right_visible,
        }
    }

    pub fn rendered_width(&self, side: PanelSide) -> f32 {
        match side {
            PanelSide::Left => self.rendered_left_width,
            PanelSide::Right => self.rendered_right_width,
        }
    }

    pub fn transitioning(&self, side: PanelSide) -> bool {
        match side {
            PanelSide::Left => self.left_slide.is_some(),
            PanelSide::Right => self.right_slide.is_some() || self.right_window_resize.is_some(),
        }
    }

    pub fn resize_active(&self, side: PanelSide) -> bool {
        self.resize_drag.is_some_and(|drag| drag.side == side)
    }

    pub fn fitted_content_widths(&self, viewport_width: f32) -> (f32, f32) {
        let layout_width = self
            .right_window_resize
            .map(PanelWindowResize::layout_width)
            .unwrap_or(viewport_width);
        let (left, right) = fitted_panel_widths(
            self.config,
            layout_width,
            self.left_visible || self.left_slide.is_some(),
            self.right_visible || self.right_slide.is_some(),
            self.left_width,
            self.right_width,
        );
        (
            left,
            self.right_window_resize
                .map(PanelWindowResize::panel_content_width)
                .unwrap_or(right),
        )
    }

    pub fn panel_content_width(&self, side: PanelSide, viewport_width: f32) -> f32 {
        let (left, right) = self.fitted_content_widths(viewport_width);
        match side {
            PanelSide::Left => left,
            PanelSide::Right => right,
        }
    }

    pub fn toggle(
        &mut self,
        side: PanelSide,
        window: &Window,
        reduce_motion: bool,
        minimum_window_width: f32,
    ) {
        match side {
            PanelSide::Left => {
                self.left_visible = !self.left_visible;
                self.left_slide =
                    (!reduce_motion).then(|| PanelSlide::new(self.rendered_left_width));
            }
            PanelSide::Right => {
                let panel_content_width = self
                    .right_window_resize
                    .map(PanelWindowResize::panel_content_width)
                    .unwrap_or_else(|| {
                        if self.right_visible {
                            self.rendered_right_width
                        } else if window.is_fullscreen() || window.is_maximized() {
                            fitted_panel_widths(
                                self.config,
                                f32::from(window.viewport_size().width),
                                self.left_visible || self.left_slide.is_some(),
                                true,
                                self.left_width,
                                self.right_width,
                            )
                            .1
                        } else {
                            self.right_width
                        }
                    });
                self.right_visible = !self.right_visible;
                self.right_slide =
                    (!reduce_motion).then(|| PanelSlide::new(self.rendered_right_width));
                self.right_window_resize = Some(PanelWindowResize::new(
                    window,
                    self.rendered_right_width,
                    panel_content_width,
                    minimum_window_width,
                ));
            }
        }
    }

    pub fn layout(&mut self, window: &mut Window) -> WorkspaceShellLayout {
        let left_was_transitioning = self.left_slide.is_some();
        let right_was_transitioning =
            self.right_slide.is_some() || self.right_window_resize.is_some();
        let (left_content, right_content) =
            self.fitted_content_widths(f32::from(window.viewport_size().width));
        let left = slide_width(
            &mut self.left_slide,
            if self.left_visible { left_content } else { 0.0 },
        );
        let right = slide_width(
            &mut self.right_slide,
            if self.right_visible {
                right_content
            } else {
                0.0
            },
        );
        self.rendered_left_width = left;
        let mut frame = PanelFrame {
            left_content,
            right_content,
            left,
            right,
            left_sliding: self.left_slide.is_some(),
            right_sliding: self.right_slide.is_some(),
        };

        if let Some(resize) = self.right_window_resize {
            let window_frame = resize.sync(frame.right, window);
            frame.right = window_frame.panel_width;
            if self.right_slide.is_none() && window_frame.window_resize_complete {
                self.right_window_resize = None;
                frame.right = if self.right_visible {
                    frame.right_content
                } else {
                    0.0
                };
            }
        }
        frame.right_sliding = self.right_slide.is_some() || self.right_window_resize.is_some();
        self.rendered_right_width = frame.right;
        if frame.sliding() {
            window.request_animation_frame();
        }

        WorkspaceShellLayout {
            frame,
            left_transition_finished: left_was_transitioning && self.left_slide.is_none(),
            right_transition_finished: right_was_transitioning
                && self.right_slide.is_none()
                && self.right_window_resize.is_none(),
        }
    }

    pub fn begin_resize(&mut self, side: PanelSide, pointer_x: f32) {
        let start_width = match side {
            PanelSide::Left => {
                self.left_slide = None;
                self.rendered_left_width
            }
            PanelSide::Right => {
                self.right_slide = None;
                self.right_window_resize = None;
                self.rendered_right_width
            }
        };
        self.resize_drag = Some(PanelResizeDrag {
            side,
            start_mouse_x: pointer_x,
            start_width,
        });
    }

    pub fn resize(&mut self, pointer_x: f32, viewport_width: f32) -> bool {
        let Some(drag) = self.resize_drag else {
            return false;
        };
        let delta = pointer_x - drag.start_mouse_x;
        match drag.side {
            PanelSide::Left => {
                let maximum = self
                    .config
                    .left
                    .max
                    .min(viewport_width - self.config.content_min_width - self.rendered_right_width)
                    .max(self.config.left.min);
                self.left_width = (drag.start_width + delta).clamp(self.config.left.min, maximum);
            }
            PanelSide::Right => {
                let maximum = self
                    .config
                    .right
                    .max
                    .min(viewport_width - self.config.content_min_width - self.rendered_left_width)
                    .max(self.config.right.min);
                self.right_width = (drag.start_width - delta).clamp(self.config.right.min, maximum);
            }
        }
        true
    }

    pub fn finish_resize(&mut self) -> bool {
        self.resize_drag.take().is_some()
    }

    pub fn reset_width(&mut self, side: PanelSide, width: f32) {
        self.resize_drag = None;
        match side {
            PanelSide::Left => {
                self.left_width = width.clamp(self.config.left.min, self.config.left.max);
                self.left_slide = None;
            }
            PanelSide::Right => {
                self.right_width = width.clamp(self.config.right.min, self.config.right.max);
                self.right_slide = None;
                self.right_window_resize = None;
            }
        }
    }
}

impl PanelWindowResize {
    fn new(
        window: &Window,
        panel_width: f32,
        panel_content_width: f32,
        minimum_window_width: f32,
    ) -> Self {
        Self {
            window_width: f32::from(window.viewport_size().width),
            panel_width,
            panel_content_width,
            minimum_window_width,
            enabled: !window.is_fullscreen() && !window.is_maximized(),
        }
    }

    fn panel_content_width(self) -> f32 {
        self.panel_content_width
    }

    fn layout_width(self) -> f32 {
        if self.enabled {
            self.window_width + self.panel_content_width - self.panel_width
        } else {
            self.window_width
        }
    }

    fn sync(self, panel_width: f32, window: &mut Window) -> PanelWindowFrame {
        if !self.enabled {
            return PanelWindowFrame {
                panel_width,
                window_resize_complete: true,
            };
        }
        let viewport_size = window.viewport_size();
        let target_width = self.target_window_width(panel_width);
        let current_width = f32::from(viewport_size.width);
        let window_resize_complete = (current_width - target_width).abs() <= 0.5;
        let panel_width = self.synced_panel_width(panel_width, current_width, target_width);
        if !window_resize_complete {
            window.resize(gpui::size(px(target_width), viewport_size.height));
            window.request_animation_frame();
        }
        PanelWindowFrame {
            panel_width,
            window_resize_complete,
        }
    }

    fn target_window_width(self, panel_width: f32) -> f32 {
        (self.window_width + panel_width - self.panel_width).max(self.minimum_window_width)
    }

    fn synced_panel_width(
        self,
        panel_width: f32,
        current_window_width: f32,
        target_window_width: f32,
    ) -> f32 {
        let window_delta = target_window_width - self.window_width;
        if window_delta.abs() <= 0.5 {
            return panel_width;
        }
        let progress = ((current_window_width - self.window_width) / window_delta).clamp(0.0, 1.0);
        self.panel_width + (panel_width - self.panel_width) * progress
    }
}

impl PanelSlide {
    fn new(from: f32) -> Self {
        Self {
            from,
            started: Instant::now(),
        }
    }

    fn width_toward(self, target: f32) -> Option<f32> {
        width_at(self.from, target, self.started.elapsed())
    }
}

fn slide_width(slide: &mut Option<PanelSlide>, target: f32) -> f32 {
    match slide.and_then(|slide| slide.width_toward(target)) {
        Some(width) => width,
        None => {
            *slide = None;
            target
        }
    }
}

fn fitted_panel_widths(
    config: WorkspaceShellConfig,
    viewport_width: f32,
    left_visible: bool,
    right_visible: bool,
    preferred_left: f32,
    preferred_right: f32,
) -> (f32, f32) {
    let left_min = if left_visible { config.left.min } else { 0.0 };
    let right_min = if right_visible { config.right.min } else { 0.0 };
    let mut left = if left_visible {
        preferred_left.clamp(config.left.min, config.left.max)
    } else {
        0.0
    };
    let mut right = if right_visible {
        preferred_right.clamp(config.right.min, config.right.max)
    } else {
        0.0
    };

    let available = (viewport_width - config.content_min_width).max(0.0);
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

pub fn panel_toggle_button(
    id: &'static str,
    label: impl Into<gpui::SharedString>,
    side: PanelSide,
    theme: &Theme,
) -> gpui_component::button::Button {
    let icon = match side {
        PanelSide::Left => IconName::PanelLeft,
        PanelSide::Right => IconName::PanelRight,
    };
    toolbar_icon_button(id, label, icon, theme)
}

pub fn panel_resize_handle(
    id: &'static str,
    side: PanelSide,
    active: bool,
    theme: &Theme,
) -> Stateful<Div> {
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
            theme.drag_border
        } else {
            transparent_black()
        });
    handle
        .id(id)
        .absolute()
        .top_0()
        .bottom_0()
        .cursor_col_resize()
        .child(edge)
}

pub struct WorkspaceShell {
    frame: PanelFrame,
    left: AnyElement,
    content: AnyElement,
    right: AnyElement,
    left_toggle: AnyElement,
    right_toggle: AnyElement,
    left_toggle_offset: f32,
    control_inset: f32,
    sidebar_divider_color: Option<Hsla>,
}

impl WorkspaceShell {
    pub fn new(
        frame: PanelFrame,
        left: AnyElement,
        content: AnyElement,
        right: AnyElement,
        left_toggle: AnyElement,
        right_toggle: AnyElement,
    ) -> Self {
        Self {
            frame,
            left,
            content,
            right,
            left_toggle,
            right_toggle,
            left_toggle_offset: 11.0,
            control_inset: 11.0,
            sidebar_divider_color: None,
        }
    }

    pub fn left_toggle_offset(mut self, offset: f32) -> Self {
        self.left_toggle_offset = offset;
        self
    }

    pub fn control_inset(mut self, inset: f32) -> Self {
        self.control_inset = inset;
        self
    }

    pub fn sidebar_divider_color(mut self, color: Hsla) -> Self {
        self.sidebar_divider_color = Some(color);
        self
    }

    pub fn into_element(self) -> Div {
        let frame = self.frame;
        let sidebar_divider_color = self.sidebar_divider_color;
        div()
            .size_full()
            .relative()
            .flex()
            .when(frame.left > 0.0, |root| {
                root.child(
                    div()
                        .h_full()
                        .flex_none()
                        .w(px(frame.left))
                        .when(frame.left_sliding, |panel| panel.overflow_hidden())
                        .child(
                            div()
                                .w(px(frame.left_content))
                                .h_full()
                                .flex_none()
                                .child(self.left),
                        ),
                )
            })
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_w(px(0.0))
                    .h_full()
                    .child(self.content)
                    .when(sidebar_divider_visible(frame.left), |content| {
                        content.when_some(sidebar_divider_color, |content, color| {
                            content.child(
                                div()
                                    .absolute()
                                    .top_0()
                                    .left_0()
                                    .bottom_0()
                                    .w(px(1.0))
                                    .bg(color),
                            )
                        })
                    })
                    .when(sidebar_divider_visible(frame.right), |content| {
                        content.when_some(sidebar_divider_color, |content, color| {
                            content.child(
                                div()
                                    .absolute()
                                    .top_0()
                                    .right_0()
                                    .bottom_0()
                                    .w(px(1.0))
                                    .bg(color),
                            )
                        })
                    }),
            )
            .when(frame.right > 0.0, |root| {
                root.child(
                    div()
                        .h_full()
                        .flex_none()
                        .w(px(frame.right))
                        .flex()
                        .relative()
                        .when(frame.right_sliding, |panel| panel.overflow_hidden())
                        .child(
                            div()
                                .absolute()
                                .top_0()
                                .right_0()
                                .h_full()
                                .w(px(frame.right_content))
                                .child(self.right),
                        ),
                )
            })
            .child(
                div()
                    .absolute()
                    .top(px(self.control_inset))
                    .left(px(self.left_toggle_offset))
                    .child(self.left_toggle),
            )
            .child(
                div()
                    .absolute()
                    .top(px(self.control_inset))
                    .right(px(right_toggle_offset(frame.right, self.control_inset)))
                    .child(self.right_toggle),
            )
    }
}

fn right_toggle_offset(right_panel_width: f32, control_inset: f32) -> f32 {
    right_panel_width + control_inset
}

fn sidebar_divider_visible(panel_width: f32) -> bool {
    panel_width >= 1.0
}

fn width_at(from: f32, target: f32, elapsed: Duration) -> Option<f32> {
    let progress = elapsed.as_secs_f32() / PANEL_SLIDE_DURATION.as_secs_f32();
    (progress < 1.0).then(|| from + (target - from) * ease_out_quint()(progress.max(0.0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: WorkspaceShellConfig = WorkspaceShellConfig {
        content_min_width: 360.0,
        left: PanelSize {
            min: 180.0,
            max: 420.0,
        },
        right: PanelSize {
            min: 280.0,
            max: 1000.0,
        },
    };

    #[test]
    fn panel_widths_preserve_the_content_minimum() {
        let (left, right) = fitted_panel_widths(CONFIG, 880.0, true, true, 420.0, 480.0);
        assert_eq!((left, right), (240.0, 280.0));
        assert_eq!(880.0 - left - right, CONFIG.content_min_width);
    }

    #[test]
    fn hidden_panels_do_not_consume_width() {
        assert_eq!(
            fitted_panel_widths(CONFIG, 880.0, false, true, 420.0, 480.0),
            (0.0, 480.0)
        );
    }

    #[test]
    fn panel_slide_eases_out_and_retires() {
        assert_eq!(width_at(0.0, 260.0, Duration::ZERO), Some(0.0));
        assert!(width_at(0.0, 260.0, PANEL_SLIDE_DURATION / 2).unwrap() > 130.0);
        assert_eq!(width_at(0.0, 260.0, PANEL_SLIDE_DURATION), None);
    }

    #[test]
    fn app_shell_mode_returns_to_the_existing_workspace() {
        let mut shell =
            AppShellState::new(WorkspaceShellState::new(CONFIG, true, false, 300.0, 520.0));

        shell.open_settings();
        assert!(shell.is_settings_open());
        shell.close_settings();

        assert_eq!(shell.mode(), AppShellMode::Workspace);
        assert_eq!(shell.workspace().snapshot().left_width, 300.0);
    }

    #[test]
    fn workspace_state_clamps_saved_widths_and_resizes_from_the_visible_frame() {
        let mut workspace = WorkspaceShellState::new(CONFIG, true, true, 800.0, 20.0);
        assert_eq!(workspace.snapshot().left_width, CONFIG.left.max);
        assert_eq!(workspace.snapshot().right_width, CONFIG.right.min);

        workspace.begin_resize(PanelSide::Left, 100.0);
        assert!(workspace.resize(40.0, 1200.0));
        assert_eq!(workspace.snapshot().left_width, 360.0);
        assert!(workspace.finish_resize());
        assert!(!workspace.finish_resize());
    }

    #[test]
    fn panel_window_resize_uses_the_visible_panel_delta() {
        let resize = PanelWindowResize {
            window_width: 1180.0,
            panel_width: 0.0,
            panel_content_width: 520.0,
            minimum_window_width: 980.0,
            enabled: true,
        };

        assert_eq!(resize.layout_width(), 1700.0);
        assert_eq!(resize.target_window_width(260.0), 1440.0);
        assert_eq!(resize.target_window_width(520.0), 1700.0);
        assert_eq!(resize.synced_panel_width(520.0, 1180.0, 1700.0), 0.0);
        assert_eq!(resize.synced_panel_width(520.0, 1440.0, 1700.0), 260.0);
        assert_eq!(resize.synced_panel_width(520.0, 1700.0, 1700.0), 520.0);
    }

    #[test]
    fn panel_window_resize_respects_the_window_minimum() {
        let resize = PanelWindowResize {
            window_width: 1180.0,
            panel_width: 520.0,
            panel_content_width: 520.0,
            minimum_window_width: 980.0,
            enabled: true,
        };

        assert_eq!(resize.target_window_width(0.0), 980.0);
        assert_eq!(resize.synced_panel_width(0.0, 1080.0, 980.0), 260.0);
        assert_eq!(resize.synced_panel_width(0.0, 980.0, 980.0), 0.0);
    }

    #[test]
    fn disabled_panel_window_resize_keeps_the_current_layout_width() {
        let resize = PanelWindowResize {
            window_width: 1440.0,
            panel_width: 0.0,
            panel_content_width: 520.0,
            minimum_window_width: 980.0,
            enabled: false,
        };

        assert_eq!(resize.layout_width(), 1440.0);
    }

    #[test]
    fn right_toggle_stays_on_the_content_edge_as_the_window_expands() {
        let collapsed_x = 1180.0 - right_toggle_offset(0.0, 11.0);
        let expanded_x = 1700.0 - right_toggle_offset(520.0, 11.0);

        assert_eq!(collapsed_x, expanded_x);
    }

    #[test]
    fn sidebar_divider_only_appears_with_a_visible_panel() {
        assert!(!sidebar_divider_visible(0.0));
        assert!(!sidebar_divider_visible(0.5));
        assert!(sidebar_divider_visible(1.0));
        assert!(sidebar_divider_visible(520.0));
    }
}
