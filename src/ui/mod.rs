mod link_icon;
mod local_image;
mod markdown_extensions;

pub use gpui_component::IconName as Icon;
pub use local_image::document_icon_image;
pub use markdown_extensions::LocusMarkdownExtensions;
pub use oneo_gpui_app_shell::toolbar_icon_button;
pub use oneo_gpui_markdown::{
    Markdown, MarkdownCodeBlocks, MarkdownDocument, MarkdownLabels, MarkdownPresentation,
    MarkdownSelection, MarkdownTypography, SelectableCodeStyle, selectable_code_text,
};

pub fn icon(icon: Icon, size: f32, color: gpui::Hsla) -> gpui_component::Icon {
    use gpui::prelude::*;

    gpui_component::Icon::new(icon)
        .size(gpui::px(size))
        .text_color(color)
}
