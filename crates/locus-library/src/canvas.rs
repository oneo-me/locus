use std::{fs, io, path::Path};

pub const CANVAS_FILE: &str = "canvas.html";

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
