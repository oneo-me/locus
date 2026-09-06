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
