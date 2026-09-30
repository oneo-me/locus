use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub const CANVAS_CONFIG_FILE: &str = "canvas/index.json";
pub const CANVAS_MAIN_FILE: &str = "canvas/main.js";
pub const CANVAS_FILE: &str = "canvas/index.html";
pub const ASSETS_DIRECTORY: &str = "assets";
pub const CANVAS_DIRECTORY: &str = "canvas";

pub fn is_reserved_directory(name: &str) -> bool {
    name.eq_ignore_ascii_case(ASSETS_DIRECTORY) || name.eq_ignore_ascii_case(CANVAS_DIRECTORY)
}

pub(crate) fn validate_resource_path(relative: &str) -> io::Result<()> {
    for part in relative.split('/') {
        if part.is_empty()
            || matches!(part, "." | ".." | "node_modules")
            || part.contains(['\\', ':', '\0'])
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid resource path",
            ));
        }
    }
    Ok(())
}

pub(crate) fn regular_path(
    base: &Path,
    relative: &str,
    create_parents: bool,
) -> io::Result<PathBuf> {
    validate_resource_path(relative)?;
    let mut path = base.to_path_buf();
    let parts: Vec<_> = relative.split('/').collect();
    for (index, part) in parts.iter().enumerate() {
        path.push(part);
        if index + 1 < parts.len() {
            if create_parents && !path.try_exists()? {
                fs::create_dir(&path)?;
            }
            match fs::symlink_metadata(&path) {
                Ok(metadata) if metadata.file_type().is_dir() => (),
                Err(error) if error.kind() == io::ErrorKind::NotFound && !create_parents => (),
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Resource parents must be ordinary directories",
                    ));
                }
            }
        }
    }
    Ok(path)
}

fn note_directory(root: &Path, uri: &str) -> io::Result<PathBuf> {
    let path = crate::library_reference_path(uri)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid note address"))?;
    if path.split('/').any(|name| {
        name.starts_with('.')
            || name.ends_with(['.', ' '])
            || name == "node_modules"
            || is_reserved_directory(name)
    }) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid note address",
        ));
    }
    let path = regular_path(&root.canonicalize()?, &format!("{path}/index.md"), false)?;
    require_regular_file(&path)?;
    Ok(path.parent().unwrap().to_path_buf())
}

#[derive(Debug)]
pub struct CanvasSource {
    directory: PathBuf,
    html: String,
    data: serde_json::Value,
}

impl CanvasSource {
    pub fn directory(&self) -> &Path {
        &self.directory
    }
    pub fn html(&self) -> &str {
        &self.html
    }
    pub fn data(&self) -> &serde_json::Value {
        &self.data
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CanvasConfig {
    template: String,
    data: serde_json::Value,
}

pub fn resolve_canvas(root: &Path, directory: &Path) -> io::Result<CanvasSource> {
    let directory = canvas_directory(root, directory)?;
    let html = regular_path(&directory, CANVAS_FILE, false)?;
    let config = regular_path(&directory, CANVAS_CONFIG_FILE, false)?;
    let has_html = fs::symlink_metadata(&html).is_ok();
    let has_config = fs::symlink_metadata(&config).is_ok();
    if has_html && has_config {
        return Err(io::Error::other(
            "Canvas index.html and index.json are mutually exclusive",
        ));
    }
    let (directory, data) = if has_config {
        require_regular_file(&config)?;
        let value: serde_json::Value = serde_json::from_slice(&fs::read(config)?)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        if value.get("data").is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Canvas configuration requires data",
            ));
        }
        let config: CanvasConfig = serde_json::from_value(value)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let template = note_directory(root, &config.template)?;
        if fs::symlink_metadata(regular_path(&template, CANVAS_CONFIG_FILE, false)?).is_ok() {
            return Err(io::Error::other(
                "A template must define index.html and main.js",
            ));
        }
        (template, config.data)
    } else {
        (directory, serde_json::Value::Null)
    };
    require_regular_file(&regular_path(&directory, CANVAS_MAIN_FILE, false)?)?;
    let html = load_canvas(root, &directory)?;
    if html.trim().is_empty() {
        return Err(io::Error::other("Canvas is empty"));
    }
    Ok(CanvasSource {
        directory,
        html,
        data,
    })
}

pub fn load_canvas(root: &Path, directory: &Path) -> io::Result<String> {
    let directory = canvas_directory(root, directory)?;
    let path = regular_path(&directory, CANVAS_FILE, false)?;
    require_regular_file(&path)?;
    fs::read_to_string(path)
}

pub fn load_canvas_resource(root: &Path, directory: &Path, name: &str) -> io::Result<Vec<u8>> {
    let directory = canvas_directory(root, directory)?;
    let path = regular_path(&directory, &format!("canvas/{name}"), false)?;
    require_regular_file(&path)?;
    fs::read(path)
}

pub fn load_library_resource(root: &Path, uri: &str) -> io::Result<Vec<u8>> {
    let (address, part) = uri
        .split_once('?')
        .map_or((uri, None), |(address, query)| (address, Some(query)));
    let path = crate::library_reference_path(address)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid resource address"))?;
    let parts: Vec<_> = path.split('/').collect();
    let boundary = parts
        .iter()
        .position(|name| is_reserved_directory(name) || *name == "index.md")
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Expected assets, canvas or index.md",
            )
        })?;
    let note = format!(
        "locus://{}",
        address
            .strip_prefix("locus://")
            .unwrap()
            .split('/')
            .take(boundary)
            .collect::<Vec<_>>()
            .join("/")
    );
    let directory = note_directory(root, &note)?;
    let relative = parts[boundary..].join("/");
    let file = regular_path(&directory, &relative, false)?;
    require_regular_file(&file)?;
    let bytes = fs::read(file)?;
    match part {
        None => Ok(bytes),
        Some("part=yaml" | "part=body") if relative == "index.md" => {
            let text = std::str::from_utf8(&bytes)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            let text = text.strip_prefix('\u{feff}').unwrap_or(text);
            let (yaml, body) = crate::scanner::frontmatter_parts(text).unwrap_or(("", text));
            Ok(if part == Some("part=yaml") {
                yaml
            } else {
                body
            }
            .as_bytes()
            .to_vec())
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid content part",
        )),
    }
}

fn canvas_directory(root: &Path, directory: &Path) -> io::Result<PathBuf> {
    let canonical_root = root.canonicalize()?;
    let relative = directory
        .strip_prefix(root)
        .or_else(|_| directory.strip_prefix(&canonical_root))
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Canvas is outside the library",
            )
        })?;
    let root = canonical_root;
    if relative.as_os_str().is_empty() {
        require_regular_file(&root.join("index.md"))?;
        return Ok(root);
    }
    let file = regular_path(
        &root,
        &format!("{}/index.md", relative.to_string_lossy()),
        false,
    )?;
    require_regular_file(&file)?;
    Ok(directory.to_path_buf())
}

fn require_regular_file(path: &Path) -> io::Result<()> {
    if !fs::symlink_metadata(path)?.file_type().is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Resources must be regular files",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    fn write_fixture(
        path: impl AsRef<std::path::Path>,
        contents: impl AsRef<[u8]>,
    ) -> std::io::Result<()> {
        let path = path.as_ref();
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(path, contents)
    }

    use super::*;

    #[test]
    fn resolves_json_to_template_with_exact_data_and_observes_edits() {
        let root = tempfile::tempdir().unwrap();
        let caller = root.path().join("caller");
        let template = root.path().join("模版");
        for directory in [&caller, &template] {
            write_fixture(directory.join("index.md"), "Note").unwrap();
        }
        write_fixture(template.join(CANVAS_FILE), "<p>Template</p>").unwrap();
        write_fixture(
            template.join(CANVAS_MAIN_FILE),
            "export async function main(data) {}",
        )
        .unwrap();
        write_fixture(caller.join(CANVAS_CONFIG_FILE), r#"{"template":"locus://%E6%A8%A1%E7%89%88","data":{"nested":[1,false,null,"</script>"]}}"#).unwrap();
        let canvas = resolve_canvas(root.path(), &caller).unwrap();
        assert_eq!(canvas.directory(), template.canonicalize().unwrap());
        assert_eq!(
            canvas.data(),
            &serde_json::json!({"nested":[1,false,null,"</script>"]})
        );
        assert_eq!(canvas.html(), "<p>Template</p>");
        assert!(
            resolve_canvas(root.path(), &template)
                .unwrap()
                .data()
                .is_null()
        );
        write_fixture(template.join(CANVAS_FILE), "<p>Updated</p>").unwrap();
        assert_eq!(
            resolve_canvas(root.path(), &caller).unwrap().html(),
            "<p>Updated</p>"
        );
        let tree = crate::LibraryTree::load(Some(root.path()));
        assert_eq!(
            tree.node("caller").unwrap().canvas_path.as_ref().unwrap(),
            &caller.join(CANVAS_CONFIG_FILE)
        );
    }

    #[test]
    fn invalid_entries_fail_without_falling_back_or_following_template_chains() {
        let root = tempfile::tempdir().unwrap();
        let note = root.path().join("note");
        write_fixture(note.join("index.md"), "Note").unwrap();
        write_fixture(note.join(CANVAS_FILE), "<p>Canvas</p>").unwrap();
        assert!(resolve_canvas(root.path(), &note).is_err());
        write_fixture(
            note.join(CANVAS_MAIN_FILE),
            "export async function main(data) {}",
        )
        .unwrap();
        write_fixture(
            note.join(CANVAS_CONFIG_FILE),
            r#"{"template":"locus://note","data":{}}"#,
        )
        .unwrap();
        assert!(
            resolve_canvas(root.path(), &note)
                .unwrap_err()
                .to_string()
                .contains("mutually exclusive")
        );
        fs::remove_file(note.join(CANVAS_FILE)).unwrap();
        for json in [
            r#"{"template":"locus://note","data":{}}"#,
            r#"{"template":"locus://note"}"#,
            r#"{"tempalte":"locus://note","date":{}}"#,
            r#"{"template":"locus://missing","data":{}}"#,
            r#"{"template":"file:///note","data":{}}"#,
            r#"{"template":"locus://../note","data":{}}"#,
            r#"{"template":"locus://note","data":{},"extra":true}"#,
            "null",
            "[]",
            "{",
        ] {
            write_fixture(note.join(CANVAS_CONFIG_FILE), json).unwrap();
            assert!(resolve_canvas(root.path(), &note).is_err(), "{json}");
        }
    }

    #[test]
    fn templates_resolve_only_explicit_library_note_addresses_and_read_fresh() {
        let root = tempfile::tempdir().unwrap();
        let note = root.path().join("旅行");
        fs::create_dir(&note).unwrap();
        write_fixture(note.join("index.md"), "Travel").unwrap();
        write_fixture(note.join(CANVAS_FILE), "<p>Template</p>").unwrap();
        for uri in ["locus://旅行", "locus://%E6%97%85%E8%A1%8C"] {
            assert_eq!(
                note_directory(root.path(), uri)
                    .and_then(|directory| load_canvas(root.path(), &directory))
                    .unwrap(),
                "<p>Template</p>"
            );
        }
        write_fixture(note.join(CANVAS_FILE), "Updated").unwrap();
        assert_eq!(
            note_directory(root.path(), "locus://旅行")
                .and_then(|directory| load_canvas(root.path(), &directory))
                .unwrap(),
            "Updated"
        );
        for uri in [
            "旅行",
            "locus://",
            "file:///旅行",
            "https://旅行",
            "locus:///旅行",
            "locus://旅行/",
            "locus://旅行/../旅行",
            "locus://旅行/%2e%2e/旅行",
            "locus://%2f旅行",
            "locus://旅行%5c",
            "locus://%252e%252e",
            "locus://旅行?x",
            "locus://旅行#x",
            "locus://旅行/canvas.html",
            "locus://旅行/index.md",
            "locus://.hidden",
            "locus://%FF",
            "locus://C:/旅行",
            "locus://旅行%00",
            "locus://旅行.",
            "locus://旅行%20",
        ] {
            assert!(
                note_directory(root.path(), uri)
                    .and_then(|directory| load_canvas(root.path(), &directory))
                    .is_err(),
                "{uri}"
            );
        }
        fs::remove_file(note.join("index.md")).unwrap();
        assert!(
            note_directory(root.path(), "locus://旅行")
                .and_then(|directory| load_canvas(root.path(), &directory))
                .is_err()
        );
    }

    #[cfg(unix)]
    #[test]
    fn templates_reject_symlinked_ancestors_and_files() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        for path in [root.path(), outside.path()] {
            write_fixture(path.join("index.md"), "Note").unwrap();
            write_fixture(path.join(CANVAS_FILE), "Template").unwrap();
        }
        symlink(outside.path(), root.path().join("outside")).unwrap();
        symlink(root.path(), root.path().join("inside")).unwrap();
        for uri in ["locus://outside", "locus://inside", "locus://inside/inside"] {
            assert!(
                note_directory(root.path(), uri)
                    .and_then(|directory| load_canvas(root.path(), &directory))
                    .is_err()
            );
        }
        let note = root.path().join("note");
        fs::create_dir(&note).unwrap();
        write_fixture(note.join("index.md"), "Note").unwrap();
        fs::create_dir(note.join("canvas")).unwrap();
        symlink(outside.path().join(CANVAS_FILE), note.join(CANVAS_FILE)).unwrap();
        assert!(
            note_directory(root.path(), "locus://note")
                .and_then(|directory| load_canvas(root.path(), &directory))
                .is_err()
        );
    }

    #[test]
    fn resources_belong_to_the_original_note_and_are_read_fresh() {
        let root = tempfile::tempdir().unwrap();
        let note = root.path().join("note");
        fs::create_dir(&note).unwrap();
        write_fixture(note.join("index.md"), "Note").unwrap();
        write_fixture(note.join("canvas/照片 1.png"), [0, 128, 255]).unwrap();
        assert_eq!(
            load_canvas_resource(root.path(), &note, "照片 1.png").unwrap(),
            [0, 128, 255]
        );
        write_fixture(note.join("canvas/照片 1.png"), [42]).unwrap();
        assert_eq!(
            load_canvas_resource(root.path(), &note, "照片 1.png").unwrap(),
            [42]
        );
        for name in [
            "",
            ".",
            "..",
            "../note/照片 1.png",
            "sub/photo.png",
            "/photo.png",
            "C:\\photo.png",
            "photo.png:stream",
            ".secret",
            "index.md",
            "INDEX.MD",
            "index.md.",
            "index.md ",
            "canvas.html",
            "missing.png",
        ] {
            assert!(
                load_canvas_resource(root.path(), &note, name).is_err(),
                "{name}"
            );
        }
        let outside = tempfile::tempdir().unwrap();
        write_fixture(outside.path().join("index.md"), "Outside").unwrap();
        write_fixture(outside.path().join("canvas/photo.png"), [1]).unwrap();
        assert!(load_canvas_resource(root.path(), outside.path(), "photo.png").is_err());
        fs::remove_file(note.join("index.md")).unwrap();
        assert!(load_canvas_resource(root.path(), &note, "照片 1.png").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn resources_reject_symlinks_and_directories() {
        let root = tempfile::tempdir().unwrap();
        write_fixture(root.path().join("index.md"), "Note").unwrap();
        write_fixture(root.path().join("canvas/photo.png"), [1]).unwrap();
        std::os::unix::fs::symlink(
            root.path().join("canvas/photo.png"),
            root.path().join("canvas/link.png"),
        )
        .unwrap();
        fs::create_dir(root.path().join("canvas/folder.png")).unwrap();
        for name in ["link.png", "folder.png"] {
            assert!(load_canvas_resource(root.path(), root.path(), name).is_err());
        }
    }
}
