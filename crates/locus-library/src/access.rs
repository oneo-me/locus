use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

mod organize;
#[cfg(test)]
mod tests;

pub const MAX_TEXT_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Resource {
    Note,
    Canvas,
    CanvasFile { name: String },
    Icon,
    Attachment { name: String },
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ContentTarget {
    uri: String,
    resource: Resource,
}

impl ContentTarget {
    pub fn new(uri: impl Into<String>, resource: Resource) -> Self {
        Self {
            uri: uri.into(),
            resource,
        }
    }

    pub fn uri(&self) -> &str {
        &self.uri
    }
    pub fn resource(&self) -> &Resource {
        &self.resource
    }
}

pub struct LibraryAccess {
    root: Option<PathBuf>,
}

impl LibraryAccess {
    pub fn new(root: Option<PathBuf>) -> Self {
        Self {
            root: root.map(|path| path.canonicalize().unwrap_or(path)),
        }
    }

    fn root(&self) -> Result<PathBuf, String> {
        let root = self
            .root
            .as_ref()
            .ok_or("library_not_configured: Select a library first.")?;
        if !fs::symlink_metadata(root)
            .map_err(io_error)?
            .file_type()
            .is_dir()
        {
            return Err("invalid_library: Library is not a directory.".into());
        }
        Ok(root.clone())
    }

    fn directory(&self, uri: &str) -> Result<PathBuf, String> {
        let mut path = self.root()?;
        for name in uri_segments(uri)? {
            path.push(name);
            if !fs::symlink_metadata(&path)
                .map_err(io_error)?
                .file_type()
                .is_dir()
            {
                return Err("unsafe_target: Expected an ordinary directory.".into());
            }
        }
        Ok(path)
    }

    fn target_path(&self, target: &ContentTarget) -> Result<PathBuf, String> {
        if uri_segments(&target.uri)?.is_empty() {
            return Err("invalid_uri: Select a note.".into());
        }
        let directory = self.directory(&target.uri)?;
        if target.resource != Resource::Note {
            require_file(&directory.join("index.md"))?;
        }
        crate::canvas::regular_path(&directory, &resource_name(&target.resource)?, false)
            .map_err(io_error)
    }

    fn prepare_resource_parents(&self, target: &ContentTarget) -> Result<(), String> {
        let directory = self.directory(&target.uri)?;
        crate::canvas::regular_path(&directory, &resource_name(&target.resource)?, true)
            .map_err(io_error)?;
        Ok(())
    }

    pub fn validate_write(&self, target: &ContentTarget, expected: &str) -> Result<(), String> {
        let path = self.target_path(target)?;
        check_version(optional_version(&path)?.as_deref(), expected)
    }

    pub fn stat(&self, target: &ContentTarget) -> Result<Value, String> {
        let path = self.target_path(target)?;
        let (size, version) = file_version(&path)?;
        Ok(json!({"target": target, "size": size, "version": version}))
    }

    pub fn read_chunk(
        &self,
        target: &ContentTarget,
        version: &str,
        offset: u64,
        limit: usize,
    ) -> Result<Vec<u8>, String> {
        if limit == 0 || limit > 256 * 1024 {
            return Err("invalid_limit: Chunk size must be 1..262144 bytes.".into());
        }
        let path = self.target_path(target)?;
        let (size, current) = file_version(&path)?;
        check_version(Some(&current), version)?;
        if offset > size {
            return Err("invalid_offset: Offset exceeds file size.".into());
        }
        let mut file = File::open(&path).map_err(io_error)?;
        file.seek(SeekFrom::Start(offset)).map_err(io_error)?;
        let mut bytes = Vec::new();
        file.take(limit as u64)
            .read_to_end(&mut bytes)
            .map_err(io_error)?;
        check_version(Some(&file_version(&path)?.1), version)?;
        Ok(bytes)
    }

    pub fn read_text(&self, target: &ContentTarget) -> Result<(String, String), String> {
        let path = self.target_path(target)?;
        require_file(&path)?;
        let mut bytes = Vec::new();
        File::open(&path)
            .map_err(io_error)?
            .take(MAX_TEXT_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(io_error)?;
        if bytes.len() as u64 > MAX_TEXT_BYTES {
            return Err("text_too_large: Use stat_content and download_chunk.".into());
        }
        let version = content_version(&bytes);
        let text = String::from_utf8(bytes)
            .map_err(|_| "invalid_utf8: Use download_chunk for binary content.".to_owned())?;
        Ok((text, version))
    }

    pub fn write_text(
        &self,
        target: &ContentTarget,
        text: &str,
        expected: &str,
    ) -> Result<Value, String> {
        if text.len() as u64 > MAX_TEXT_BYTES {
            return Err("text_too_large: Text exceeds 8 MiB.".into());
        }
        if target.resource == Resource::Icon {
            return Err("invalid_resource: Upload image bytes to update an icon.".into());
        }
        self.write_content(target, &mut text.as_bytes(), expected)
    }

    pub fn write_content(
        &self,
        target: &ContentTarget,
        source: &mut dyn Read,
        expected: &str,
    ) -> Result<Value, String> {
        let path = self.target_path(target)?;
        let current = optional_version(&path)?;
        check_version(current.as_deref(), expected)?;
        let directory = path.parent().ok_or("invalid_target")?;
        self.prepare_resource_parents(target)?;
        let mut pending = tempfile::NamedTempFile::new_in(directory).map_err(io_error)?;
        std::io::copy(source, &mut pending).map_err(io_error)?;
        if matches!(target.resource, Resource::Note | Resource::Canvas) {
            if pending.as_file().metadata().map_err(io_error)?.len() > MAX_TEXT_BYTES {
                return Err("text_too_large: Notes and canvases cannot exceed 8 MiB.".into());
            }
            let bytes = fs::read(pending.path()).map_err(io_error)?;
            std::str::from_utf8(&bytes)
                .map_err(|_| "invalid_utf8: Notes and canvases must be UTF-8.".to_owned())?;
        }
        if target.resource == Resource::Icon {
            let document = ContentTarget::new(&target.uri, Resource::Note);
            let (content, note_version) = self.read_text(&document)?;
            let updated =
                crate::mutations::set_frontmatter_scalar(&content, "icon", "assets/icon.png")?;
            let bytes = crate::mutations::prepare_icon(pending.path())?;
            pending.as_file_mut().set_len(0).map_err(io_error)?;
            pending
                .as_file_mut()
                .seek(SeekFrom::Start(0))
                .map_err(io_error)?;
            pending.write_all(&bytes).map_err(io_error)?;
            check_version(optional_version(&path)?.as_deref(), expected)?;
            let previous = if current.is_some() {
                Some(fs::read(&path).map_err(io_error)?)
            } else {
                None
            };
            let icon_version = content_version(&bytes);
            self.commit_file(target, pending, expected)?;
            if let Err(error) = self.write_text(&document, &updated, &note_version) {
                let path = self.target_path(target)?;
                check_version(optional_version(&path)?.as_deref(), &icon_version).map_err(
                    |_| {
                        "partial_failure: Icon changed during rollback; inspect the note and icon."
                            .to_owned()
                    },
                )?;
                match previous {
                    Some(bytes) => {
                        crate::mutations::write_atomically(&path, &bytes)?;
                    }
                    None => {
                        fs::remove_file(&path).map_err(io_error)?;
                    }
                }
                return Err(error);
            }
        } else {
            self.commit_file(target, pending, expected)?;
        }
        self.stat(target)
    }

    fn commit_file(
        &self,
        target: &ContentTarget,
        mut pending: tempfile::NamedTempFile,
        expected: &str,
    ) -> Result<(), String> {
        let path = self.target_path(target)?;
        check_version(optional_version(&path)?.as_deref(), expected)?;
        if expected != "missing" {
            pending
                .as_file_mut()
                .set_permissions(fs::metadata(&path).map_err(io_error)?.permissions())
                .map_err(io_error)?;
        }
        pending.as_file_mut().sync_all().map_err(io_error)?;
        if expected == "missing" {
            pending
                .persist_noclobber(&path)
                .map_err(|e| io_error(e.error))?;
        } else {
            pending.persist(&path).map_err(|e| io_error(e.error))?;
        }
        Ok(())
    }

    pub fn patch_text(
        &self,
        target: &ContentTarget,
        expected: &str,
        old: &str,
        new: &str,
    ) -> Result<Value, String> {
        let (content, version) = self.read_text(target)?;
        check_version(Some(&version), expected)?;
        if old.is_empty() || content.matches(old).count() != 1 {
            return Err("ambiguous_edit: old_text must match exactly once.".into());
        }
        self.write_text(target, &content.replacen(old, new, 1), expected)
    }

    pub fn set_archived(&self, uri: &str, expected: &str, archived: bool) -> Result<Value, String> {
        let target = ContentTarget::new(uri, Resource::Note);
        let (content, version) = self.read_text(&target)?;
        check_version(Some(&version), expected)?;
        let updated = crate::mutations::set_frontmatter_scalar(
            &content,
            "archived",
            if archived { "true" } else { "false" },
        )?;
        self.write_text(&target, &updated, expected)
    }

    pub fn delete_content(&self, target: &ContentTarget, expected: &str) -> Result<Value, String> {
        let path = self.target_path(target)?;
        check_version(optional_version(&path)?.as_deref(), expected)?;
        require_file(&path)?;
        if target.resource == Resource::Icon {
            let note = ContentTarget::new(&target.uri, Resource::Note);
            let (content, version) = self.read_text(&note)?;
            let updated = crate::mutations::set_frontmatter_scalar(&content, "icon", "null")?;
            self.write_text(&note, &updated, &version)?;
            if let Err(error) = fs::remove_file(&path) {
                self.write_text(&note, &content, &content_version(updated.as_bytes()))?;
                return Err(io_error(error));
            }
        } else {
            fs::remove_file(path).map_err(io_error)?;
        }
        Ok(json!({"deleted": target}))
    }

    pub fn create_directory(&self, uri: &str) -> Result<Value, String> {
        let (parent, name) = parent_uri(uri)?;
        let path = self.directory(&parent)?.join(name);
        fs::create_dir(path).map_err(io_error)?;
        self.describe(uri)
    }

    pub fn create_note(&self, uri: &str, markdown: &str) -> Result<Value, String> {
        let (parent, name) = parent_uri(uri)?;
        let parent = self.directory(&parent)?;
        let path = parent.join(name);
        let created = match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_dir() => false,
            Ok(_) => return Err("already_exists: Target is not an ordinary directory.".into()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&path).map_err(io_error)?;
                true
            }
            Err(error) => return Err(io_error(error)),
        };
        let result = self.write_text(
            &ContentTarget::new(uri, Resource::Note),
            markdown,
            "missing",
        );
        if result.is_err() && created {
            let _ = fs::remove_dir(path);
        }
        result
    }

    pub fn describe(&self, uri: &str) -> Result<Value, String> {
        let directory = self.directory(uri)?;
        let mut metadata = json!({});
        let note = if uri_segments(uri)?.is_empty() {
            None
        } else {
            optional_version(&directory.join("index.md"))?
        };
        if note.is_some() {
            let parsed = crate::scanner::read_document_metadata(&directory.join("index.md"))
                .map_err(io_error)?;
            metadata = json!({"title": parsed.title, "icon": parsed.icon, "summary": parsed.summary, "archived": parsed.archived});
            for (key, value) in parsed.additional {
                metadata[&key] = value;
            }
        }
        let mut files = Vec::new();
        let mut children = 0;
        for entry in fs::read_dir(&directory).map_err(io_error)? {
            let entry = entry.map_err(io_error)?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| "invalid_name: Non-UTF-8 library name.".to_owned())?;
            if name.starts_with('.')
                || name == "node_modules"
                || crate::is_reserved_directory(&name)
            {
                continue;
            }
            let kind = entry.file_type().map_err(io_error)?;
            if kind.is_dir() {
                children += 1;
            }
            if kind.is_file() && note.is_some() && name == "index.md" {
                files.push(json!({"name": name, "resource": resource_for_name(&name), "size": entry.metadata().map_err(io_error)?.len()}));
            }
        }
        if note.is_some() {
            for folder in ["assets", "canvas"] {
                let path = directory.join(folder);
                match fs::symlink_metadata(&path) {
                    Ok(metadata) if metadata.file_type().is_dir() => {
                        collect_resources(&path, folder, "", &mut files)?
                    }
                    Ok(_) => {
                        return Err("unsafe_target: Resource directory must be ordinary.".into());
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
                    Err(error) => return Err(io_error(error)),
                }
            }
        }
        files.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
        Ok(
            json!({"uri": canonical_uri(&uri_segments(uri)?), "kind": if note.is_some() {"note"} else {"directory"}, "metadata": metadata, "version": note, "files": files, "childCount": children}),
        )
    }

    pub fn list(&self, parent: &str, cursor: usize, limit: usize) -> Result<Value, String> {
        if limit == 0 || limit > 200 {
            return Err("invalid_limit: limit must be 1..200.".into());
        }
        let directory = self.directory(parent)?;
        let mut names = Vec::new();
        for entry in fs::read_dir(directory).map_err(io_error)? {
            let entry = entry.map_err(io_error)?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| "invalid_name: Non-UTF-8 library name.".to_owned())?;
            if !name.starts_with('.')
                && name != "node_modules"
                && !crate::is_reserved_directory(&name)
                && entry.file_type().map_err(io_error)?.is_dir()
            {
                names.push(name);
            }
        }
        names.sort();
        let mut entries = Vec::new();
        for name in names.iter().skip(cursor).take(limit) {
            let mut segments = uri_segments(parent)?;
            segments.push(name.clone());
            entries.push(self.describe(&canonical_uri(&segments))?);
        }
        Ok(
            json!({"entries": entries, "nextCursor": (cursor.saturating_add(limit) < names.len()).then_some(cursor.saturating_add(limit))}),
        )
    }

    pub fn search(&self, query: &str, cursor: usize, limit: usize) -> Result<Value, String> {
        if query.trim().is_empty() || limit == 0 || limit > 100 {
            return Err("invalid_query: Provide a query and limit 1..100.".into());
        }
        let mut matches = Vec::new();
        let needle = query.trim().to_lowercase();
        for uri in self.all_directories("locus://")? {
            let description = self.describe(&uri)?;
            let mut searchable = description.to_string();
            if description["kind"] == "note" {
                searchable.push_str(&self.read_text(&ContentTarget::new(&uri, Resource::Note))?.0);
            }
            if searchable.to_lowercase().contains(&needle) {
                matches.push(description);
            }
        }
        let total = matches.len();
        let entries = matches
            .into_iter()
            .skip(cursor)
            .take(limit)
            .collect::<Vec<_>>();
        Ok(
            json!({"entries": entries, "total": total, "nextCursor": (cursor.saturating_add(limit) < total).then_some(cursor.saturating_add(limit))}),
        )
    }

    fn all_directories(&self, uri: &str) -> Result<Vec<String>, String> {
        let mut pending = vec![uri.to_owned()];
        let mut result = Vec::new();
        while let Some(parent) = pending.pop() {
            let directory = self.directory(&parent)?;
            for entry in fs::read_dir(directory).map_err(io_error)? {
                let entry = entry.map_err(io_error)?;
                let name = entry
                    .file_name()
                    .into_string()
                    .map_err(|_| "invalid_name".to_owned())?;
                if name.starts_with('.')
                    || name == "node_modules"
                    || crate::is_reserved_directory(&name)
                    || !entry.file_type().map_err(io_error)?.is_dir()
                {
                    continue;
                }
                let mut segments = uri_segments(&parent)?;
                segments.push(name);
                let child = canonical_uri(&segments);
                result.push(child.clone());
                pending.push(child);
            }
        }
        result.sort();
        Ok(result)
    }
}

pub fn content_version(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn file_version(path: &Path) -> Result<(u64, String), String> {
    require_file(path)?;
    let mut file = File::open(path).map_err(io_error)?;
    let mut digest = Sha256::new();
    let size = std::io::copy(&mut file, &mut digest).map_err(io_error)?;
    Ok((size, format!("{:x}", digest.finalize())))
}

fn optional_version(path: &Path) -> Result<Option<String>, String> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(Some(file_version(path)?.1)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(io_error(error)),
    }
}

fn check_version(current: Option<&str>, expected: &str) -> Result<(), String> {
    if current == Some(expected) || (current.is_none() && expected == "missing") {
        Ok(())
    } else {
        Err(
            "conflict: Content changed or target already exists; read again before retrying."
                .into(),
        )
    }
}

fn require_file(path: &Path) -> Result<(), String> {
    if fs::symlink_metadata(path)
        .map_err(io_error)?
        .file_type()
        .is_file()
    {
        Ok(())
    } else {
        Err("unsafe_target: Expected an ordinary file.".into())
    }
}

fn io_error(error: std::io::Error) -> String {
    format!("io_error: {:?}.", error.kind())
}

fn validate_name(name: &str) -> Result<(), String> {
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    if name.is_empty()
        || name.starts_with('.')
        || name.ends_with(['.', ' '])
        || name == "node_modules"
        || name
            .chars()
            .any(|c| c.is_control() || "/\\:*?\"<>|".contains(c))
        || matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit())
    {
        return Err(
            "invalid_name: Use a visible, portable library name without separators.".into(),
        );
    }
    Ok(())
}

fn uri_segments(uri: &str) -> Result<Vec<String>, String> {
    if uri == "locus://" {
        return Ok(Vec::new());
    }
    let path = crate::library_reference_path(uri)
        .ok_or("invalid_uri: Expected a valid locus:// address.")?;
    path.split('/')
        .map(|name| {
            validate_name(name)?;
            if crate::is_reserved_directory(name) {
                return Err("reserved_name: assets and canvas cannot be notes.".into());
            }
            Ok(name.to_owned())
        })
        .collect()
}

fn canonical_uri(segments: &[String]) -> String {
    const ESCAPE: &percent_encoding::AsciiSet = &percent_encoding::CONTROLS
        .add(b' ')
        .add(b'%')
        .add(b'#')
        .add(b'?');
    format!(
        "locus://{}",
        segments
            .iter()
            .map(|s| percent_encoding::utf8_percent_encode(s, ESCAPE).to_string())
            .collect::<Vec<_>>()
            .join("/")
    )
}

fn parent_uri(uri: &str) -> Result<(String, String), String> {
    let mut segments = uri_segments(uri)?;
    let name = segments
        .pop()
        .ok_or("invalid_uri: Cannot modify the library root.")?;
    Ok((canonical_uri(&segments), name))
}

fn resource_name(resource: &Resource) -> Result<String, String> {
    match resource {
        Resource::Note => Ok("index.md".into()),
        Resource::Canvas => Ok(crate::CANVAS_FILE.into()),
        Resource::Icon => Ok("assets/icon.png".into()),
        Resource::CanvasFile { name } => {
            if name.split('/').any(|part| part == "node_modules") {
                return Err("reserved_name: Installed dependencies are not canvas content.".into());
            }
            if name.eq_ignore_ascii_case("package.json") {
                return Err("reserved_name: Use the canvas resource for its entry.".into());
            }
            crate::canvas::validate_resource_path(name).map_err(io_error)?;
            Ok(format!("canvas/{name}"))
        }
        Resource::Attachment { name } => {
            validate_name(name)?;
            if name.eq_ignore_ascii_case("icon.png") {
                return Err("reserved_name: Use the icon resource.".into());
            }
            Ok(format!("assets/{name}"))
        }
    }
}

fn resource_for_name(name: &str) -> Resource {
    match name {
        "index.md" => Resource::Note,
        "canvas/package.json" => Resource::Canvas,
        "assets/icon.png" => Resource::Icon,
        _ => {
            if let Some(name) = name.strip_prefix("canvas/") {
                Resource::CanvasFile {
                    name: name.to_owned(),
                }
            } else {
                Resource::Attachment {
                    name: name.strip_prefix("assets/").unwrap_or(name).to_owned(),
                }
            }
        }
    }
}

fn collect_resources(
    directory: &Path,
    folder: &str,
    prefix: &str,
    files: &mut Vec<Value>,
) -> Result<(), String> {
    for entry in fs::read_dir(directory).map_err(io_error)? {
        let entry = entry.map_err(io_error)?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "invalid_name")?;
        let relative = format!("{prefix}{name}");
        if folder == "canvas" && name == "node_modules" {
            continue;
        }
        let kind = entry.file_type().map_err(io_error)?;
        if kind.is_dir() && folder == "canvas" {
            collect_resources(&entry.path(), folder, &format!("{relative}/"), files)?;
        } else if kind.is_file() && (folder == "canvas" || !name.starts_with('.')) {
            let name = format!("{folder}/{relative}");
            files.push(json!({"name": name, "resource": resource_for_name(&name), "size": entry.metadata().map_err(io_error)?.len()}));
        }
    }
    Ok(())
}
