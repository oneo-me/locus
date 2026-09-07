use std::{fs, io, path::Path};

pub const CANVAS_FILE: &str = "canvas.html";

pub fn load_canvas(root: &Path, directory: &Path) -> io::Result<String> {
    let root = root.canonicalize()?;
    let directory = directory.canonicalize()?;
    if !directory.starts_with(&root) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Canvas is outside the library",
        ));
    }
    for path in [directory.join("index.md"), directory.join(CANVAS_FILE)] {
        if !fs::symlink_metadata(&path)?.file_type().is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Note and canvas must be regular files",
            ));
        }
    }
    fs::read_to_string(directory.join(CANVAS_FILE))
}
