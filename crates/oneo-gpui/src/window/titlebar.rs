use gpui::{Div, ElementId, MouseButton, Stateful, WindowControlArea, div, prelude::*};

pub fn titlebar_drag_region(id: impl Into<ElementId>) -> Stateful<Div> {
    div()
        .id(id)
        .h_full()
        .window_control_area(WindowControlArea::Drag)
        .on_mouse_down(MouseButton::Left, |_, window, _| window.start_window_move())
        .on_click(|event, window, _| {
            if event.click_count() == 2 {
                window.titlebar_double_click();
            }
        })
}

pub const TITLEBAR_HEIGHT: f32 = 48.0;
pub const TITLEBAR_CONTROL_INSET: f32 = 11.0;
pub const TITLEBAR_CONTROL_SIZE: f32 = 26.0;
pub const MACOS_TRAFFIC_LIGHT_CLEARANCE: f32 = 86.0;

pub fn titlebar_left_control_offset(fullscreen: bool) -> f32 {
    if cfg!(target_os = "macos") && !fullscreen {
        MACOS_TRAFFIC_LIGHT_CLEARANCE
    } else {
        TITLEBAR_CONTROL_INSET
    }
}

pub fn titlebar_content_left_padding(
    control_clearance: f32,
    rendered_left_width: f32,
    content_inset: f32,
) -> f32 {
    (control_clearance - rendered_left_width).max(content_inset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_title_padding_moves_continuously_with_the_left_panel() {
        assert_eq!(titlebar_content_left_padding(124.0, 252.0, 20.0), 20.0);
        assert_eq!(titlebar_content_left_padding(124.0, 50.0, 20.0), 74.0);
        assert_eq!(titlebar_content_left_padding(124.0, 0.0, 20.0), 124.0);
    }
}
