mod attachment_views;
mod extension_syntax;
mod plugins;
mod thumbnail;

pub use plugins::note_markdown_extensions;

use crate::{document_icon_image, link_icon::remote_link_icon};
use gpui_kit::component::{Icon, IconName, Theme};
use gpui_kit::{AnyElement, ObjectFit, StyledImage, div, prelude::*, rems};
use locus_library::{LIBRARY_REFERENCE_SCHEME, LibraryTree, library_reference_path};

pub fn markdown_link_icon(url: &str, library_tree: &LibraryTree, theme: &Theme) -> AnyElement {
    if let Some(path) = library_reference_path(url) {
        let Some(node) = library_tree.node(&path) else {
            return generic_link_icon(theme);
        };
        if let Some(source) = node.icon_image.as_ref() {
            let fallback_icon = node.icon.clone();
            let fallback_theme = theme.clone();
            return document_icon_image(source)
                .size(rems(0.8125))
                .rounded(rems(0.1875))
                .object_fit(ObjectFit::Cover)
                .with_fallback(move || {
                    fallback_icon
                        .clone()
                        .map(emoji_link_icon)
                        .unwrap_or_else(|| generic_link_icon(&fallback_theme))
                })
                .into_any_element();
        }
        return node
            .icon
            .clone()
            .map(emoji_link_icon)
            .unwrap_or_else(|| generic_link_icon(theme));
    }
    if url.starts_with("http://") || url.starts_with("https://") {
        let loading_theme = theme.clone();
        let fallback_theme = theme.clone();
        return remote_link_icon(url)
            .size(rems(0.8125))
            .rounded(rems(0.1875))
            .object_fit(ObjectFit::Cover)
            .with_loading(move || generic_link_icon(&loading_theme))
            .with_fallback(move || generic_link_icon(&fallback_theme))
            .into_any_element();
    }
    if url.starts_with(LIBRARY_REFERENCE_SCHEME) {
        return generic_link_icon(theme);
    }
    generic_link_icon(theme)
}

fn emoji_link_icon(value: String) -> AnyElement {
    div()
        .size(rems(0.8125))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .text_size(rems(0.75))
        .line_height(rems(0.8125))
        .child(value)
        .into_any_element()
}

fn generic_link_icon(theme: &Theme) -> AnyElement {
    Icon::new(IconName::ExternalLink)
        .size(rems(0.75))
        .text_color(theme.muted_foreground)
        .into_any_element()
}

#[cfg(test)]
mod tests;
