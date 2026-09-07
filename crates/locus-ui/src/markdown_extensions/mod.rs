mod attachment_views;
mod extension_syntax;
mod thumbnail;

use gpui_kit::{AnyElement, ObjectFit, StyledImage, div, prelude::*, px};
use oneo_gpui_markdown::{MarkdownExtensions, generic_link_icon};

use crate::{document_icon_image, link_icon::remote_link_icon};
use gpui_kit::component::Theme;
use locus_library::{Attachment, LIBRARY_REFERENCE_SCHEME, LibraryTree, library_reference_path};
use oneo_gpui::Language;

pub struct LocusMarkdownExtensions<'a> {
    attachments: &'a [Attachment],
    library_tree: &'a LibraryTree,
    language: Language,
}

impl<'a> LocusMarkdownExtensions<'a> {
    pub fn new(
        attachments: &'a [Attachment],
        library_tree: &'a LibraryTree,
        language: Language,
    ) -> Self {
        Self {
            attachments,
            library_tree,
            language,
        }
    }
}

impl MarkdownExtensions for LocusMarkdownExtensions<'_> {
    fn render_code_block(&self, language: &str, source: &str, theme: &Theme) -> Option<AnyElement> {
        match language {
            "gallery" => Some(attachment_views::render_gallery(
                source,
                self.attachments,
                theme.clone(),
                self.language,
            )),
            "attachments" => Some(attachment_views::render_attachments(
                source,
                self.attachments,
                theme.clone(),
                self.language,
            )),
            _ => None,
        }
    }

    fn render_image(&self, url: &str, theme: &Theme) -> Option<AnyElement> {
        extension_syntax::find_image_attachment(self.attachments, url)
            .map(|attachment| attachment_views::render_thumbnail_image(attachment, theme.clone()))
    }

    fn render_link_icon(&self, url: &str, theme: &Theme) -> Option<AnyElement> {
        if let Some(path) = library_reference_path(url) {
            let Some(node) = self.library_tree.node(path) else {
                return Some(generic_link_icon(theme));
            };
            if let Some(source) = node.icon_image.as_ref() {
                let fallback_icon = node.icon.clone();
                let fallback_theme = theme.clone();
                return Some(
                    document_icon_image(source)
                        .size(px(13.0))
                        .rounded(px(3.0))
                        .object_fit(ObjectFit::Cover)
                        .with_fallback(move || {
                            fallback_icon
                                .clone()
                                .map(emoji_link_icon)
                                .unwrap_or_else(|| generic_link_icon(&fallback_theme))
                        })
                        .into_any_element(),
                );
            }
            return Some(
                node.icon
                    .clone()
                    .map(emoji_link_icon)
                    .unwrap_or_else(|| generic_link_icon(theme)),
            );
        }
        if url.starts_with("http://") || url.starts_with("https://") {
            let loading_theme = theme.clone();
            let fallback_theme = theme.clone();
            return Some(
                remote_link_icon(url)
                    .size(px(13.0))
                    .rounded(px(3.0))
                    .object_fit(ObjectFit::Cover)
                    .with_loading(move || generic_link_icon(&loading_theme))
                    .with_fallback(move || generic_link_icon(&fallback_theme))
                    .into_any_element(),
            );
        }
        if url.starts_with(LIBRARY_REFERENCE_SCHEME) {
            return Some(generic_link_icon(theme));
        }
        None
    }
}

fn emoji_link_icon(value: String) -> AnyElement {
    div()
        .size(px(13.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(12.0))
        .line_height(px(13.0))
        .child(value)
        .into_any_element()
}
