use rust_i18n::t;
use std::path::{Component, Path};

use locus_library::Attachment;
use markdown::{ParseOptions, mdast::Node};
use oneo_gpui::Language;

#[derive(Clone)]
pub(super) struct GalleryItem {
    pub attachment: Attachment,
}

pub(super) fn gallery_items(
    source: &str,
    attachments: &[Attachment],
    language: Language,
) -> Result<Vec<GalleryItem>, String> {
    let invalid = || {
        t!(
            "markdown.gallery.local_images_only",
            locale = language.locale()
        )
        .into_owned()
    };
    let nodes = paragraph_items(source).ok_or_else(invalid)?;
    let mut items = Vec::new();
    for node in nodes {
        if matches!(&node, Node::Text(text) if text.value.trim().is_empty())
            || matches!(&node, Node::Break(_))
        {
            continue;
        }
        let (url, alt) = match node {
            Node::Image(image) => (image.url, image.alt),
            Node::ImageReference(image) => (
                reference_url(source, &image.identifier).ok_or_else(invalid)?,
                image.alt,
            ),
            _ => return Err(invalid()),
        };
        let Some(attachment) = find_image_attachment(attachments, &url) else {
            return Err(t!(
                "markdown.gallery.image_unavailable",
                url = url,
                locale = language.locale()
            )
            .into_owned());
        };
        if alt.trim().is_empty() {
            return Err(t!(
                "markdown.gallery.alt_text_required",
                name = attachment.name.to_string(),
                locale = language.locale()
            )
            .into_owned());
        }
        items.push(GalleryItem {
            attachment: attachment.clone(),
        });
    }
    if items.is_empty() {
        return Err(t!(
            "markdown.gallery.image_required",
            locale = language.locale()
        )
        .into_owned());
    }
    Ok(items)
}

pub(super) fn attachment_items(
    source: &str,
    attachments: &[Attachment],
    language: Language,
) -> Result<Vec<Attachment>, String> {
    let invalid = || {
        t!(
            "markdown.attachments.local_links_only",
            locale = language.locale()
        )
        .into_owned()
    };
    let nodes = paragraph_items(source).ok_or_else(invalid)?;
    let mut items = Vec::new();
    for node in nodes {
        if matches!(&node, Node::Text(text) if text.value.trim().is_empty())
            || matches!(&node, Node::Break(_))
        {
            continue;
        }
        let (target, label) = match node {
            Node::Link(link) => (link.url, plain_text(&link.children)),
            Node::LinkReference(link) => (
                reference_url(source, &link.identifier).ok_or_else(invalid)?,
                plain_text(&link.children),
            ),
            _ => {
                return Err(t!(
                    "markdown.attachments.links_only",
                    locale = language.locale()
                )
                .into_owned());
            }
        };
        let Some(attachment) = local_attachment(attachments, &target) else {
            return Err(t!(
                "markdown.attachments.missing",
                target = target,
                locale = language.locale()
            )
            .into_owned());
        };
        if label.trim() != attachment.name {
            return Err(t!(
                "markdown.attachments.label_mismatch",
                name = attachment.name.to_string(),
                locale = language.locale()
            )
            .into_owned());
        }
        items.push(attachment.clone());
    }
    if items.is_empty() {
        return Err(t!(
            "markdown.attachments.link_required",
            locale = language.locale()
        )
        .into_owned());
    }
    Ok(items)
}

pub(super) fn find_image_attachment<'a>(
    attachments: &'a [Attachment],
    source: &str,
) -> Option<&'a Attachment> {
    let attachment = local_attachment(attachments, source)?;
    attachment.image_size.map(|_| attachment)
}

fn paragraph_items(source: &str) -> Option<Vec<Node>> {
    let Node::Root(root) = markdown::to_mdast(source, &ParseOptions::gfm()).ok()? else {
        return None;
    };
    let mut items = Vec::new();
    for node in root.children {
        match node {
            Node::Paragraph(paragraph) => items.extend(paragraph.children),
            Node::Definition(_) => {}
            _ => return None,
        }
    }
    Some(items)
}

pub(super) fn plain_text(nodes: &[Node]) -> String {
    nodes
        .iter()
        .map(|node| match node {
            Node::Text(text) => text.value.clone(),
            Node::InlineCode(code) => code.value.clone(),
            Node::Image(image) => image.alt.clone(),
            Node::ImageReference(image) => image.alt.clone(),
            Node::Break(_) => "\n".to_owned(),
            _ => node
                .children()
                .map(|children| plain_text(children))
                .unwrap_or_default(),
        })
        .collect()
}

pub(super) fn reference_url(source: &str, identifier: &str) -> Option<String> {
    fn find(node: &Node, identifier: &str) -> Option<String> {
        if let Node::Definition(definition) = node
            && definition.identifier.eq_ignore_ascii_case(identifier)
        {
            return Some(definition.url.clone());
        }
        node.children()?
            .iter()
            .find_map(|child| find(child, identifier))
    }
    find(
        &markdown::to_mdast(source, &ParseOptions::gfm()).ok()?,
        identifier,
    )
}

fn local_attachment<'a>(attachments: &'a [Attachment], source: &str) -> Option<&'a Attachment> {
    let source = source.strip_prefix("assets/")?;
    let source = percent_encoding::percent_decode_str(source)
        .decode_utf8()
        .ok()?;
    let path = Path::new(source.as_ref());
    let mut components = path.components();
    let Component::Normal(name) = components.next()? else {
        return None;
    };
    if components.next().is_some() {
        return None;
    }
    let name = name.to_str()?;
    attachments
        .iter()
        .find(|attachment| attachment.name == name)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use locus_library::ImageSize;

    fn attachment(name: &str, image: bool) -> Attachment {
        Attachment {
            name: name.to_owned(),
            path: PathBuf::from("/notes/example").join(name),
            modified: None,
            byte_len: 0,
            image_size: image.then_some(ImageSize {
                width: 1200,
                height: 800,
            }),
        }
    }

    #[test]
    fn gallery_accepts_only_local_images_with_alt_text() {
        let attachments = [attachment("sea.jpg", true)];
        let items = gallery_items(
            "![Sea](assets/sea.jpg)\n![Coast](assets/sea.jpg)",
            &attachments,
            Language::Chinese,
        )
        .unwrap();
        assert_eq!(items[0].attachment.name, "sea.jpg");
        assert_eq!(items.len(), 2);
        assert!(gallery_items("![](assets/sea.jpg)", &attachments, Language::Chinese).is_err());
        assert!(gallery_items("![Sea](../sea.jpg)", &attachments, Language::Chinese).is_err());
    }

    #[test]
    fn extensions_preserve_reference_links_and_reject_unsafe_or_non_attachment_content() {
        let attachments = [
            attachment("海 边.jpg", true),
            attachment("report.pdf", false),
        ];
        assert_eq!(
            gallery_items(
                "![海边][photo]\n\n[photo]: assets/海%20边.jpg",
                &attachments,
                Language::Chinese
            )
            .unwrap()
            .len(),
            1
        );
        assert_eq!(
            attachment_items(
                "[**report.pdf**][doc]\n\n[doc]: assets/report.pdf",
                &attachments,
                Language::Chinese
            )
            .unwrap()
            .len(),
            1
        );
        for source in [
            "![图](https://example.com/a.png)",
            "![图](assets/../海%20边.jpg)",
            "![图](assets/%2E%2E/海%20边.jpg)",
            "文字 ![图](assets/海%20边.jpg)",
            "- ![图](assets/海%20边.jpg)",
        ] {
            assert!(
                gallery_items(source, &attachments, Language::Chinese).is_err(),
                "{source}"
            );
        }
        assert!(
            attachment_items(
                "[report.pdf](assets/report.pdf) 多余文字",
                &attachments,
                Language::Chinese
            )
            .is_err()
        );
        assert!(
            attachment_items(
                "[report.pdf](assets/%2E%2E/report.pdf)",
                &attachments,
                Language::Chinese
            )
            .is_err()
        );
    }

    #[test]
    fn attachment_links_require_the_file_name_as_the_label() {
        let attachments = [attachment("report.pdf", false)];
        let items = attachment_items(
            "[report.pdf](assets/report.pdf)",
            &attachments,
            Language::Chinese,
        )
        .unwrap();
        assert_eq!(items[0].name, "report.pdf");
        assert!(
            attachment_items(
                "[Report](assets/report.pdf)",
                &attachments,
                Language::Chinese
            )
            .is_err()
        );
    }
}
