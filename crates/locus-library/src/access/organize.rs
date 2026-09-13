use super::*;
use pulldown_cmark::{Event, LinkType, Options, Parser, Tag};

impl LibraryAccess {
    pub fn export_manifest(&self, uri: &str) -> Result<Value, String> {
        let mut directories = vec![uri.to_owned()];
        directories.extend(self.all_directories(uri)?);
        let mut entries = Vec::new();
        for directory in directories {
            let path = self.directory(&directory)?;
            entries.push(json!({"uri": directory, "kind": "directory"}));
            if uri_segments(&directory)?.is_empty()
                || optional_version(&path.join("index.md"))?.is_none()
            {
                continue;
            }
            for file in self.describe(&directory)?["files"]
                .as_array()
                .ok_or("invalid_manifest")?
            {
                let resource: Resource = serde_json::from_value(file["resource"].clone())
                    .map_err(|_| "invalid_resource")?;
                entries.push(self.stat(&ContentTarget::new(&directory, resource))?);
            }
        }
        entries.sort_by_key(Value::to_string);
        let version = content_version(
            serde_json::to_string(&entries)
                .map_err(|_| "encoding_error")?
                .as_bytes(),
        );
        Ok(json!({"uri": uri, "version": version, "entries": entries}))
    }

    fn tree_version(&self, uri: &str) -> Result<String, String> {
        let root = self.directory(uri)?;
        let mut pending = vec![root.clone()];
        let mut entries = Vec::new();
        while let Some(path) = pending.pop() {
            for entry in fs::read_dir(&path).map_err(io_error)? {
                let entry = entry.map_err(io_error)?;
                let kind = entry.file_type().map_err(io_error)?;
                let name = entry
                    .file_name()
                    .into_string()
                    .map_err(|_| "invalid_name".to_owned())?;
                let in_canvas = path
                    .strip_prefix(&root)
                    .map_err(|_| "invalid_target")?
                    .components()
                    .any(|part| part.as_os_str() == "canvas");
                if (!in_canvas && (name.starts_with('.') || name == "node_modules"))
                    || !(kind.is_dir() || kind.is_file())
                {
                    return Err("unmanaged_content: Directory contains hidden, excluded or non-regular entries; move or remove them explicitly outside MCP before changing this tree.".into());
                }
                let relative = entry
                    .path()
                    .strip_prefix(&root)
                    .map_err(|_| "invalid_target")?
                    .to_string_lossy()
                    .into_owned();
                if kind.is_dir() {
                    entries.push(json!([relative, "directory"]));
                    pending.push(entry.path());
                } else {
                    entries.push(json!([relative, file_version(&entry.path())?.1]));
                }
            }
        }
        entries.sort_by_key(Value::to_string);
        Ok(content_version(
            serde_json::to_string(&entries)
                .map_err(|_| "encoding_error")?
                .as_bytes(),
        ))
    }

    pub fn inspect_tree(&self, uri: &str) -> Result<Value, String> {
        parent_uri(uri)?;
        let mut manifest = self.export_manifest(uri)?;
        manifest["version"] = json!(self.tree_version(uri)?);
        Ok(manifest)
    }

    pub fn delete_tree(&self, uri: &str, expected: &str, recursive: bool) -> Result<Value, String> {
        parent_uri(uri)?;
        let directory = self.directory(uri)?;
        check_version(Some(&self.tree_version(uri)?), expected)?;
        if recursive {
            fs::remove_dir_all(directory).map_err(|error| {
                format!(
                    "partial_failure: Deletion failed; inspect remaining content. {}",
                    io_error(error)
                )
            })?;
        } else {
            fs::remove_dir(directory).map_err(io_error)?;
        }
        Ok(json!({"deleted": uri, "recursive": recursive}))
    }

    pub fn plan_move(&self, uri: &str, destination: &str) -> Result<Value, String> {
        self.prepare_move(uri, destination).map(|(plan, _)| plan)
    }

    fn prepare_move(&self, uri: &str, destination: &str) -> Result<(Value, Vec<LinkEdit>), String> {
        let source_segments = uri_segments(uri)?;
        let destination_segments = uri_segments(destination)?;
        parent_uri(uri)?;
        let (parent, name) = parent_uri(destination)?;
        let destination_path = self.directory(&parent)?.join(name);
        if destination_segments.starts_with(&source_segments) {
            return Err("invalid_move: Destination is inside source.".into());
        }
        match fs::symlink_metadata(&destination_path) {
            Ok(_) => return Err("already_exists: Destination must not exist.".into()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(io_error(error)),
        }
        let version = self.tree_version(uri)?;
        let edits = self.link_edits(&source_segments, &destination_segments)?;
        let references = edits.iter().map(|edit| json!({"uri": edit.uri, "destinationUri": edit.destination_uri, "version": edit.version})).collect::<Vec<_>>();
        let token = content_version(
            json!([uri, destination, version, references])
                .to_string()
                .as_bytes(),
        );
        Ok((
            json!({"uri": uri, "destination": destination, "version": token, "treeVersion": version, "updatedNotes": references,
            "manualReview": "Canvas HTML, scripts, custom metadata and dynamically constructed references are preserved; review them with read_text/patch_text after moving."}),
            edits,
        ))
    }

    pub fn move_tree(&self, uri: &str, destination: &str, expected: &str) -> Result<Value, String> {
        let (plan, edits) = self.prepare_move(uri, destination)?;
        check_version(plan["version"].as_str(), expected)?;
        let source = self.directory(uri)?;
        let (parent, name) = parent_uri(destination)?;
        let destination_path = self.directory(&parent)?.join(name);
        check_version(
            Some(&self.tree_version(uri)?),
            plan["treeVersion"].as_str().ok_or("invalid_plan")?,
        )?;
        rename_directory(&source, &destination_path)?;
        if let Err(error) = self.apply_link_edits(&edits) {
            if rename_directory(&destination_path, &source).is_err() {
                return Err("partial_failure: Move rollback was incomplete; inspect both logical addresses and affected notes.".into());
            }
            return Err(error);
        }
        Ok(plan)
    }

    pub fn move_attachment(
        &self,
        source: &ContentTarget,
        destination: &ContentTarget,
        expected: &str,
    ) -> Result<Value, String> {
        if !matches!(source.resource, Resource::Attachment { .. })
            || !matches!(destination.resource, Resource::Attachment { .. })
        {
            return Err("invalid_resource: Only attachments can be moved independently.".into());
        }
        let source_path = self.target_path(source)?;
        let destination_path = self.target_path(destination)?;
        check_version(optional_version(&source_path)?.as_deref(), expected)?;
        check_version(optional_version(&destination_path)?.as_deref(), "missing")?;
        let mut source_segments = uri_segments(&source.uri)?;
        source_segments.extend(
            resource_name(&source.resource)?
                .split('/')
                .map(str::to_owned),
        );
        let mut destination_segments = uri_segments(&destination.uri)?;
        destination_segments.extend(
            resource_name(&destination.resource)?
                .split('/')
                .map(str::to_owned),
        );
        let edits = self.link_edits(&source_segments, &destination_segments)?;
        check_version(optional_version(&source_path)?.as_deref(), expected)?;
        self.prepare_resource_parents(destination)?;
        fs::hard_link(&source_path, &destination_path).map_err(io_error)?;
        if let Err(error) = fs::remove_file(&source_path) {
            fs::remove_file(&destination_path).map_err(io_error)?;
            return Err(io_error(error));
        }
        if let Err(error) = self.apply_link_edits(&edits) {
            if fs::hard_link(&destination_path, &source_path).is_err()
                || fs::remove_file(&destination_path).is_err()
            {
                return Err("partial_failure: Attachment move rollback failed; inspect both resource targets and affected notes.".into());
            }
            return Err(error);
        }
        let references = edits.iter().map(|edit| json!({"uri": edit.uri, "version": content_version(edit.updated.as_bytes())})).collect::<Vec<_>>();
        Ok(
            json!({"content": self.stat(destination)?, "previousTarget": source, "updatedNotes": references,
            "manualReview": "Canvas HTML, scripts and custom metadata are preserved; review and patch their attachment references explicitly."}),
        )
    }

    fn apply_link_edits(&self, edits: &[LinkEdit]) -> Result<(), String> {
        let mut applied: Vec<&LinkEdit> = Vec::new();
        for edit in edits {
            let target = ContentTarget::new(&edit.destination_uri, Resource::Note);
            if let Err(error) = self.write_text(&target, &edit.updated, &edit.version) {
                let mut rollback_failed = false;
                for previous in applied.into_iter().rev() {
                    let target = ContentTarget::new(&previous.destination_uri, Resource::Note);
                    if self
                        .write_text(
                            &target,
                            &previous.original,
                            &content_version(previous.updated.as_bytes()),
                        )
                        .is_err()
                    {
                        rollback_failed = true;
                    }
                }
                return Err(if rollback_failed {
                    "partial_failure: Link rollback failed; inspect affected notes.".into()
                } else {
                    error
                });
            }
            applied.push(edit);
        }
        Ok(())
    }

    fn link_edits(
        &self,
        source: &[String],
        destination: &[String],
    ) -> Result<Vec<LinkEdit>, String> {
        let mut edits = Vec::new();
        for uri in self.all_directories("locus://")? {
            let path = self.directory(&uri)?.join("index.md");
            if optional_version(&path)?.is_none() {
                continue;
            }
            let (original, version) = self.read_text(&ContentTarget::new(&uri, Resource::Note))?;
            let owner = uri_segments(&uri)?;
            let moved_owner = remap(&owner, source, destination);
            let updated = rewrite_links(&original, &owner, &moved_owner, source, destination);
            if updated != original {
                edits.push(LinkEdit {
                    uri,
                    destination_uri: canonical_uri(&moved_owner),
                    original,
                    updated,
                    version,
                });
            }
        }
        Ok(edits)
    }
}

struct LinkEdit {
    uri: String,
    destination_uri: String,
    original: String,
    updated: String,
    version: String,
}

fn remap(path: &[String], source: &[String], destination: &[String]) -> Vec<String> {
    if let Some(suffix) = path.strip_prefix(source) {
        let mut result = destination.to_vec();
        result.extend_from_slice(suffix);
        result
    } else {
        path.to_vec()
    }
}

fn rewrite_links(
    content: &str,
    owner: &[String],
    moved_owner: &[String],
    source: &[String],
    destination: &[String],
) -> String {
    let body = crate::scanner::frontmatter_parts(content.trim_start_matches('\u{feff}'))
        .map_or(content, |(_, body)| body);
    let body_offset = content.len() - body.len();
    let parser = Parser::new_ext(body, Options::all());
    let mut links = Vec::new();
    for (_, definition) in parser.reference_definitions().iter() {
        if let Some(span) = destination_span(body, &definition.dest) {
            links.push((definition.dest.to_string(), span));
        }
    }
    for event in parser {
        if let Event::Start(
            Tag::Link {
                link_type: LinkType::Inline | LinkType::Autolink,
                dest_url,
                ..
            }
            | Tag::Image {
                link_type: LinkType::Inline,
                dest_url,
                ..
            },
        ) = event
            && let Some(span) = destination_span(body, &dest_url)
        {
            links.push((dest_url.to_string(), span));
        }
    }
    let mut replacements = Vec::new();
    for (url, range) in links {
        let Some(updated) = rewrite_url(&url, owner, moved_owner, source, destination) else {
            continue;
        };
        replacements.push((body_offset + range.start, range.len(), updated));
    }

    replacements.sort_by_key(|r| r.0);
    replacements.dedup_by_key(|r| r.0);
    let mut updated = content.to_owned();
    for (start, len, replacement) in replacements.into_iter().rev() {
        updated.replace_range(start..start + len, &replacement);
    }
    updated
}

fn destination_span(content: &str, url: &str) -> Option<std::ops::Range<usize>> {
    let start = (url.as_ptr() as usize).checked_sub(content.as_ptr() as usize)?;
    let end = start.checked_add(url.len())?;
    (content.get(start..end) == Some(url)).then_some(start..end)
}

fn rewrite_url(
    url: &str,
    owner: &[String],
    moved_owner: &[String],
    source: &[String],
    destination: &[String],
) -> Option<String> {
    let (address, suffix) = url
        .find(['#', '?'])
        .map_or((url, ""), |at| (&url[..at], &url[at..]));
    if address.starts_with("locus://") {
        let original = crate::library_reference_path(address)?
            .split('/')
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let mapped = remap(&original, source, destination);
        return (mapped != original).then(|| format!("{}{suffix}", canonical_uri(&mapped)));
    }
    if address.is_empty() || address.starts_with('/') || address.contains(':') {
        return None;
    }
    let mut resolved = owner.to_vec();
    for part in address.split('/') {
        let name = percent_encoding::percent_decode_str(part)
            .decode_utf8()
            .ok()?;
        match name.as_ref() {
            ".." => {
                resolved.pop()?;
            }
            "." | "" => {}
            _ => {
                validate_name(&name).ok()?;
                resolved.push(name.into_owned());
            }
        }
    }
    let mapped = remap(&resolved, source, destination);
    if mapped == resolved && owner == moved_owner {
        return None;
    }
    let common = mapped
        .iter()
        .zip(moved_owner)
        .take_while(|(a, b)| a == b)
        .count();
    let mut parts = vec!["..".to_owned(); moved_owner.len() - common];
    parts.extend_from_slice(&mapped[common..]);
    let relative = canonical_uri(&parts)
        .trim_start_matches("locus://")
        .to_owned();
    Some(format!(
        "{}{suffix}",
        if relative.is_empty() { "." } else { &relative }
    ))
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn rename_directory(source: &Path, destination: &Path) -> Result<(), String> {
    use std::os::unix::ffi::OsStrExt;
    let source =
        std::ffi::CString::new(source.as_os_str().as_bytes()).map_err(|_| "invalid_target")?;
    let destination =
        std::ffi::CString::new(destination.as_os_str().as_bytes()).map_err(|_| "invalid_target")?;
    #[cfg(target_os = "macos")]
    let result =
        unsafe { libc::renamex_np(source.as_ptr(), destination.as_ptr(), libc::RENAME_EXCL) };
    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            destination.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(io_error(std::io::Error::last_os_error()))
    }
}

#[cfg(target_os = "windows")]
fn rename_directory(source: &Path, destination: &Path) -> Result<(), String> {
    fs::rename(source, destination).map_err(io_error)
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn rename_directory(_: &Path, _: &Path) -> Result<(), String> {
    Err("unsupported_platform: Exclusive directory moves are unavailable.".into())
}
