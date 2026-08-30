use std::time::{Duration, Instant};

use crate::{Icon, Theme, toolbar_icon_button};
use gpui::{AnyElement, Div, FocusHandle, Stateful, div, ease_out_quint, prelude::*, px};

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

    pub fn into_element(self) -> Div {
        let frame = self.frame;
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
            .child(div().flex_1().min_w(px(0.0)).h_full().child(self.content))
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
                    .right(px(self.control_inset))
                    .child(self.right_toggle),
            )
    }
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
}
