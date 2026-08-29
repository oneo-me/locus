use gpui::{Hsla, Svg, prelude::*, px, svg};

#[derive(Clone, Copy)]
pub enum Icon {
    PanelLeft,
    PanelRight,
    Settings,
}

pub fn icon(icon: Icon, size: f32, color: Hsla) -> Svg {
    let data: &[u8] = match icon {
        Icon::PanelLeft => include_bytes!("../assets/icons/panel-left.svg"),
        Icon::PanelRight => include_bytes!("../assets/icons/panel-right.svg"),
        Icon::Settings => include_bytes!("../assets/icons/settings.svg"),
    };

    svg()
        .data(data)
        .w(px(size))
        .h(px(size))
        .flex_none()
        .text_color(color)
}
