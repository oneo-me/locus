use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use image::{ImageDecoder, ImageReader, metadata::Orientation};

use super::repository::{RepositoryChange, document_change};
use super::{ArchivedNote, Attachment, ChangeStatus, ImageSize, LibraryNode};

const DOCUMENT_FILE: &str = "index.md";
const DOCUMENT_ICON_FILE: &str = "icon.png";
static NEXT_TEMPORARY_FILE: AtomicU64 = AtomicU64::new(0);

#[derive(Default)]
struct DocumentMetadata {
    title: Option<String>,
    icon: Option<String>,
    summary: Option<String>,
    archived: bool,
}

#[derive(Default, serde::Deserialize)]
#[serde(default)]
struct RawDocumentMetadata {
    title: Option<serde_json::Value>,
    icon: Option<serde_json::Value>,
    summary: Option<serde_json::Value>,
    archived: bool,
}

pub(super) fn scan_children(
    root: &Path,
    directory: &Path,
    changes: &[RepositoryChange],
    ancestor_archived: bool,
    archived_notes: &mut Vec<ArchivedNote>,
) -> Result<Vec<LibraryNode>, std::io::Error> {
    let mut nodes = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !file_type.is_dir()
            || file_type.is_symlink()
            || name.starts_with('.')
            || name == "node_modules"
        {
            continue;
        }

        let path = entry.path();
        let document_path = path.join(DOCUMENT_FILE);
        let has_document = document_path
            .symlink_metadata()
            .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink());
        let metadata = if has_document {
            read_document_metadata(&document_path)?
        } else {
            DocumentMetadata::default()
        };
        let relative_directory = relative_path(root, &path);
        let title = metadata.title.unwrap_or_else(|| name.into_owned());
        let icon_path = regular_icon_path(&path);
        if metadata.archived {
            archived_notes.push(ArchivedNote {
                path: relative_directory.clone(),
                document_path: document_path.clone(),
                title: title.clone(),
                icon: metadata.icon.clone(),
                icon_path: icon_path.clone(),
            });
        }

        let hidden = ancestor_archived || metadata.archived;
        let mut children = scan_children(root, &path, changes, hidden, archived_notes)?;
        children.sort_by(compare_nodes);
        if hidden {
            continue;
        }
        if !has_document && children.is_empty() {
            continue;
        }

        let document_relative_path = format!("{relative_directory}/{DOCUMENT_FILE}");
        let change = has_document
            .then(|| document_change(&document_relative_path, &relative_directory, changes))
            .flatten();
        let descendant_change = combined_descendant_change(&children);
        let attachments = if has_document {
            read_attachments(&path)?
        } else {
            Vec::new()
        };
        nodes.push(LibraryNode {
            path: relative_directory,
            directory: path,
            document_path: has_document.then_some(document_path),
            title,
            icon: metadata.icon,
            icon_path,
            summary: metadata.summary,
            attachments,
            change,
            descendant_change,
            children,
        });
    }
    Ok(nodes)
}

pub(super) fn compare_nodes(left: &LibraryNode, right: &LibraryNode) -> std::cmp::Ordering {
    left.children
        .is_empty()
        .cmp(&right.children.is_empty())
        .then_with(|| left.path.cmp(&right.path))
}

pub fn load_note(path: &Path) -> Result<String, std::io::Error> {
    fs::read_to_string(path).map(|content| document_body(&content).to_owned())
}

pub fn restore_archived_note(path: &Path) -> Result<(), String> {
    let file_type = path
        .symlink_metadata()
        .map_err(|error| error.to_string())?
        .file_type();
    if !file_type.is_file() || file_type.is_symlink() {
        return Err("笔记文件不存在或无法安全访问。".to_owned());
    }

    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let restored = remove_archived_metadata(&content)?;
    write_atomically(path, restored.as_bytes())
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn regular_icon_path(directory: &Path) -> Option<std::path::PathBuf> {
    let path = directory.join(DOCUMENT_ICON_FILE);
    path.symlink_metadata()
        .ok()
        .filter(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
        .map(|_| path)
}

fn read_attachments(directory: &Path) -> Result<Vec<Attachment>, std::io::Error> {
    let mut attachments = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if file_type.is_file()
            && !file_type.is_symlink()
            && !name.starts_with('.')
            && name != DOCUMENT_FILE
            && !name.eq_ignore_ascii_case(DOCUMENT_ICON_FILE)
        {
            let path = entry.path();
            let metadata = entry.metadata()?;
            attachments.push(Attachment {
                name: name.into_owned(),
                image_size: image_size(&path),
                path,
                modified: metadata.modified().ok(),
                byte_len: metadata.len(),
            });
        }
    }
    attachments.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(attachments)
}

fn image_size(path: &Path) -> Option<ImageSize> {
    let reader = ImageReader::open(path).ok()?.with_guessed_format().ok()?;
    let mut decoder = reader.into_decoder().ok()?;
    let (mut width, mut height) = decoder.dimensions();
    if matches!(
        decoder.orientation().ok()?,
        Orientation::Rotate90
            | Orientation::Rotate270
            | Orientation::Rotate90FlipH
            | Orientation::Rotate270FlipH
    ) {
        std::mem::swap(&mut width, &mut height);
    }
    (width > 0 && height > 0).then_some(ImageSize { width, height })
}

fn read_document_metadata(path: &Path) -> Result<DocumentMetadata, std::io::Error> {
    let mut reader = BufReader::new(fs::File::open(path)?);
    let mut line = String::new();
    if reader.read_line(&mut line)? == 0
        || line
            .strip_prefix('\u{feff}')
            .unwrap_or(&line)
            .trim_end_matches(['\r', '\n'])
            != "---"
    {
        return Ok(DocumentMetadata::default());
    }

    let mut frontmatter = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            return Ok(DocumentMetadata::default());
        }
        if line.trim_end_matches(['\r', '\n']) == "---" {
            return Ok(parse_frontmatter_metadata(&frontmatter));
        }
        frontmatter.push_str(&line);
    }
}

fn parse_frontmatter_metadata(frontmatter: &str) -> DocumentMetadata {
    let Ok(raw) = serde_saphyr::from_str::<RawDocumentMetadata>(frontmatter) else {
        return DocumentMetadata::default();
    };
    DocumentMetadata {
        title: optional_string(raw.title),
        icon: optional_string(raw.icon),
        summary: optional_string(raw.summary),
        archived: raw.archived,
    }
}

fn document_body(content: &str) -> &str {
    let normalized = content.strip_prefix('\u{feff}').unwrap_or(content);
    frontmatter_parts(normalized).map_or(normalized, |(_, body)| body)
}

fn frontmatter_parts(content: &str) -> Option<(&str, &str)> {
    let rest = content
        .strip_prefix("---\r\n")
        .or_else(|| content.strip_prefix("---\n"))?;
    if let Some(end) = rest.find("\n---\r\n") {
        return Some((&rest[..end], &rest[end + 6..]));
    }
    if let Some(end) = rest.find("\n---\n") {
        return Some((&rest[..end], &rest[end + 5..]));
    }
    rest.strip_suffix("\n---")
        .map(|frontmatter| (frontmatter, ""))
}

fn remove_archived_metadata(content: &str) -> Result<String, String> {
    let (bom, normalized) = content
        .strip_prefix('\u{feff}')
        .map_or(("", content), |content| ("\u{feff}", content));
    let (opening, line_ending) = if normalized.starts_with("---\r\n") {
        ("---\r\n", "\r\n")
    } else if normalized.starts_with("---\n") {
        ("---\n", "\n")
    } else {
        return Err("笔记缺少有效的 YAML frontmatter。".to_owned());
    };
    let rest = &normalized[opening.len()..];
    let closing = format!("{line_ending}---");
    let Some(frontmatter_end) = rest.find(&closing) else {
        return Err("笔记缺少有效的 YAML frontmatter。".to_owned());
    };
    let frontmatter = &rest[..frontmatter_end];
    if !parse_frontmatter_metadata(frontmatter).archived {
        return Err("笔记当前没有归档。".to_owned());
    }

    let mut removed = false;
    let remaining = frontmatter
        .split(line_ending)
        .filter(|line| {
            if is_archived_metadata_line(line) {
                removed = true;
                false
            } else {
                true
            }
        })
        .collect::<Vec<_>>()
        .join(line_ending);
    if !removed || parse_frontmatter_metadata(&remaining).archived {
        return Err("无法安全移除归档字段。".to_owned());
    }

    Ok(format!(
        "{bom}{opening}{remaining}{}",
        &rest[frontmatter_end..]
    ))
}

fn is_archived_metadata_line(line: &str) -> bool {
    if line.starts_with(char::is_whitespace) {
        return false;
    }
    let Some((key, _)) = line.split_once(':') else {
        return false;
    };
    matches!(key.trim_end(), "archived" | "'archived'" | "\"archived\"")
}

fn write_atomically(path: &Path, content: &[u8]) -> Result<(), String> {
    let directory = path.parent().ok_or_else(|| "笔记路径无效。".to_owned())?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "笔记路径无效。".to_owned())?;
    let temporary = directory.join(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        NEXT_TEMPORARY_FILE.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let permissions = fs::metadata(path)
            .map_err(|error| error.to_string())?
            .permissions();
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| error.to_string())?;
        file.set_permissions(permissions)
            .map_err(|error| error.to_string())?;
        file.write_all(content).map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        fs::rename(&temporary, path).map_err(|error| error.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn optional_string(value: Option<serde_json::Value>) -> Option<String> {
    let value = match value? {
        serde_json::Value::String(value) => value,
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::Number(value) => value.to_string(),
        _ => return None,
    };
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_owned())
    }
}

fn combined_descendant_change(children: &[LibraryNode]) -> Option<ChangeStatus> {
    let statuses = children
        .iter()
        .flat_map(|child| [child.change, child.descendant_change])
        .flatten()
        .collect::<HashSet<_>>();
    if statuses.contains(&ChangeStatus::Added) {
        Some(ChangeStatus::Added)
    } else if statuses.contains(&ChangeStatus::Modified) {
        Some(ChangeStatus::Modified)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_sidebar_metadata_from_frontmatter() {
        let document = "---\ntitle: 'Locus: Notes'\nicon: 🧭\narchived: true\n---\nBody";
        let metadata = parse_frontmatter_metadata(frontmatter_parts(document).unwrap().0);
        assert_eq!(metadata.title.as_deref(), Some("Locus: Notes"));
        assert_eq!(metadata.icon.as_deref(), Some("🧭"));
        assert!(metadata.archived);
    }

    #[test]
    fn reads_summary_and_separates_markdown_body() {
        let document = "---\ntitle: Notes\nsummary: A short summary.\n---\n# Heading\n\nBody";
        let metadata = parse_frontmatter_metadata(frontmatter_parts(document).unwrap().0);
        assert_eq!(metadata.summary.as_deref(), Some("A short summary."));
        assert_eq!(document_body(document), "# Heading\n\nBody");
        assert_eq!(document_body("---\ntitle: Notes\n---"), "");
    }

    #[test]
    fn removes_only_the_archived_frontmatter_field() {
        let document = "\u{feff}---\r\ntitle: Notes\r\narchived: true # hidden\r\nsummary: Kept\r\n---\r\n# Body\r\n";

        assert_eq!(
            remove_archived_metadata(document).unwrap(),
            "\u{feff}---\r\ntitle: Notes\r\nsummary: Kept\r\n---\r\n# Body\r\n"
        );
    }

    #[test]
    fn refuses_to_rewrite_an_unarchived_document() {
        let error = remove_archived_metadata("---\ntitle: Notes\n---\nBody").unwrap_err();

        assert_eq!(error, "笔记当前没有归档。");
    }
}
