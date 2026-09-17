use std::{
    collections::BTreeMap,
    io::{self, Write},
    path::{Path, PathBuf},
};

pub struct CanvasDownload {
    files: BTreeMap<String, Vec<u8>>,
}

impl CanvasDownload {
    pub fn save(&self, parent: &Path, title: &str) -> io::Result<PathBuf> {
        if !parent.is_absolute() {
            return Err(io::Error::other("Export directory must be absolute"));
        }
        std::fs::create_dir_all(parent)?;
        let title = export_name(title);
        for number in 1.. {
            let name = if number == 1 {
                title.clone()
            } else {
                format!("{title} ({number})")
            };
            let directory = parent.join(name);
            match std::fs::create_dir(&directory) {
                Ok(()) => {
                    if let Err(error) = self.write_files(&directory) {
                        let _ = std::fs::remove_dir_all(&directory);
                        return Err(error);
                    }
                    return Ok(directory);
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        unreachable!()
    }

    fn write_files(&self, directory: &Path) -> io::Result<()> {
        if !directory.is_dir() {
            return Err(io::Error::other("Choose an existing directory"));
        }
        for name in self.files.keys() {
            match std::fs::symlink_metadata(directory.join(name)) {
                Ok(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::AlreadyExists,
                        format!("{name} already exists. Choose another directory."),
                    ));
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        let mut pending = Vec::new();
        for (name, bytes) in &self.files {
            let path = directory.join(name);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut file = tempfile::NamedTempFile::new_in(directory)?;
            file.write_all(bytes)?;
            file.as_file().sync_all()?;
            pending.push((file, directory.join(name)));
        }
        let mut created = Vec::new();
        for (file, path) in pending {
            match file.persist_noclobber(&path) {
                Ok(_) => created.push(path),
                Err(error) => {
                    for path in created {
                        if let Err(cleanup_error) = std::fs::remove_file(&path) {
                            return Err(io::Error::other(format!(
                                "{}; could not remove incomplete export {}: {cleanup_error}",
                                error.error,
                                path.display()
                            )));
                        }
                    }
                    return Err(error.error);
                }
            }
        }
        Ok(())
    }
}

fn export_name(title: &str) -> String {
    let title: String = title
        .chars()
        .take(60)
        .map(|ch| {
            if ch.is_control() || "<>:\"/\\|?*".contains(ch) {
                '_'
            } else {
                ch
            }
        })
        .collect();
    let title = title.trim_matches(['.', ' ']);
    if title.is_empty() {
        return "canvas".to_owned();
    }
    let stem = title
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    if matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || (stem.starts_with("COM") || stem.starts_with("LPT"))
        && stem.len() == 4
        && matches!(stem.as_bytes()[3], b'1'..=b'9')
    {
        return format!("_{title}");
    }
    title.to_owned()
}

pub(crate) fn prepare_download(
    runtime: &super::CanvasRuntime,
    directory: &Path,
    dark: bool,
) -> io::Result<CanvasDownload> {
    let output = tempfile::tempdir()?;
    runtime.export(directory, output.path(), dark)?;
    let mut files = BTreeMap::new();
    collect_files(output.path(), output.path(), &mut files)?;
    if !files.contains_key("canvas.html") {
        return Err(io::Error::other("Canvas build did not produce canvas.html"));
    }
    Ok(CanvasDownload { files })
}

fn collect_files(
    root: &Path,
    directory: &Path,
    files: &mut BTreeMap<String, Vec<u8>>,
) -> io::Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let file = entry.path();
        if entry.file_type()?.is_dir() {
            collect_files(root, &file, files)?;
        } else if entry.file_type()?.is_file() {
            let name = file
                .strip_prefix(root)
                .map_err(io::Error::other)?
                .to_string_lossy()
                .replace('\\', "/");
            files.insert(name, std::fs::read(file)?);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exports_do_not_overwrite_existing_files_or_directories() {
        let root = tempfile::tempdir().unwrap();
        let files = BTreeMap::from([
            ("canvas.html".into(), b"<html></html>".to_vec()),
            ("assets/main.js".into(), b"export {};".to_vec()),
        ]);
        let download = CanvasDownload { files };
        let first = download.save(root.path(), "Canvas").unwrap();
        let second = download.save(root.path(), "Canvas").unwrap();
        assert_ne!(first, second);
        assert_eq!(
            std::fs::read(first.join("assets/main.js")).unwrap(),
            b"export {};"
        );
        assert!(second.join("canvas.html").is_file());
    }
}
