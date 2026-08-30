use gpui::{Hsla, Svg, prelude::*, px, svg};

#[derive(Clone, Copy)]
pub enum Icon {
    Appearance,
    Archive,
    ArrowLeft,
    ArrowRight,
    Check,
    ChevronDown,
    Copy,
    Edit,
    File,
    FolderOpen,
    ImageOff,
    Link,
    Loader,
    MoreHorizontal,
    PanelLeft,
    PanelRight,
    Plug,
    Settings,
    Terminal,
    Upload,
}

pub fn icon(icon: Icon, size: f32, color: Hsla) -> Svg {
    let data: &[u8] = match icon {
        Icon::Appearance => include_bytes!("../../assets/icons/appearance.svg"),
        Icon::Archive => include_bytes!("../../assets/icons/archive.svg"),
        Icon::ArrowLeft => include_bytes!("../../assets/icons/arrow-left.svg"),
        Icon::ArrowRight => include_bytes!("../../assets/icons/arrow-right.svg"),
        Icon::Check => include_bytes!("../../assets/icons/check.svg"),
        Icon::ChevronDown => include_bytes!("../../assets/icons/chevron-down.svg"),
        Icon::Copy => include_bytes!("../../assets/icons/copy.svg"),
        Icon::Edit => include_bytes!("../../assets/icons/edit.svg"),
        Icon::File => include_bytes!("../../assets/icons/file.svg"),
        Icon::FolderOpen => include_bytes!("../../assets/icons/folder-open.svg"),
        Icon::ImageOff => include_bytes!("../../assets/icons/image-off.svg"),
        Icon::Link => include_bytes!("../../assets/icons/link.svg"),
        Icon::Loader => include_bytes!("../../assets/icons/loader.svg"),
        Icon::MoreHorizontal => include_bytes!("../../assets/icons/more-horizontal.svg"),
        Icon::PanelLeft => include_bytes!("../../assets/icons/panel-left.svg"),
        Icon::PanelRight => include_bytes!("../../assets/icons/panel-right.svg"),
        Icon::Plug => include_bytes!("../../assets/icons/plug.svg"),
        Icon::Settings => include_bytes!("../../assets/icons/settings.svg"),
        Icon::Terminal => include_bytes!("../../assets/icons/terminal.svg"),
        Icon::Upload => include_bytes!("../../assets/icons/upload.svg"),
    };

    svg()
        .data(data)
        .w(px(size))
        .h(px(size))
        .flex_none()
        .text_color(color)
}
