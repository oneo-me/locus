use std::fs::{self, OpenOptions};
use std::io::{Cursor, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader};

const DOCUMENT_ICON_FILE: &str = "icon.png";
const DOCUMENT_ICON_SIZE: u32 = 64;
const MAX_SOURCE_IMAGE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_DOCUMENT_ICON_BYTES: usize = 256 * 1024;
static NEXT_TEMPORARY_FILE: AtomicU64 = AtomicU64::new(0);

pub fn archive_note(path: &Path) -> Result<(), String> {
    let content = read_regular_document(path)?;
    let archived = set_frontmatter_scalar(&content, "archived", "true")?;
    write_atomically(path, archived.as_bytes())
}

pub fn restore_archived_note(path: &Path) -> Result<(), String> {
    let content = read_regular_document(path)?;
    let restored = remove_archived_frontmatter(&content)?;
    write_atomically(path, restored.as_bytes())
}

pub fn update_note_icon(document_path: &Path, source_path: &Path) -> Result<(), String> {
    let content = read_regular_document(document_path)?;
    let updated_content = set_frontmatter_scalar(&content, "icon", DOCUMENT_ICON_FILE)?;
    let icon = prepare_icon(source_path)?;
    let directory = document_path
        .parent()
        .ok_or_else(|| "笔记路径无效。".to_owned())?;
    let icon_path = directory.join(DOCUMENT_ICON_FILE);
    let previous_icon = match icon_path.symlink_metadata() {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            return Err("笔记图标路径无法安全写入。".to_owned());
        }
        Ok(_) => Some(fs::read(&icon_path).map_err(|error| error.to_string())?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.to_string()),
    };

    write_atomically(&icon_path, &icon)?;
    if let Err(error) = write_atomically(document_path, updated_content.as_bytes()) {
        let rollback = match previous_icon {
            Some(previous) => write_atomically(&icon_path, &previous),
            None => fs::remove_file(&icon_path).map_err(|rollback| rollback.to_string()),
        };
        return match rollback {
            Ok(()) => Err(error),
            Err(rollback) => Err(format!("{error}；同时无法回滚图标：{rollback}")),
        };
    }
    Ok(())
}

fn read_regular_document(path: &Path) -> Result<String, String> {
    let file_type = path
        .symlink_metadata()
        .map_err(|error| error.to_string())?
        .file_type();
    if !file_type.is_file() || file_type.is_symlink() {
        return Err("笔记文件不存在或无法安全访问。".to_owned());
    }
    fs::read_to_string(path).map_err(|error| error.to_string())
}

fn prepare_icon(path: &Path) -> Result<Vec<u8>, String> {
    let metadata = path.symlink_metadata().map_err(|error| error.to_string())?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("请选择图片文件。".to_owned());
    }
    if metadata.len() > MAX_SOURCE_IMAGE_BYTES {
        return Err("原始图片不能超过 16 MiB。".to_owned());
    }

    let reader = ImageReader::open(path)
        .map_err(|_| "无法读取这张图片。".to_owned())?
        .with_guessed_format()
        .map_err(|_| "无法读取这张图片。".to_owned())?;
    let mut decoder = reader
        .into_decoder()
        .map_err(|_| "无法读取这张图片。".to_owned())?;
    let orientation = decoder
        .orientation()
        .map_err(|_| "无法读取这张图片。".to_owned())?;
    let mut image =
        DynamicImage::from_decoder(decoder).map_err(|_| "无法读取这张图片。".to_owned())?;
    if image.width() == 0 || image.height() == 0 {
        return Err("图片尺寸无效。".to_owned());
    }
    image.apply_orientation(orientation);
    let icon = if image.width() > DOCUMENT_ICON_SIZE || image.height() > DOCUMENT_ICON_SIZE {
        image.thumbnail(DOCUMENT_ICON_SIZE, DOCUMENT_ICON_SIZE)
    } else {
        image
    };
    let mut encoded = Cursor::new(Vec::new());
    icon.write_to(&mut encoded, ImageFormat::Png)
        .map_err(|_| "无法生成 PNG 图标。".to_owned())?;
    let encoded = encoded.into_inner();
    if encoded.len() > MAX_DOCUMENT_ICON_BYTES {
        return Err("处理后的图标不能超过 256 KiB。".to_owned());
    }
    Ok(encoded)
}

fn set_frontmatter_scalar(content: &str, key: &str, value: &str) -> Result<String, String> {
    let (bom, normalized) = split_bom(content);
    let Some(parts) = FrontmatterParts::parse(normalized)? else {
        let line_ending = if normalized.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        return Ok(format!(
            "{bom}---{line_ending}{key}: {value}{line_ending}---{line_ending}{normalized}"
        ));
    };
    let metadata = frontmatter_object(parts.content)?;
    let lines = parts.content.split(parts.line_ending).collect::<Vec<_>>();
    let matches = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (frontmatter_key(line) == Some(key)).then_some(index))
        .collect::<Vec<_>>();
    if matches.len() > 1 {
        return Err(format!("无法安全更新 {key} 字段。"));
    }
    if matches.is_empty() && metadata.contains_key(key) {
        return Err(format!("无法安全更新 {key} 字段。"));
    }
    if let Some(index) = matches.first() {
        let line = lines[*index];
        let raw_value = line.split_once(':').map_or("", |(_, value)| value.trim());
        if raw_value.starts_with('|') || raw_value.starts_with('>') {
            return Err(format!("无法安全更新 {key} 字段。"));
        }
    }

    let field = format!("{key}: {value}");
    let updated = if let Some(index) = matches.first() {
        lines
            .into_iter()
            .enumerate()
            .map(|(line_index, line)| {
                if line_index == *index {
                    field.as_str()
                } else {
                    line
                }
            })
            .collect::<Vec<_>>()
            .join(parts.line_ending)
    } else if parts.content.is_empty() {
        field
    } else {
        format!(
            "{}{ending}{field}",
            parts.content,
            ending = parts.line_ending
        )
    };
    let updated_metadata = frontmatter_object(&updated)?;
    let updated_value_matches = match key {
        "archived" => {
            updated_metadata
                .get(key)
                .and_then(serde_json::Value::as_bool)
                == Some(true)
        }
        "icon" => {
            updated_metadata
                .get(key)
                .and_then(serde_json::Value::as_str)
                == Some(value)
        }
        _ => false,
    };
    if !updated_value_matches {
        return Err(format!("无法安全更新 {key} 字段。"));
    }
    Ok(parts.reassemble(bom, &updated))
}

fn remove_archived_frontmatter(content: &str) -> Result<String, String> {
    let (bom, normalized) = split_bom(content);
    let Some(parts) = FrontmatterParts::parse(normalized)? else {
        return Err("笔记缺少有效的 YAML frontmatter。".to_owned());
    };
    let metadata = frontmatter_object(parts.content)?;
    if metadata
        .get("archived")
        .and_then(serde_json::Value::as_bool)
        != Some(true)
    {
        return Err("笔记当前没有归档。".to_owned());
    }
    let mut removed = 0;
    let remaining = parts
        .content
        .split(parts.line_ending)
        .filter(|line| {
            if frontmatter_key(line) == Some("archived") {
                removed += 1;
                false
            } else {
                true
            }
        })
        .collect::<Vec<_>>()
        .join(parts.line_ending);
    if removed == 0 {
        return Err("笔记当前没有归档。".to_owned());
    }
    if removed > 1 {
        return Err("无法安全移除 archived 字段。".to_owned());
    }
    if frontmatter_object(&remaining)?
        .get("archived")
        .and_then(serde_json::Value::as_bool)
        == Some(true)
    {
        return Err("无法安全移除 archived 字段。".to_owned());
    }
    Ok(parts.reassemble(bom, &remaining))
}

fn frontmatter_object(
    frontmatter: &str,
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    if frontmatter
        .lines()
        .all(|line| line.trim().is_empty() || line.trim_start().starts_with('#'))
    {
        return Ok(serde_json::Map::new());
    }
    let value = serde_saphyr::from_str::<serde_json::Value>(frontmatter)
        .map_err(|_| "笔记缺少有效的 YAML frontmatter。".to_owned())?;
    value
        .as_object()
        .cloned()
        .ok_or_else(|| "笔记缺少有效的 YAML frontmatter。".to_owned())
}

fn frontmatter_key(line: &str) -> Option<&str> {
    if line.starts_with(char::is_whitespace) {
        return None;
    }
    let (key, _) = line.split_once(':')?;
    let key = key.trim_end();
    Some(
        key.strip_prefix('\'')
            .and_then(|key| key.strip_suffix('\''))
            .or_else(|| key.strip_prefix('"').and_then(|key| key.strip_suffix('"')))
            .unwrap_or(key),
    )
}

fn split_bom(content: &str) -> (&str, &str) {
    content
        .strip_prefix('\u{feff}')
        .map_or(("", content), |content| ("\u{feff}", content))
}

struct FrontmatterParts<'a> {
    opening: &'static str,
    line_ending: &'static str,
    content: &'a str,
    suffix: &'a str,
}

impl<'a> FrontmatterParts<'a> {
    fn parse(content: &'a str) -> Result<Option<Self>, String> {
        let (opening, line_ending) = if content.starts_with("---\r\n") {
            ("---\r\n", "\r\n")
        } else if content.starts_with("---\n") {
            ("---\n", "\n")
        } else {
            return Ok(None);
        };
        let rest = &content[opening.len()..];
        let closing = format!("{line_ending}---");
        let Some(end) = rest.find(&closing) else {
            return Err("笔记缺少有效的 YAML frontmatter。".to_owned());
        };
        Ok(Some(Self {
            opening,
            line_ending,
            content: &rest[..end],
            suffix: &rest[end..],
        }))
    }

    fn reassemble(&self, bom: &str, content: &str) -> String {
        format!("{bom}{}{content}{}", self.opening, self.suffix)
    }
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
        let permissions = match path.symlink_metadata() {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
                return Err("目标路径无法安全写入。".to_owned());
            }
            Ok(metadata) => Some(metadata.permissions()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.to_string()),
        };
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| error.to_string())?;
        if let Some(permissions) = permissions {
            file.set_permissions(permissions)
                .map_err(|error| error.to_string())?;
        }
        file.write_all(content).map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        fs::rename(&temporary, path).map_err(|error| error.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archives_and_restores_without_changing_other_content() {
        let document = "\u{feff}---\r\ntitle: Notes\r\nsummary: Kept\r\n---\r\n# Body\r\n";
        let archived = set_frontmatter_scalar(document, "archived", "true").unwrap();
        assert_eq!(
            archived,
            "\u{feff}---\r\ntitle: Notes\r\nsummary: Kept\r\narchived: true\r\n---\r\n# Body\r\n"
        );
        assert_eq!(remove_archived_frontmatter(&archived).unwrap(), document);
    }

    #[test]
    fn adds_frontmatter_when_setting_an_icon() {
        assert_eq!(
            set_frontmatter_scalar("# Notes\n", "icon", "icon.png").unwrap(),
            "---\nicon: icon.png\n---\n# Notes\n"
        );
    }

    #[test]
    fn replaces_a_quoted_icon_key() {
        assert_eq!(
            set_frontmatter_scalar(
                "---\n'icon': 🧭\ntitle: Notes\n---\nBody",
                "icon",
                "icon.png"
            )
            .unwrap(),
            "---\nicon: icon.png\ntitle: Notes\n---\nBody"
        );
    }

    #[test]
    fn refuses_to_restore_an_unarchived_document() {
        let error = remove_archived_frontmatter("---\ntitle: Notes\n---\nBody").unwrap_err();
        assert_eq!(error, "笔记当前没有归档。");

        let error = remove_archived_frontmatter("---\narchived: false\n---\nBody").unwrap_err();
        assert_eq!(error, "笔记当前没有归档。");
    }

    #[test]
    fn prepares_a_png_icon_without_cropping_or_enlarging() {
        let directory = std::env::temp_dir().join(format!(
            "locus-icon-test-{}-{}",
            std::process::id(),
            NEXT_TEMPORARY_FILE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        let source = directory.join("source.png");
        DynamicImage::new_rgba8(120, 80)
            .save_with_format(&source, ImageFormat::Png)
            .unwrap();

        let icon = prepare_icon(&source).unwrap();
        let decoded = image::load_from_memory_with_format(&icon, ImageFormat::Png).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (64, 43));

        let small_source = directory.join("small.png");
        DynamicImage::new_rgba8(32, 20)
            .save_with_format(&small_source, ImageFormat::Png)
            .unwrap();
        let small_icon = prepare_icon(&small_source).unwrap();
        let decoded = image::load_from_memory_with_format(&small_icon, ImageFormat::Png).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (32, 20));

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn icon_update_writes_the_png_and_frontmatter() {
        let directory = std::env::temp_dir().join(format!(
            "locus-icon-update-test-{}-{}",
            std::process::id(),
            NEXT_TEMPORARY_FILE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        let document = directory.join("index.md");
        fs::write(&document, "---\ntitle: Notes\nicon: 🧭\n---\nBody").unwrap();
        let source = directory.join("source.png");
        DynamicImage::new_rgba8(128, 64)
            .save_with_format(&source, ImageFormat::Png)
            .unwrap();

        update_note_icon(&document, &source).unwrap();

        assert_eq!(
            fs::read_to_string(&document).unwrap(),
            "---\ntitle: Notes\nicon: icon.png\n---\nBody"
        );
        let icon = image::open(directory.join(DOCUMENT_ICON_FILE)).unwrap();
        assert_eq!((icon.width(), icon.height()), (64, 32));

        fs::remove_dir_all(directory).unwrap();
    }
}
