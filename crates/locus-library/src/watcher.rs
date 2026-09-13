use std::path::{Component, Path, PathBuf};

use notify::{EventKind, RecursiveMode, Watcher};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LibraryWatchEvent {
    Changed {
        paths: Vec<PathBuf>,
        rescan_all: bool,
    },
    Error(String),
}

pub struct LibraryWatcher {
    watcher: notify::RecommendedWatcher,
    root: PathBuf,
}

impl LibraryWatcher {
    pub fn start(
        root: &Path,
        on_event: impl Fn(LibraryWatchEvent) + Send + 'static,
    ) -> Result<Self, String> {
        let watched_root = root.canonicalize().map_err(|error| error.to_string())?;
        let event_root = watched_root.clone();
        let mut watcher = notify::recommended_watcher(
            move |result: notify::Result<notify::Event>| match result {
                Ok(event) => {
                    if matches!(event.kind, EventKind::Access(_)) {
                        return;
                    }

                    let rescan_all = event.paths.is_empty();
                    let paths = event
                        .paths
                        .iter()
                        .filter_map(|path| relevant_relative_path(&event_root, path))
                        .collect::<Vec<_>>();
                    if rescan_all || !paths.is_empty() {
                        on_event(LibraryWatchEvent::Changed { paths, rescan_all });
                    }
                }
                Err(error) => on_event(LibraryWatchEvent::Error(error.to_string())),
            },
        )
        .map_err(|error| error.to_string())?;
        watcher
            .watch(&watched_root, RecursiveMode::Recursive)
            .map_err(|error| error.to_string())?;
        Ok(Self {
            watcher,
            root: watched_root,
        })
    }
}

impl Drop for LibraryWatcher {
    fn drop(&mut self) {
        let _ = self.watcher.unwatch(&self.root);
    }
}

fn relevant_relative_path(root: &Path, path: &Path) -> Option<PathBuf> {
    let Ok(relative) = path.strip_prefix(root) else {
        return None;
    };
    if relative.as_os_str().is_empty() {
        return Some(PathBuf::new());
    }

    for (index, component) in relative.components().enumerate() {
        let Component::Normal(component) = component else {
            continue;
        };
        let name = component.to_string_lossy();
        if index == 0 && name == ".git" {
            return Some(relative.to_path_buf());
        }
        if name == "canvas" {
            return Some(relative.to_path_buf());
        }
        if name.starts_with('.') || name == "node_modules" {
            return None;
        }
    }
    Some(relative.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_paths_outside_the_library_projection() {
        let root = Path::new("/library");
        assert_eq!(
            relevant_relative_path(root, Path::new("/library/notes/index.md")),
            Some(PathBuf::from("notes/index.md"))
        );
        assert_eq!(
            relevant_relative_path(root, Path::new("/library/.git/index")),
            Some(PathBuf::from(".git/index"))
        );
        assert_eq!(
            relevant_relative_path(root, Path::new("/library/.cache/value")),
            None
        );
        assert_eq!(
            relevant_relative_path(root, Path::new("/library/node_modules/pkg/index.md")),
            None
        );
        assert_eq!(
            relevant_relative_path(root, Path::new("/other/index.md")),
            None
        );
    }
}
