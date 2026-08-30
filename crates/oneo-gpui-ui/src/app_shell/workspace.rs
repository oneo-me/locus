use std::time::{Duration, Instant};

use crate::{Icon, Theme, toolbar_icon_button};
use gpui::{
    AnyElement, Div, FocusHandle, Hsla, Stateful, Window, div, ease_out_quint, prelude::*, px,
};

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
pub struct PanelSlide {
    from: f32,
    started: Instant,
}

#[derive(Clone, Copy, Debug)]
pub struct PanelWindowResize {
    window_width: f32,
    panel_width: f32,
    panel_content_width: f32,
    minimum_window_width: f32,
    enabled: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct PanelWindowFrame {
    pub panel_width: f32,
    pub window_resize_complete: bool,
}

impl PanelWindowResize {
    pub fn new(
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

    pub fn panel_content_width(self) -> f32 {
        self.panel_content_width
    }

    pub fn layout_width(self) -> f32 {
        if self.enabled {
            self.window_width + self.panel_content_width - self.panel_width
        } else {
            self.window_width
        }
    }

    pub fn sync(self, panel_width: f32, window: &mut Window) -> PanelWindowFrame {
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
    pub fn new(from: f32) -> Self {
        Self {
            from,
            started: Instant::now(),
        }
    }

    fn width_toward(self, target: f32) -> Option<f32> {
        width_at(self.from, target, self.started.elapsed())
    }
}

pub fn slide_width(slide: &mut Option<PanelSlide>, target: f32) -> f32 {
    match slide.and_then(|slide| slide.width_toward(target)) {
        Some(width) => width,
        None => {
            *slide = None;
            target
        }
    }
}

pub fn fitted_panel_widths(
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
    label: &'static str,
    focus: &FocusHandle,
    side: PanelSide,
    theme: Theme,
) -> Stateful<Div> {
    let icon = match side {
        PanelSide::Left => Icon::PanelLeft,
        PanelSide::Right => Icon::PanelRight,
    };
    toolbar_icon_button(id, label, focus, icon, theme)
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
    right_divider_color: Option<Hsla>,
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
            right_divider_color: None,
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

    pub fn right_divider_color(mut self, color: Hsla) -> Self {
        self.right_divider_color = Some(color);
        self
    }

    pub fn into_element(self) -> Div {
        let frame = self.frame;
        let right_divider_color = self.right_divider_color;
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
                    .when(right_divider_visible(frame.right), |content| {
                        content.when_some(right_divider_color, |content, color| {
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

fn right_divider_visible(right_panel_width: f32) -> bool {
    right_panel_width > 0.0
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
    fn right_divider_only_appears_with_the_right_panel() {
        assert!(!right_divider_visible(0.0));
        assert!(right_divider_visible(0.5));
        assert!(right_divider_visible(520.0));
    }
}
