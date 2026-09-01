use std::time::{Duration, Instant};

use gpui::{Context, Render, SharedString, Window, div, prelude::*, px};
use gpui_component::Theme;

pub struct FpsCounter {
    visible: bool,
    frame_count: u32,
    value: u32,
    last_frame: Instant,
    label: SharedString,
}

impl FpsCounter {
    pub fn new(visible: bool, label: impl Into<SharedString>) -> Self {
        Self {
            visible,
            frame_count: 0,
            value: 0,
            last_frame: Instant::now(),
            label: label.into(),
        }
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn toggle(&mut self, cx: &mut Context<Self>) {
        self.visible = !self.visible;
        self.frame_count = 0;
        self.value = 0;
        self.last_frame = Instant::now();
        cx.notify();
    }

    pub fn set_label(&mut self, label: impl Into<SharedString>, cx: &mut Context<Self>) {
        let label = label.into();
        if self.label != label {
            self.label = label;
            cx.notify();
        }
    }
}

impl Render for FpsCounter {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.visible {
            return div();
        }

        let now = Instant::now();
        self.frame_count = self.frame_count.saturating_add(1);
        let elapsed = now.duration_since(self.last_frame);
        if elapsed >= Duration::from_secs(1) {
            self.value = (self.frame_count as f32 / elapsed.as_secs_f32()).round() as u32;
            self.frame_count = 0;
            self.last_frame = now;
        }
        window.request_animation_frame();
        let theme = Theme::global(cx);
        div()
            .h(px(26.0))
            .flex_none()
            .px(px(6.0))
            .flex()
            .items_center()
            .gap(px(5.0))
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(div().size(px(6.0)).rounded_full().bg(if self.value > 0 {
                theme.success
            } else {
                theme.muted_foreground
            }))
            .child(format!("{} {}", self.value, self.label))
    }
}
