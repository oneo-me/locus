mod process_environment;
pub use process_environment::command as background_command;

use std::path::{Path, PathBuf};
use std::time::SystemTime;

mod access;
mod canvas;
mod mutations;
mod reference;
mod repository;
mod scanner;
mod watcher;

pub use access::{ContentTarget, LibraryAccess, Resource, content_version};
pub use canvas::{
    ASSETS_DIRECTORY, CANVAS_DIRECTORY, CANVAS_FILE, is_reserved_directory, load_canvas,
    load_canvas_resource, load_library_resource,
};
pub use mutations::{archive_note, restore_archived_note, update_note_icon};
pub use reference::{LIBRARY_REFERENCE_SCHEME, library_reference_path};
pub use scanner::load_note;
pub use watcher::{LibraryWatchEvent, LibraryWatcher};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ChangeStatus {
    Added,
    Modified,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DocumentIcon {
    pub path: PathBuf,
    pub modified: Option<SystemTime>,
    pub byte_len: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibraryNode {
    pub path: String,
    pub directory: PathBuf,
    pub document_path: Option<PathBuf>,
    pub canvas_path: Option<PathBuf>,
    pub title: String,
    pub icon: Option<String>,
    pub icon_image: Option<DocumentIcon>,
    pub summary: Option<String>,
    pub additional_metadata: Vec<(String, serde_json::Value)>,
    pub attachments: Vec<Attachment>,
    pub change: Option<ChangeStatus>,
    pub descendant_change: Option<ChangeStatus>,
    pub children: Vec<LibraryNode>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchivedNote {
    pub path: String,
    pub document_path: PathBuf,
    pub title: String,
    pub icon: Option<String>,
    pub icon_image: Option<DocumentIcon>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Attachment {
    pub name: String,
    pub path: PathBuf,
    pub modified: Option<SystemTime>,
    pub byte_len: u64,
    pub image_size: Option<ImageSize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImageSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LibraryTree {
    pub nodes: Vec<LibraryNode>,
    pub archived_notes: Vec<ArchivedNote>,
    pub error: Option<String>,
    pub repository_error: Option<String>,
}

impl LibraryTree {
    pub fn load(root: Option<&Path>) -> Self {
        let Some(root) = root else {
            return Self::default();
        };
        if !root.is_dir() {
            return Self {
                error: Some("文档库目录不存在或无法访问。".to_owned()),
                ..Self::default()
            };
        }

        let (changes, repository_error) = match repository::read_changes(root) {
            Ok(changes) => (changes, None),
            Err(error) => (Vec::new(), Some(error)),
        };
        let mut archived_notes = Vec::new();
        match scanner::scan_children(root, root, &changes, false, &mut archived_notes) {
            Ok(mut nodes) => {
                nodes.sort_by(scanner::compare_nodes);
                archived_notes.sort_by(|left, right| left.path.cmp(&right.path));
                Self {
                    nodes,
                    archived_notes,
                    error: None,
                    repository_error,
                }
            }
            Err(error) => Self {
                error: Some(format!("无法读取文档库：{error}")),
                repository_error,
                ..Self::default()
            },
        }
    }

    pub fn node(&self, path: &str) -> Option<&LibraryNode> {
        find_node(&self.nodes, path)
    }
}

fn find_node<'a>(nodes: &'a [LibraryNode], path: &str) -> Option<&'a LibraryNode> {
    for node in nodes {
        if node.path == path {
            return Some(node);
        }
        if let Some(node) = find_node(&node.children, path) {
            return Some(node);
        }
    }
    None
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

    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestLibrary(PathBuf);

    impl TestLibrary {
        fn new() -> Self {
            let id = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "locus-document-library-{}-{id}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestLibrary {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn canvas_is_discovered_without_being_read_or_counted_as_an_attachment() {
        let library = TestLibrary::new();
        let directory = library.0.join("note");
        fs::create_dir(&directory).unwrap();
        write_fixture(directory.join("index.md"), "---\ntitle: Note\n---\nBody").unwrap();
        write_fixture(directory.join("assets/file.txt"), "attachment").unwrap();
        assert!(
            LibraryTree::load(Some(&library.0))
                .node("note")
                .unwrap()
                .canvas_path
                .is_none()
        );
        let canvas = directory.join(CANVAS_FILE);
        write_fixture(&canvas, [0xff]).unwrap();
        let tree = LibraryTree::load(Some(&library.0));
        let node = tree.node("note").unwrap();
        assert_eq!(node.canvas_path.as_deref(), Some(canvas.as_path()));
        assert_eq!(
            node.attachments
                .iter()
                .map(|file| file.name.as_str())
                .collect::<Vec<_>>(),
            ["file.txt"]
        );
        assert!(load_canvas(&library.0, &directory).is_err());
        write_fixture(&canvas, "<button>First</button>").unwrap();
        assert_eq!(
            load_canvas(&library.0, &directory).unwrap(),
            "<button>First</button>"
        );
        write_fixture(&canvas, "<button>Latest</button>").unwrap();
        assert_eq!(
            load_canvas(&library.0, &directory).unwrap(),
            "<button>Latest</button>"
        );
        fs::remove_file(&canvas).unwrap();
        assert!(
            LibraryTree::load(Some(&library.0))
                .node("note")
                .unwrap()
                .canvas_path
                .is_none()
        );
    }

    #[cfg(unix)]
    #[test]
    fn canvas_loading_rejects_symlinks_and_directories_outside_the_library() {
        let library = TestLibrary::new();
        let outside = TestLibrary::new();
        let directory = library.0.join("note");
        fs::create_dir(&directory).unwrap();
        write_fixture(directory.join("index.md"), "Note").unwrap();
        write_fixture(outside.0.join("index.md"), "Outside").unwrap();
        write_fixture(outside.0.join(CANVAS_FILE), "Outside canvas").unwrap();
        assert!(load_canvas(&library.0, &outside.0).is_err());
        fs::create_dir(directory.join("canvas")).unwrap();
        std::os::unix::fs::symlink(outside.0.join(CANVAS_FILE), directory.join(CANVAS_FILE))
            .unwrap();
        assert!(load_canvas(&library.0, &directory).is_err());
        let tree = LibraryTree::load(Some(&library.0));
        let node = tree.node("note").unwrap();
        assert!(node.canvas_path.is_none());
        assert!(node.attachments.is_empty());
    }

    #[test]
    fn loads_metadata_into_the_tree_and_note_content_on_demand() {
        let library = TestLibrary::new();
        let note_directory = library.0.join("notes");
        fs::create_dir(&note_directory).unwrap();
        let document_path = note_directory.join("index.md");
        write_fixture(
            &document_path,
            "---\ntitle: Notes\nsummary: Metadata only.\nstatus: active\npriority: 2\ngraph: false\n---\n# Body",
        )
        .unwrap();

        let tree = LibraryTree::load(Some(&library.0));
        let node = tree.node("notes").unwrap();
        assert_eq!(node.title, "Notes");
        assert_eq!(node.summary.as_deref(), Some("Metadata only."));
        assert_eq!(
            node.additional_metadata,
            vec![
                (
                    "status".to_owned(),
                    serde_json::Value::String("active".to_owned())
                ),
                ("priority".to_owned(), serde_json::json!(2)),
                ("graph".to_owned(), serde_json::Value::Bool(false)),
            ]
        );
        assert_eq!(node.document_path.as_deref(), Some(document_path.as_path()));
        assert_eq!(load_note(&document_path).unwrap(), "# Body");
    }

    #[test]
    fn metadata_scan_does_not_read_the_document_body() {
        let library = TestLibrary::new();
        let note_directory = library.0.join("notes");
        fs::create_dir(&note_directory).unwrap();
        let document_path = note_directory.join("index.md");
        let mut document = b"---\ntitle: Notes\n---\n".to_vec();
        document.push(0xff);
        write_fixture(&document_path, document).unwrap();

        let tree = LibraryTree::load(Some(&library.0));
        assert_eq!(tree.node("notes").unwrap().title, "Notes");
        assert!(load_note(&document_path).is_err());
    }

    #[test]
    fn icon_source_changes_when_the_icon_file_changes() {
        let library = TestLibrary::new();
        let note_directory = library.0.join("notes");
        fs::create_dir(&note_directory).unwrap();
        write_fixture(note_directory.join("index.md"), "---\ntitle: Notes\n---\n").unwrap();
        let icon_path = note_directory.join("assets/icon.png");
        write_fixture(&icon_path, [1]).unwrap();
        let first = LibraryTree::load(Some(&library.0))
            .node("notes")
            .unwrap()
            .icon_image
            .clone()
            .unwrap();

        write_fixture(&icon_path, [1, 2]).unwrap();
        let second = LibraryTree::load(Some(&library.0))
            .node("notes")
            .unwrap()
            .icon_image
            .clone()
            .unwrap();

        assert_ne!(first, second);
        assert_eq!(second.byte_len, 2);
    }

    #[test]
    fn lists_explicit_archives_without_exposing_their_descendants() {
        let library = TestLibrary::new();
        let parent = library.0.join("parent");
        let child = parent.join("child");
        fs::create_dir_all(&child).unwrap();
        write_fixture(
            parent.join("index.md"),
            "---\ntitle: Parent\narchived: true\n---\n",
        )
        .unwrap();
        write_fixture(
            child.join("index.md"),
            "---\ntitle: Child\narchived: true\n---\n",
        )
        .unwrap();

        let tree = LibraryTree::load(Some(&library.0));

        assert!(tree.nodes.is_empty());
        assert_eq!(
            tree.archived_notes
                .iter()
                .map(|note| note.path.as_str())
                .collect::<Vec<_>>(),
            ["parent", "parent/child"]
        );
    }

    #[test]
    fn restoring_an_archive_returns_it_to_the_visible_tree() {
        let library = TestLibrary::new();
        let note_directory = library.0.join("notes");
        fs::create_dir(&note_directory).unwrap();
        let document_path = note_directory.join("index.md");
        write_fixture(
            &document_path,
            "---\ntitle: Notes\narchived: true\n---\nBody",
        )
        .unwrap();

        restore_archived_note(&document_path).unwrap();
        let tree = LibraryTree::load(Some(&library.0));

        assert!(tree.archived_notes.is_empty());
        assert_eq!(tree.node("notes").unwrap().title, "Notes");
        assert_eq!(
            fs::read_to_string(document_path).unwrap(),
            "---\ntitle: Notes\n---\nBody"
        );
    }
}
