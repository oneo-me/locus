use crate::motion::WidthTween;
use crate::state::{
    LEFT_SIDEBAR_MAX_WIDTH, LEFT_SIDEBAR_MIN_WIDTH, RIGHT_SIDEBAR_MAX_WIDTH,
    RIGHT_SIDEBAR_MIN_WIDTH,
};

pub(super) const CONTENT_MIN_WIDTH: f32 = 360.0;

#[derive(Clone, Copy)]
pub(super) struct PanelFrame {
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

pub(super) fn slide_width(slide: &mut Option<WidthTween>, target: f32) -> f32 {
    match slide.and_then(|slide| slide.width_toward(target)) {
        Some(width) => width,
        None => {
            *slide = None;
            target
        }
    }
}

pub(super) fn fitted_panel_widths(
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
