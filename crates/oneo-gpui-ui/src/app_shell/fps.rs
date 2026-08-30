use std::time::{Duration, Instant};

use crate::Theme;
use gpui::{Div, Window, div, prelude::*, px};

pub struct FpsCounterState {
    visible: bool,
    frame_count: u32,
    value: u32,
    last_frame: Instant,
}

impl FpsCounterState {
    pub fn new(visible: bool) -> Self {
        Self {
            visible,
            frame_count: 0,
            value: 0,
            last_frame: Instant::now(),
        }
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        self.frame_count = 0;
        self.value = 0;
        self.last_frame = Instant::now();
    }

    pub fn tick(&mut self, window: &Window) -> bool {
        if !self.visible {
            return false;
        }
        let now = Instant::now();
        self.frame_count = self.frame_count.saturating_add(1);
        let elapsed = now.duration_since(self.last_frame);
        let mut changed = false;
        if elapsed >= Duration::from_secs(1) {
            self.value = (self.frame_count as f32 / elapsed.as_secs_f32()).round() as u32;
            self.frame_count = 0;
            self.last_frame = now;
            changed = true;
        }
        window.request_animation_frame();
        changed
    }
}

pub fn fps_counter(state: &FpsCounterState, theme: Theme) -> Div {
    div()
        .h(px(26.0))
        .flex_none()
        .px(px(6.0))
        .flex()
        .items_center()
        .gap(px(5.0))
        .text_xs()
        .text_color(theme.text_tertiary)
        .child(div().size(px(6.0)).rounded_full().bg(if state.value > 0 {
            theme.focus
        } else {
            theme.text_tertiary
        }))
        .child(format!("{} FPS", state.value))
}
