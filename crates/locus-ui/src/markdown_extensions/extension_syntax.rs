use rust_i18n::t;
use std::path::{Component, Path};

use locus_library::Attachment;
use oneo_gpui::Language;
use oneo_gpui_markdown::{Block, InlineRun, parse_blocks};

#[derive(Clone)]
pub(super) struct GalleryItem {
    pub attachment: Attachment,
}

pub(super) fn gallery_items(
    source: &str,
    attachments: &[Attachment],
    language: Language,
) -> Result<Vec<GalleryItem>, String> {
    let blocks = parse_blocks(source);
    let mut items = Vec::new();
    for block in blocks {
        if let Block::Paragraph { runs } = &block
            && runs.iter().all(|run| run.text.trim().is_empty())
        {
            continue;
        }
        let Block::Image { url, alt, .. } = block else {
            return Err(t!(
                "markdown.gallery.local_images_only",
                locale = language.locale()
            )
            .into_owned());
        };
        let Some(attachment) = find_image_attachment(attachments, &url) else {
            return Err(t!(
                "markdown.gallery.image_unavailable",
                url = url.to_string(),
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
    let blocks = parse_blocks(source);
    let mut items = Vec::new();
    for block in blocks {
        let Block::Paragraph { runs } = block else {
            return Err(t!(
                "markdown.attachments.local_links_only",
                locale = language.locale()
            )
            .into_owned());
        };
        collect_attachment_links(&runs, attachments, language, &mut items)?;
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

fn collect_attachment_links(
    runs: &[InlineRun],
    attachments: &[Attachment],
    language: Language,
    items: &mut Vec<Attachment>,
) -> Result<(), String> {
    let mut index = 0;
    while index < runs.len() {
        let run = &runs[index];
        let Some(target) = run.style.link.as_deref() else {
            if run.text.trim().is_empty() {
                index += 1;
                continue;
            }
            return Err(t!(
                "markdown.attachments.links_only",
                locale = language.locale()
            )
            .into_owned());
        };
        let mut label = String::new();
        while index < runs.len() && runs[index].style.link.as_deref() == Some(target) {
            label.push_str(&runs[index].text);
            index += 1;
        }
        let Some(attachment) = local_attachment(attachments, target) else {
            return Err(t!(
                "markdown.attachments.missing",
                target = target.to_string(),
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
    Ok(())
}

fn local_attachment<'a>(attachments: &'a [Attachment], source: &str) -> Option<&'a Attachment> {
    let path = Path::new(source);
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
            "![Sea](sea.jpg)\n![Coast](sea.jpg)",
            &attachments,
            Language::Chinese,
        )
        .unwrap();
        assert_eq!(items[0].attachment.name, "sea.jpg");
        assert_eq!(items.len(), 2);
        assert!(gallery_items("![](sea.jpg)", &attachments, Language::Chinese).is_err());
        assert!(gallery_items("![Sea](../sea.jpg)", &attachments, Language::Chinese).is_err());
    }

    #[test]
    fn attachment_links_require_the_file_name_as_the_label() {
        let attachments = [attachment("report.pdf", false)];
        let items =
            attachment_items("[report.pdf](report.pdf)", &attachments, Language::Chinese).unwrap();
        assert_eq!(items[0].name, "report.pdf");
        assert!(attachment_items("[Report](report.pdf)", &attachments, Language::Chinese).is_err());
    }
}
