use std::{fs, io, path::Path};

pub const CANVAS_FILE: &str = "canvas.html";

pub fn load_canvas_template(root: &Path, uri: &str) -> io::Result<String> {
    let invalid = || {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid canvas template address",
        )
    };
    let path = crate::library_reference_path(uri).ok_or_else(invalid)?;
    let mut directory = root.canonicalize()?;
    for name in path.split('/') {
        if name.is_empty()
            || name.starts_with('.')
            || name.ends_with(['.', ' '])
            || name.contains(['/', '\\', ':', '\0'])
            || name == "node_modules"
        {
            return Err(invalid());
        }
        directory.push(name);
        if !fs::symlink_metadata(&directory)?.file_type().is_dir() {
            return Err(invalid());
        }
    }
    load_canvas(root, &directory)
}

pub fn load_canvas(root: &Path, directory: &Path) -> io::Result<String> {
    let directory = canvas_directory(root, directory)?;
    let path = directory.join(CANVAS_FILE);
    require_regular_file(&path)?;
    fs::read_to_string(path)
}

pub fn load_canvas_resource(root: &Path, directory: &Path, name: &str) -> io::Result<Vec<u8>> {
    if name.is_empty()
        || name.starts_with('.')
        || name.ends_with(['.', ' '])
        || name.contains(['/', '\\', ':', '\0'])
        || name.eq_ignore_ascii_case("index.md")
        || name.eq_ignore_ascii_case(CANVAS_FILE)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid canvas resource",
        ));
    }
    let directory = canvas_directory(root, directory)?;
    let path = directory.join(name);
    require_regular_file(&path)?;
    fs::read(path)
}

fn canvas_directory(root: &Path, directory: &Path) -> io::Result<std::path::PathBuf> {
    let root = root.canonicalize()?;
    let directory = directory.canonicalize()?;
    if !directory.starts_with(&root) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Canvas is outside the library",
        ));
    }
    require_regular_file(&directory.join("index.md"))?;
    Ok(directory)
}

fn require_regular_file(path: &Path) -> io::Result<()> {
    if !fs::symlink_metadata(path)?.file_type().is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Canvas files must be regular files",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_resolve_only_explicit_library_note_addresses_and_read_fresh() {
        let root = tempfile::tempdir().unwrap();
        let note = root.path().join("旅行");
        fs::create_dir(&note).unwrap();
        fs::write(note.join("index.md"), "Travel").unwrap();
        fs::write(note.join(CANVAS_FILE), "<p>Template</p>").unwrap();
        for uri in ["locus://旅行", "locus://%E6%97%85%E8%A1%8C"] {
            assert_eq!(
                load_canvas_template(root.path(), uri).unwrap(),
                "<p>Template</p>"
            );
        }
        fs::write(note.join(CANVAS_FILE), "Updated").unwrap();
        assert_eq!(
            load_canvas_template(root.path(), "locus://旅行").unwrap(),
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
            assert!(load_canvas_template(root.path(), uri).is_err(), "{uri}");
        }
        fs::remove_file(note.join("index.md")).unwrap();
        assert!(load_canvas_template(root.path(), "locus://旅行").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn templates_reject_symlinked_ancestors_and_files() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        for path in [root.path(), outside.path()] {
            fs::write(path.join("index.md"), "Note").unwrap();
            fs::write(path.join(CANVAS_FILE), "Template").unwrap();
        }
        symlink(outside.path(), root.path().join("outside")).unwrap();
        symlink(root.path(), root.path().join("inside")).unwrap();
        for uri in ["locus://outside", "locus://inside", "locus://inside/inside"] {
            assert!(load_canvas_template(root.path(), uri).is_err());
        }
        let note = root.path().join("note");
        fs::create_dir(&note).unwrap();
        fs::write(note.join("index.md"), "Note").unwrap();
        symlink(outside.path().join(CANVAS_FILE), note.join(CANVAS_FILE)).unwrap();
        assert!(load_canvas_template(root.path(), "locus://note").is_err());
    }

    #[test]
    fn resources_belong_to_the_original_note_and_are_read_fresh() {
        let root = tempfile::tempdir().unwrap();
        let note = root.path().join("note");
        fs::create_dir(&note).unwrap();
        fs::write(note.join("index.md"), "Note").unwrap();
        fs::write(note.join("照片 1.png"), [0, 128, 255]).unwrap();
        assert_eq!(
            load_canvas_resource(root.path(), &note, "照片 1.png").unwrap(),
            [0, 128, 255]
        );
        fs::write(note.join("照片 1.png"), [42]).unwrap();
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
        fs::write(outside.path().join("index.md"), "Outside").unwrap();
        fs::write(outside.path().join("photo.png"), [1]).unwrap();
        assert!(load_canvas_resource(root.path(), outside.path(), "photo.png").is_err());
        fs::remove_file(note.join("index.md")).unwrap();
        assert!(load_canvas_resource(root.path(), &note, "照片 1.png").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn resources_reject_symlinks_and_directories() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("index.md"), "Note").unwrap();
        fs::write(root.path().join("photo.png"), [1]).unwrap();
        std::os::unix::fs::symlink(root.path().join("photo.png"), root.path().join("link.png"))
            .unwrap();
        fs::create_dir(root.path().join("folder.png")).unwrap();
        for name in ["link.png", "folder.png"] {
            assert!(load_canvas_resource(root.path(), root.path(), name).is_err());
        }
    }
}
