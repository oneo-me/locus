use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{
    App, BorderStyle, Bounds, DispatchPhase, EntityId, IntoElement, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, Pixels, Point, ScrollHandle, Styled, Window, canvas, point, px,
    quad, size, transparent_black,
};

use crate::theme::Theme;

const TRACK_WIDTH: f32 = 11.0;
const TRACK_INSET: f32 = 2.0;
const TRACK_VERTICAL_INSET: f32 = 4.0;
const THUMB_WIDTH: f32 = 7.0;
const MIN_THUMB_HEIGHT: f32 = 28.0;
const HOLD_DURATION: Duration = Duration::from_millis(900);
const FADE_DURATION: Duration = Duration::from_millis(350);

#[derive(Default)]
pub struct ScrollbarState {
    grab_offset: Cell<Option<f32>>,
    hovered: Cell<bool>,
    hover_holding: Cell<bool>,
    last_scroll: Cell<Option<Instant>>,
    last_offset: Cell<Option<Pixels>>,
    fade_wake_armed: Cell<bool>,
}

impl ScrollbarState {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    pub fn reset_visibility(&self) {
        self.grab_offset.set(None);
        self.hover_holding.set(false);
        self.last_scroll.set(None);
        self.last_offset.set(None);
    }

    fn observe(&self, offset: Pixels, now: Instant) -> bool {
        let moved = self
            .last_offset
            .replace(Some(offset))
            .is_some_and(|previous| (offset - previous).abs() > px(0.5));
        if moved {
            self.last_scroll.set(Some(now));
            if self.hovered.get() {
                self.hover_holding.set(true);
            }
        }
        moved
    }
}

#[derive(Clone, Copy)]
struct Geometry {
    thumb: Bounds<Pixels>,
    travel: Pixels,
    maximum_offset: Pixels,
}

fn geometry(
    track: Bounds<Pixels>,
    viewport_height: Pixels,
    maximum_offset: Pixels,
    offset: Pixels,
    thumb_width: Pixels,
) -> Option<Geometry> {
    if viewport_height <= Pixels::ZERO
        || maximum_offset <= Pixels::ZERO
        || track.size.height <= Pixels::ZERO
    {
        return None;
    }

    let content_height = viewport_height + maximum_offset;
    let thumb_height = (track.size.height * (viewport_height / content_height))
        .max(px(MIN_THUMB_HEIGHT))
        .min(track.size.height);
    let travel = (track.size.height - thumb_height).max(Pixels::ZERO);
    let progress = (offset / maximum_offset).clamp(0.0, 1.0);
    Some(Geometry {
        thumb: Bounds::new(
            point(
                track.right() - thumb_width - px(TRACK_INSET),
                track.top() + travel * progress,
            ),
            size(thumb_width, thumb_height),
        ),
        travel,
        maximum_offset,
    })
}

fn offset_for_thumb_top(track_top: Pixels, thumb_top: Pixels, geometry: &Geometry) -> Pixels {
    if geometry.travel <= Pixels::ZERO {
        return Pixels::ZERO;
    }
    let progress = ((thumb_top - track_top) / geometry.travel).clamp(0.0, 1.0);
    geometry.maximum_offset * progress
}

fn scroll_to(handle: &ScrollHandle, offset: Pixels, maximum_offset: Pixels) {
    let x = handle.offset().x;
    handle.set_offset(Point::new(x, -offset.clamp(Pixels::ZERO, maximum_offset)));
}

fn opacity(since_scroll: Option<Duration>, hover_holding: bool, grabbed: bool) -> f32 {
    if hover_holding || grabbed {
        return 1.0;
    }
    let Some(elapsed) = since_scroll else {
        return 0.0;
    };
    if elapsed < HOLD_DURATION {
        return 1.0;
    }
    (1.0 - (elapsed - HOLD_DURATION).as_secs_f32() / FADE_DURATION.as_secs_f32()).clamp(0.0, 1.0)
}

fn arm_fade_wake(state: &Rc<ScrollbarState>, view: EntityId, delay: Duration, cx: &mut App) {
    if state.fade_wake_armed.replace(true) {
        return;
    }
    let state = state.clone();
    cx.spawn(async move |cx| {
        cx.background_executor()
            .timer(delay + Duration::from_millis(16))
            .await;
        state.fade_wake_armed.set(false);
        cx.update(|cx| cx.notify(view));
    })
    .detach();
}

pub fn vertical_scrollbar(
    handle: &ScrollHandle,
    state: &Rc<ScrollbarState>,
    top_inset: f32,
) -> impl IntoElement {
    let handle = handle.clone();
    let state = state.clone();
    canvas(
        |_, _, _| (),
        move |track: Bounds<Pixels>, _, window: &mut Window, cx: &mut App| {
            let now = Instant::now();
            let offset = -handle.offset().y;
            state.observe(offset, now);
            let active = state.hover_holding.get() || state.grab_offset.get().is_some();
            let thumb_width = px(THUMB_WIDTH);
            let Some(geometry) = geometry(
                track,
                handle.bounds().size.height,
                handle.max_offset().y,
                offset,
                thumb_width,
            ) else {
                state.grab_offset.set(None);
                state.hovered.set(false);
                state.hover_holding.set(false);
                return;
            };

            let theme = Theme::current(cx);
            let since_scroll = state
                .last_scroll
                .get()
                .map(|last| now.saturating_duration_since(last));
            let thumb_opacity = opacity(
                since_scroll,
                state.hover_holding.get(),
                state.grab_offset.get().is_some(),
            );
            if thumb_opacity > 0.0 {
                window.paint_quad(quad(
                    geometry.thumb,
                    thumb_width / 2.0,
                    theme
                        .text_tertiary
                        .opacity(if active { 0.7 } else { 0.45 })
                        .opacity(thumb_opacity),
                    Pixels::ZERO,
                    transparent_black(),
                    BorderStyle::default(),
                ));
                if !active {
                    match since_scroll {
                        Some(elapsed) if elapsed < HOLD_DURATION => arm_fade_wake(
                            &state,
                            window.current_view(),
                            HOLD_DURATION - elapsed,
                            cx,
                        ),
                        Some(elapsed) if elapsed < HOLD_DURATION + FADE_DURATION => {
                            window.request_animation_frame();
                        }
                        _ => {}
                    }
                }
            }

            window.on_mouse_event({
                let state = state.clone();
                move |event: &MouseMoveEvent, phase, window, _| {
                    if phase == DispatchPhase::Bubble {
                        let hovered = track.contains(&event.position);
                        if state.hovered.replace(hovered) != hovered {
                            let holding = hovered
                                && state.last_scroll.get().is_some_and(|last| {
                                    opacity(
                                        Some(Instant::now().saturating_duration_since(last)),
                                        false,
                                        false,
                                    ) > 0.0
                                });
                            state.hover_holding.set(holding);
                            window.refresh();
                        }
                    }
                }
            });

            window.on_mouse_event({
                let handle = handle.clone();
                let state = state.clone();
                move |event: &MouseDownEvent, phase, window, cx| {
                    if phase != DispatchPhase::Bubble
                        || event.button != MouseButton::Left
                        || !track.contains(&event.position)
                        || thumb_opacity <= 0.0
                    {
                        return;
                    }
                    if geometry.thumb.contains(&event.position) {
                        state
                            .grab_offset
                            .set(Some(f32::from(event.position.y - geometry.thumb.top())));
                    } else {
                        let half_thumb = geometry.thumb.size.height / 2.0;
                        state.grab_offset.set(Some(f32::from(half_thumb)));
                        scroll_to(
                            &handle,
                            offset_for_thumb_top(
                                track.top(),
                                event.position.y - half_thumb,
                                &geometry,
                            ),
                            geometry.maximum_offset,
                        );
                    }
                    cx.stop_propagation();
                    window.refresh();
                }
            });

            window.on_mouse_event({
                let handle = handle.clone();
                let state = state.clone();
                move |event: &MouseMoveEvent, phase, window, _| {
                    if phase != DispatchPhase::Bubble {
                        return;
                    }
                    let Some(grab_offset) = state.grab_offset.get() else {
                        return;
                    };
                    scroll_to(
                        &handle,
                        offset_for_thumb_top(
                            track.top(),
                            event.position.y - px(grab_offset),
                            &geometry,
                        ),
                        geometry.maximum_offset,
                    );
                    window.refresh();
                }
            });

            window.on_mouse_event({
                let state = state.clone();
                move |event: &MouseUpEvent, phase, window, _| {
                    if phase == DispatchPhase::Bubble
                        && event.button == MouseButton::Left
                        && state.grab_offset.take().is_some()
                    {
                        window.refresh();
                    }
                }
            });
        },
    )
    .absolute()
    .top(px(TRACK_VERTICAL_INSET + top_inset))
    .bottom(px(TRACK_VERTICAL_INSET))
    .right_0()
    .w(px(TRACK_WIDTH))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track() -> Bounds<Pixels> {
        Bounds::new(point(px(500.0), px(100.0)), size(px(11.0), px(400.0)))
    }

    #[test]
    fn hidden_when_content_does_not_overflow() {
        assert!(geometry(track(), px(400.0), Pixels::ZERO, Pixels::ZERO, px(7.0)).is_none());
    }

    #[test]
    fn hidden_at_rest_then_held_and_faded_after_scrolling() {
        assert_eq!(opacity(None, false, false), 0.0);
        assert_eq!(opacity(Some(Duration::ZERO), false, false), 1.0);
        assert_eq!(
            opacity(Some(HOLD_DURATION - Duration::from_millis(1)), false, false),
            1.0
        );
        let halfway = opacity(Some(HOLD_DURATION + FADE_DURATION / 2), false, false);
        assert!((0.4..0.6).contains(&halfway));
        assert_eq!(
            opacity(Some(HOLD_DURATION + FADE_DURATION), false, false),
            0.0
        );
    }

    #[test]
    fn hover_and_drag_keep_the_thumb_visible() {
        let elapsed = Some(HOLD_DURATION + FADE_DURATION);
        assert_eq!(opacity(elapsed, true, false), 1.0);
        assert_eq!(opacity(elapsed, false, true), 1.0);
    }

    #[test]
    fn first_offset_observation_does_not_reveal_the_thumb() {
        let state = ScrollbarState::default();
        let now = Instant::now();
        assert!(!state.observe(px(800.0), now));
        assert_eq!(state.last_scroll.get(), None);
        assert!(state.observe(px(801.0), now));
        assert_eq!(state.last_scroll.get(), Some(now));
    }

    #[test]
    fn thumb_tracks_visible_fraction_and_offset() {
        let geometry = geometry(track(), px(400.0), px(1200.0), px(600.0), px(7.0)).unwrap();
        assert_eq!(geometry.thumb.size.height, px(100.0));
        assert_eq!(geometry.thumb.top(), px(250.0));
        assert_eq!(
            offset_for_thumb_top(track().top(), geometry.thumb.top(), &geometry),
            px(600.0)
        );
    }
}
