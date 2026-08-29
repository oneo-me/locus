use std::time::{Duration, Instant};

use gpui::ease_out_quint;

const PANEL_SLIDE: Duration = Duration::from_millis(200);

#[derive(Clone, Copy)]
pub struct WidthTween {
    from: f32,
    started: Instant,
}

impl WidthTween {
    pub fn new(from: f32) -> Self {
        Self {
            from,
            started: Instant::now(),
        }
    }

    pub fn width_toward(&self, target: f32) -> Option<f32> {
        width_at(self.from, target, self.started.elapsed())
    }
}

fn width_at(from: f32, target: f32, elapsed: Duration) -> Option<f32> {
    let progress = elapsed.as_secs_f32() / PANEL_SLIDE.as_secs_f32();
    (progress < 1.0).then(|| from + (target - from) * ease_out_quint()(progress.max(0.0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_tween_eases_out_and_retires() {
        assert_eq!(width_at(0.0, 260.0, Duration::ZERO), Some(0.0));
        assert!(width_at(0.0, 260.0, PANEL_SLIDE / 2).unwrap() > 130.0);
        assert_eq!(width_at(0.0, 260.0, PANEL_SLIDE), None);
    }
}
