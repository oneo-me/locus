rust_i18n::i18n!("locales", fallback = "en");

pub mod platform;

mod link_icon;
mod local_image;
mod markdown_extensions;

pub use gpui_kit::component::IconName as Icon;
pub use local_image::document_icon_image;
pub use markdown_extensions::{markdown_link_icon, note_markdown_extensions};
pub use oneo_gpui::toolbar_icon_button;

pub fn icon(icon: Icon, size: f32, color: gpui_kit::Hsla) -> gpui_kit::component::Icon {
    use gpui_kit::prelude::*;

    gpui_kit::component::Icon::new(icon)
        .size(gpui_kit::px(size))
        .text_color(color)
}

#[cfg(test)]
mod localization_tests;
