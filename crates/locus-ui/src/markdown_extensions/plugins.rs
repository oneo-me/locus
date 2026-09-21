use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use gpui_kit::component::{
    Sizable, Theme,
    button::Button,
    text::{
        InlineElement, InlineRenderContext, MarkdownExtensions, MarkdownNode, MarkdownParseContext,
        MarkdownPlugin, markdown_ast::Node,
    },
};
use gpui_kit::{
    AnyElement, App, MouseButton, ObjectFit, SharedString, StyledImage, Window, div, img,
    prelude::*, rems,
};
use locus_library::Attachment;
use oneo_gpui::Language;

use super::{attachment_views, extension_syntax};

type OpenLink = dyn Fn(&str, &mut Window, &mut App) + Send + Sync;
type LinkIcon = dyn Fn(&str, &mut App) -> AnyElement + Send + Sync;

/// Product extensions only; TextView owns parsing, layout and text selection.
pub fn note_markdown_extensions(
    directory: &Path,
    attachments: &[Attachment],
    language: Language,
    open_link: impl Fn(&str, &mut Window, &mut App) + Send + Sync + 'static,
    link_icon: impl Fn(&str, &mut App) -> AnyElement + Send + Sync + 'static,
) -> MarkdownExtensions {
    let attachments: Arc<[Attachment]> = attachments.into();
    MarkdownExtensions::default()
        .plugin(AttachmentBlocks {
            attachments: attachments.clone(),
            language,
        })
        .plugin(NoteInline {
            directory: directory.to_owned(),
            attachments,
            open_link: Arc::new(open_link),
            link_icon: Arc::new(link_icon),
        })
}

struct AttachmentBlocks {
    attachments: Arc<[Attachment]>,
    language: Language,
}

struct AttachmentBlock {
    language: String,
    source: String,
}

impl MarkdownPlugin for AttachmentBlocks {
    fn name(&self) -> &str {
        "locus-attachments"
    }
    fn is_block(&self) -> bool {
        true
    }

    fn parse(&self, node: &Node, _: &MarkdownParseContext<'_>) -> Option<MarkdownNode> {
        let Node::Code(code) = node else {
            return None;
        };
        let language = code.lang.as_deref()?;
        if !matches!(language, "gallery" | "attachments") {
            return None;
        }
        // Attachment declarations are presentation data, not code to copy.
        Some(MarkdownNode::new(
            self.name(),
            AttachmentBlock {
                language: language.to_owned(),
                source: code.value.clone(),
            },
        ))
    }

    fn render(&self, node: &MarkdownNode, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let block = node
            .data::<AttachmentBlock>()
            .expect("attachment block data");
        let theme = Theme::global(cx).clone();
        match block.language.as_str() {
            "gallery" => attachment_views::render_gallery(
                &block.source,
                &self.attachments,
                theme,
                self.language,
            ),
            _ => attachment_views::render_attachments(
                &block.source,
                &self.attachments,
                theme,
                self.language,
            ),
        }
    }
}

struct NoteInline {
    directory: PathBuf,
    attachments: Arc<[Attachment]>,
    open_link: Arc<OpenLink>,
    link_icon: Arc<LinkIcon>,
}

enum NoteInlineData {
    Link { url: String, label: String },
    Image { url: String, alt: String },
}

impl MarkdownPlugin for NoteInline {
    fn name(&self) -> &str {
        "locus-inline"
    }

    fn parse(&self, node: &Node, context: &MarkdownParseContext<'_>) -> Option<MarkdownNode> {
        let data = match node {
            Node::Link(link) => NoteInlineData::Link {
                url: link.url.clone(),
                label: extension_syntax::plain_text(&link.children),
            },
            Node::Image(image) if image.url.starts_with("data:") => return None,
            Node::Image(image) => NoteInlineData::Image {
                url: image.url.clone(),
                alt: image.alt.clone(),
            },
            Node::LinkReference(link) => NoteInlineData::Link {
                url: extension_syntax::reference_url(context.source(), &link.identifier)?,
                label: extension_syntax::plain_text(&link.children),
            },
            Node::ImageReference(image) => NoteInlineData::Image {
                url: extension_syntax::reference_url(context.source(), &image.identifier)?,
                alt: image.alt.clone(),
            },
            _ => return None,
        };
        let text = match &data {
            NoteInlineData::Link { label, .. } => label.clone(),
            NoteInlineData::Image { alt, .. } => alt.clone(),
        };
        Some(
            MarkdownNode::new(self.name(), data)
                .text(text)
                .markdown(context.node_source(node).unwrap_or_default().to_owned()),
        )
    }

    fn render_inline(
        &self,
        node: &MarkdownNode,
        context: &InlineRenderContext,
        _: &mut Window,
        cx: &mut App,
    ) -> Option<InlineElement> {
        let theme = Theme::global(cx).clone();
        let element = match node.data::<NoteInlineData>()? {
            NoteInlineData::Link { url, label } => {
                let target = url.clone();
                let open_link = self.open_link.clone();
                let icon = (self.link_icon)(url, cx);
                let id = SharedString::from(format!("note-link-{}", node.source_range()?.start));
                Button::new(id)
                    .outline()
                    .small()
                    .rounded_full()
                    .accessibility_label(label.clone())
                    .tooltip(url.clone())
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(icon)
                            .child(label.clone()),
                    )
                    .on_click(move |_, window, cx| open_link(&target, window, cx))
                    .into_any_element()
            }
            NoteInlineData::Image { url, alt } => {
                let image = if let Some(attachment) =
                    extension_syntax::find_image_attachment(&self.attachments, url)
                {
                    attachment_views::render_thumbnail_image(attachment, theme.clone())
                } else {
                    let image = if url.contains("://") && !url.starts_with("file://")
                        || url.starts_with("data:")
                    {
                        img(url.clone())
                    } else {
                        img(image_path(&self.directory, url))
                    };
                    image
                        .max_w_full()
                        .max_h(rems(26.25))
                        .object_fit(ObjectFit::ScaleDown)
                        .into_any_element()
                };
                div()
                    .max_w_full()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(image)
                    .when(!alt.is_empty(), |view| {
                        view.child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child(alt.clone()),
                        )
                    })
                    .into_any_element()
            }
        };
        Some(InlineElement::new(element).with_baseline(context.font_size()))
    }
}

fn image_path(directory: &Path, url: &str) -> PathBuf {
    let path = url.strip_prefix("file://").unwrap_or(url);
    let decoded = percent_encoding::percent_decode_str(path).decode_utf8_lossy();
    directory.join(decoded.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn images_resolve_relative_absolute_and_encoded_paths() {
        let base = Path::new("/notes/article");
        assert_eq!(
            image_path(base, "assets/海%20边.png"),
            base.join("assets/海 边.png")
        );
        assert_eq!(
            image_path(base, "/shared/diagram.png"),
            PathBuf::from("/shared/diagram.png")
        );
        assert_eq!(
            image_path(base, "file:///shared/diagram.png"),
            PathBuf::from("/shared/diagram.png")
        );
    }
}
