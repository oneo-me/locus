use gpui::{Hsla, Svg, prelude::*, px, svg};

#[derive(Clone, Copy)]
pub enum Icon {
    Appearance,
    ArrowLeft,
    Check,
    ChevronDown,
    Copy,
    FolderOpen,
    PanelLeft,
    PanelRight,
    Settings,
}

pub fn icon(icon: Icon, size: f32, color: Hsla) -> Svg {
    let data: &[u8] = match icon {
        Icon::Appearance => include_bytes!("../../assets/icons/appearance.svg"),
        Icon::ArrowLeft => include_bytes!("../../assets/icons/arrow-left.svg"),
        Icon::Check => include_bytes!("../../assets/icons/check.svg"),
        Icon::ChevronDown => include_bytes!("../../assets/icons/chevron-down.svg"),
        Icon::Copy => include_bytes!("../../assets/icons/copy.svg"),
        Icon::FolderOpen => include_bytes!("../../assets/icons/folder-open.svg"),
        Icon::PanelLeft => include_bytes!("../../assets/icons/panel-left.svg"),
        Icon::PanelRight => include_bytes!("../../assets/icons/panel-right.svg"),
        Icon::Settings => include_bytes!("../../assets/icons/settings.svg"),
    };

    svg()
        .data(data)
        .w(px(size))
        .h(px(size))
        .flex_none()
        .text_color(color)
}
