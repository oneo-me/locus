mod display;

use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

use gpui_kit::component::{
    ActiveTheme,
    button::{Button, ButtonVariants as _},
};
use gpui_kit::profiler::{FrameEvent, FrameTimingCollector};
use gpui_kit::{
    App, Context, IntoElement, Render, RenderOnce, Task, Window, WindowId, div, prelude::*, rems,
};
use rust_i18n::t;

const SAMPLE_WINDOW: Duration = Duration::from_secs(1);
const UPDATE_INTERVAL: Duration = Duration::from_millis(250);
const DRAW_SAMPLE_CAPACITY: usize = 120;

#[derive(IntoElement)]
pub(crate) struct SidebarFps;

impl RenderOnce for SidebarFps {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        window.use_keyed_state("sidebar-fps", cx, |window, cx| FpsCounter::new(window, cx))
    }
}

struct FrameRate {
    window_id: WindowId,
    presented: VecDeque<Instant>,
    draws: VecDeque<Duration>,
}

impl FrameRate {
    fn sample(&mut self, events: impl IntoIterator<Item = FrameEvent>, now: Instant) -> usize {
        for event in events {
            match event {
                FrameEvent::Present(timing) if timing.window_id == self.window_id => {
                    self.presented.push_back(timing.present_end);
                }
                FrameEvent::Draw(timing) if timing.window_id == self.window_id => {
                    if self.draws.len() == DRAW_SAMPLE_CAPACITY {
                        self.draws.pop_front();
                    }
                    self.draws.push_back(timing.draw_duration());
                }
                _ => {}
            }
        }
        self.presented
            .retain(|time| now.saturating_duration_since(*time) < SAMPLE_WINDOW);
        self.presented.len()
    }

    fn max_fps(&self, refresh_rate: Option<f64>) -> usize {
        let total: Duration = self.draws.iter().copied().sum();
        if total.is_zero() {
            return 0;
        }
        let estimated = self.draws.len() as f64 / total.as_secs_f64();
        match refresh_rate.filter(|rate| rate.is_finite() && *rate > 1.) {
            Some(limit) => estimated.min(limit).round() as usize,
            None => estimated.round() as usize,
        }
    }
}

struct FpsCounter {
    fps: usize,
    max_fps: usize,
    show_max: bool,
    _timer: Task<()>,
}

impl FpsCounter {
    fn new(window: &Window, cx: &mut Context<Self>) -> Self {
        let window_handle = window.window_handle();
        let mut collector = FrameTimingCollector::new();
        let mut rate = FrameRate {
            window_id: window.window_handle().window_id(),
            presented: VecDeque::new(),
            draws: VecDeque::new(),
        };
        let timer = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(UPDATE_INTERVAL).await;
                let fps = rate.sample(collector.collect_unseen(), Instant::now());
                let Ok(refresh_rate) = window_handle.update(cx, |_, window, cx| {
                    window
                        .display(cx)
                        .and_then(|display| display::refresh_rate(display.as_ref()))
                }) else {
                    break;
                };
                let max_fps = rate.max_fps(refresh_rate);
                if this
                    .update(cx, |this, cx| {
                        if this.fps != fps || this.max_fps != max_fps {
                            this.fps = fps;
                            this.max_fps = max_fps;
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        Self {
            fps: 0,
            max_fps: 0,
            show_max: true,
            _timer: timer,
        }
    }

    fn displayed_fps(&self) -> usize {
        if self.show_max {
            self.max_fps
        } else {
            self.fps
        }
    }
}

impl Render for FpsCounter {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let locale = gpui_kit::component::locale();
        let description = if self.show_max {
            t!("fps.maximum", locale = &*locale, value = self.max_fps)
        } else {
            t!("fps.current", locale = &*locale, value = self.fps)
        }
        .into_owned();
        div()
            .debug_selector(|| "sidebar-fps".to_owned())
            .flex_none()
            .flex()
            .self_center()
            .rounded(cx.theme().radius_lg)
            .border_1()
            .border_color(cx.theme().sidebar_foreground.alpha(0.1))
            .child(
                Button::new("sidebar-fps-readout")
                    .ghost()
                    .occlude()
                    .cursor_default()
                    .accessibility_label(description.clone())
                    .tooltip(description)
                    .p_1()
                    .h_full()
                    .child(
                        div()
                            .text_size(rems(0.625))
                            .line_height(gpui_kit::relative(1.))
                            .font_weight(gpui_kit::FontWeight::NORMAL)
                            .text_color(cx.theme().muted_foreground)
                            .child(self.displayed_fps().to_string()),
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.show_max = !this.show_max;
                        cx.stop_propagation();
                        cx.notify();
                    })),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::profiler::PresentTiming;

    #[gpui_kit::test]
    fn defaults_to_max_and_toggles_without_titlebar_gestures(cx: &mut gpui_kit::TestAppContext) {
        use gpui_kit::{Entity, MouseButton, div};

        struct Header {
            counter: Entity<FpsCounter>,
            presses: usize,
            clicks: usize,
        }
        impl Render for Header {
            fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
                div()
                    .id("fps-header")
                    .flex()
                    .items_center()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, _| this.presses += 1),
                    )
                    .on_click(cx.listener(|this, _, _, _| this.clicks += 1))
                    .child(self.counter.clone())
            }
        }

        cx.update(gpui_kit::init);
        let (header, cx) = cx.add_window_view(|window, cx| {
            let counter = cx.new(|cx| FpsCounter::new(window, cx));
            counter.update(cx, |counter, _| {
                counter.fps = 24;
                counter.max_fps = 60;
            });
            Header {
                counter,
                presses: 0,
                clicks: 0,
            }
        });
        let counter = header.read_with(cx, |header, _| header.counter.clone());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(
            counter.read_with(cx, |counter, _| counter.displayed_fps()),
            60
        );
        let bounds = cx.debug_bounds("sidebar-fps").unwrap();
        cx.simulate_click(bounds.center(), Default::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(
            counter.read_with(cx, |counter, _| counter.displayed_fps()),
            24
        );
        cx.simulate_click(bounds.center(), Default::default());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(
            counter.read_with(cx, |counter, _| counter.displayed_fps()),
            60
        );
        assert_eq!(
            header.read_with(cx, |header, _| (header.presses, header.clicks)),
            (0, 0)
        );
        cx.update(|window, cx| window.focus_next(cx));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        for (key, expected) in [("space", 24), ("enter", 60)] {
            let keystroke = gpui_kit::Keystroke::parse(key).unwrap();
            cx.simulate_event(gpui_kit::KeyDownEvent {
                keystroke: keystroke.clone(),
                is_held: false,
                prefer_character_input: false,
            });
            cx.simulate_event(gpui_kit::KeyUpEvent { keystroke });
            cx.update(|window, cx| window.draw(cx).clear(cx));
            assert_eq!(
                counter.read_with(cx, |counter, _| counter.displayed_fps()),
                expected
            );
        }
    }

    fn present(window_id: WindowId, time: Instant) -> FrameEvent {
        FrameEvent::Present(PresentTiming {
            window_id,
            present_start: time,
            present_end: time,
            animation_interval: None,
        })
    }

    fn draw(window_id: WindowId, time: Instant, cost: Duration) -> FrameEvent {
        FrameEvent::Draw(gpui_kit::profiler::FrameTiming {
            window_id,
            dirty_at: None,
            invalidations: 1,
            draw_start: time,
            draw_end: time + cost,
        })
    }

    #[test]
    fn max_uses_mean_draw_cost_and_replaces_old_samples() {
        let window_id = WindowId::from(1);
        let now = Instant::now();
        let mut rate = FrameRate {
            window_id,
            presented: VecDeque::new(),
            draws: VecDeque::new(),
        };
        assert_eq!(rate.max_fps(None), 0);
        rate.sample([draw(window_id, now, Duration::ZERO)], now);
        assert_eq!(rate.max_fps(None), 0);
        rate.draws.clear();
        rate.sample(
            [
                draw(window_id, now, Duration::from_millis(2)),
                draw(window_id, now, Duration::from_millis(6)),
                draw(WindowId::from(2), now, Duration::from_millis(100)),
                present(window_id, now),
            ],
            now,
        );
        assert_eq!(rate.max_fps(None), 250);
        assert_eq!(rate.max_fps(Some(60.)), 60);
        assert_eq!(rate.max_fps(Some(144.)), 144);
        assert_eq!(rate.max_fps(Some(59.94)), 60);
        assert_eq!(rate.max_fps(Some(0.)), 250);
        assert_eq!(rate.max_fps(Some(1.)), 250);
        assert_eq!(rate.max_fps(Some(f64::NAN)), 250);
        rate.sample(
            (0..DRAW_SAMPLE_CAPACITY).map(|_| draw(window_id, now, Duration::from_millis(10))),
            now,
        );
        assert_eq!(rate.max_fps(None), 100);
        assert_eq!(rate.max_fps(Some(144.)), 100);
        rate.sample([], now + Duration::from_secs(2));
        assert_eq!(rate.max_fps(None), 100);
        assert_eq!(rate.max_fps(Some(144.)), 100);
    }

    #[test]
    fn counts_presented_frames_in_the_last_second_for_its_window() {
        let window_id = WindowId::from(1);
        let other_window = WindowId::from(2);
        let start = Instant::now();
        let mut rate = FrameRate {
            window_id,
            presented: VecDeque::new(),
            draws: VecDeque::new(),
        };
        assert_eq!(rate.sample([], start), 0);
        let events = (1..=60).flat_map(|frame| {
            let time = start + Duration::from_secs_f64(frame as f64 / 60.0);
            [present(window_id, time), present(other_window, time)]
        });
        assert_eq!(rate.sample(events, start + SAMPLE_WINDOW), 60);
        assert_eq!(rate.sample([], start + SAMPLE_WINDOW), 60);
        assert_eq!(rate.sample([], start + Duration::from_millis(1500)), 30);
        assert_eq!(rate.sample([], start + Duration::from_secs(2)), 0);
    }

    #[test]
    fn expires_delayed_batches_using_presentation_time() {
        let window_id = WindowId::from(1);
        let start = Instant::now();
        let mut rate = FrameRate {
            window_id,
            presented: VecDeque::new(),
            draws: VecDeque::new(),
        };
        assert_eq!(
            rate.sample([present(window_id, start)], start + SAMPLE_WINDOW),
            0
        );
    }
}
