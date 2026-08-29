use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const DOCUMENT_FILE: &str = "index.md";
const DOCUMENT_ICON_FILE: &str = "icon.png";

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ChangeStatus {
    Added,
    Modified,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibraryNode {
    pub path: String,
    pub directory: PathBuf,
    pub title: String,
    pub icon: Option<String>,
    pub icon_path: Option<PathBuf>,
    pub summary: Option<String>,
    pub content: Option<String>,
    pub attachment_count: usize,
    pub change: Option<ChangeStatus>,
    pub descendant_change: Option<ChangeStatus>,
    pub children: Vec<LibraryNode>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LibraryTree {
    pub nodes: Vec<LibraryNode>,
    pub error: Option<String>,
    pub repository_error: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RepositoryChange {
    status: ChangeStatus,
    path: String,
    previous_path: Option<String>,
}

#[derive(Default)]
struct DocumentMetadata {
    title: Option<String>,
    icon: Option<String>,
    summary: Option<String>,
    archived: bool,
}

#[derive(Default, serde::Deserialize)]
#[serde(default)]
struct RawDocumentMetadata {
    title: Option<serde_json::Value>,
    icon: Option<serde_json::Value>,
    summary: Option<serde_json::Value>,
    archived: bool,
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

        let (changes, repository_error) = match read_repository_changes(root) {
            Ok(changes) => (changes, None),
            Err(error) => (Vec::new(), Some(error)),
        };
        match scan_children(root, root, &changes) {
            Ok(mut nodes) => {
                nodes.sort_by(compare_nodes);
                Self {
                    nodes,
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

fn scan_children(
    root: &Path,
    directory: &Path,
    changes: &[RepositoryChange],
) -> Result<Vec<LibraryNode>, std::io::Error> {
    let mut nodes = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !file_type.is_dir()
            || file_type.is_symlink()
            || name.starts_with('.')
            || name == "node_modules"
        {
            continue;
        }

        let path = entry.path();
        let document_path = path.join(DOCUMENT_FILE);
        let has_document = document_path.is_file();
        let document = has_document
            .then(|| fs::read_to_string(&document_path))
            .transpose()?;
        let metadata = if let Some(document) = &document {
            parse_document_metadata(document)
        } else {
            DocumentMetadata::default()
        };
        if metadata.archived {
            continue;
        }

        let mut children = scan_children(root, &path, changes)?;
        children.sort_by(compare_nodes);
        if !has_document && children.is_empty() {
            continue;
        }

        let relative_directory = relative_path(root, &path);
        let document_relative_path = format!("{relative_directory}/{DOCUMENT_FILE}");
        let change = has_document
            .then(|| document_change(&document_relative_path, &relative_directory, changes))
            .flatten();
        let descendant_change = combined_descendant_change(&children);
        let attachment_count = if has_document {
            count_attachments(&path)?
        } else {
            0
        };
        let icon_path = path.join(DOCUMENT_ICON_FILE);
        let icon_path = icon_path
            .symlink_metadata()
            .ok()
            .filter(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
            .map(|_| icon_path);

        nodes.push(LibraryNode {
            path: relative_directory,
            directory: path,
            title: metadata.title.unwrap_or_else(|| name.into_owned()),
            icon: metadata.icon,
            icon_path,
            summary: metadata.summary,
            content: document.as_deref().map(document_body).map(str::to_owned),
            attachment_count,
            change,
            descendant_change,
            children,
        });
    }
    Ok(nodes)
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

fn compare_nodes(left: &LibraryNode, right: &LibraryNode) -> std::cmp::Ordering {
    left.children
        .is_empty()
        .cmp(&right.children.is_empty())
        .then_with(|| left.path.cmp(&right.path))
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn count_attachments(directory: &Path) -> Result<usize, std::io::Error> {
    let mut count = 0;
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if file_type.is_file()
            && !file_type.is_symlink()
            && !name.starts_with('.')
            && name != DOCUMENT_FILE
            && !name.eq_ignore_ascii_case(DOCUMENT_ICON_FILE)
        {
            count += 1;
        }
    }
    Ok(count)
}

fn parse_document_metadata(content: &str) -> DocumentMetadata {
    let normalized = content.strip_prefix('\u{feff}').unwrap_or(content);
    let Some((frontmatter, _)) = frontmatter_parts(normalized) else {
        return DocumentMetadata::default();
    };
    let Ok(raw) = serde_saphyr::from_str::<RawDocumentMetadata>(frontmatter) else {
        return DocumentMetadata::default();
    };
    DocumentMetadata {
        title: optional_string(raw.title),
        icon: optional_string(raw.icon),
        summary: optional_string(raw.summary),
        archived: raw.archived,
    }
}

fn document_body(content: &str) -> &str {
    let normalized = content.strip_prefix('\u{feff}').unwrap_or(content);
    frontmatter_parts(normalized).map_or(normalized, |(_, body)| body)
}

fn frontmatter_parts(content: &str) -> Option<(&str, &str)> {
    let rest = content
        .strip_prefix("---\r\n")
        .or_else(|| content.strip_prefix("---\n"))?;
    if let Some(end) = rest.find("\n---\r\n") {
        return Some((&rest[..end], &rest[end + 6..]));
    }
    if let Some(end) = rest.find("\n---\n") {
        return Some((&rest[..end], &rest[end + 5..]));
    }
    rest.strip_suffix("\n---")
        .map(|frontmatter| (frontmatter, ""))
}

fn optional_string(value: Option<serde_json::Value>) -> Option<String> {
    let value = match value? {
        serde_json::Value::String(value) => value,
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::Number(value) => value.to_string(),
        _ => return None,
    };
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_owned())
    }
}

fn read_repository_changes(root: &Path) -> Result<Vec<RepositoryChange>, String> {
    let top_level = run_git(root, &["rev-parse", "--show-toplevel"])?;
    let repository_root = PathBuf::from(String::from_utf8_lossy(&top_level).trim().to_owned());
    let repository_root = repository_root
        .canonicalize()
        .map_err(|_| "文档库目录不是 Git 仓库。".to_owned())?;
    let root = root
        .canonicalize()
        .map_err(|_| "文档库目录不存在或无法访问。".to_owned())?;
    if repository_root != root {
        return Err("文档库目录必须是 Git 仓库根目录。".to_owned());
    }

    let output = run_git(
        &root,
        &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
    )?;
    Ok(parse_repository_changes(&output))
}

fn run_git(root: &Path, arguments: &[&str]) -> Result<Vec<u8>, String> {
    let output = Command::new("git")
        .arg("-c")
        .arg(format!("safe.directory={}", root.display()))
        .arg("-C")
        .arg(root)
        .args(arguments)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("LC_ALL", "C")
        .output()
        .map_err(|error| format!("Git 不可用：{error}"))?;
    if output.status.success() {
        return Ok(output.stdout);
    }
    let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    Err(if detail.is_empty() {
        "文档库目录不是 Git 仓库。".to_owned()
    } else {
        format!("无法读取 Git 状态：{detail}")
    })
}

fn parse_repository_changes(output: &[u8]) -> Vec<RepositoryChange> {
    let fields = output
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty())
        .collect::<Vec<_>>();
    let mut changes = Vec::new();
    let mut index = 0;
    while index < fields.len() {
        let field = fields[index];
        if field.len() < 4 {
            index += 1;
            continue;
        }
        let index_status = field[0];
        let worktree_status = field[1];
        let renamed = matches!(index_status, b'R' | b'C') || matches!(worktree_status, b'R' | b'C');
        let previous_path = if renamed {
            index += 1;
            fields
                .get(index)
                .map(|path| String::from_utf8_lossy(path).into_owned())
        } else {
            None
        };
        let added = index_status == b'A'
            || worktree_status == b'A'
            || (index_status == b'?' && worktree_status == b'?');
        changes.push(RepositoryChange {
            status: if added {
                ChangeStatus::Added
            } else {
                ChangeStatus::Modified
            },
            path: String::from_utf8_lossy(&field[3..]).into_owned(),
            previous_path,
        });
        index += 1;
    }
    changes
}

fn document_change(
    document_path: &str,
    directory: &str,
    changes: &[RepositoryChange],
) -> Option<ChangeStatus> {
    let mut status = None;
    for change in changes {
        let paths = [Some(change.path.as_str()), change.previous_path.as_deref()];
        let changes_document = paths
            .into_iter()
            .flatten()
            .any(|path| path == document_path);
        let changes_attachment = paths
            .into_iter()
            .flatten()
            .any(|path| parent_path(path) == directory);
        if !changes_document && !changes_attachment {
            continue;
        }
        let next = if changes_document && change.status == ChangeStatus::Added {
            ChangeStatus::Added
        } else {
            ChangeStatus::Modified
        };
        if next == ChangeStatus::Added {
            return Some(next);
        }
        status = Some(next);
    }
    status
}

fn parent_path(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(parent, _)| parent)
}

fn combined_descendant_change(children: &[LibraryNode]) -> Option<ChangeStatus> {
    let statuses = children
        .iter()
        .flat_map(|child| [child.change, child.descendant_change])
        .flatten()
        .collect::<HashSet<_>>();
    if statuses.contains(&ChangeStatus::Added) {
        Some(ChangeStatus::Added)
    } else if statuses.contains(&ChangeStatus::Modified) {
        Some(ChangeStatus::Modified)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_sidebar_metadata_from_frontmatter() {
        let metadata = parse_document_metadata(
            "---\ntitle: 'Locus: Notes'\nicon: 🧭\narchived: true\n---\nBody",
        );
        assert_eq!(metadata.title.as_deref(), Some("Locus: Notes"));
        assert_eq!(metadata.icon.as_deref(), Some("🧭"));
        assert!(metadata.archived);
    }

    #[test]
    fn reads_summary_and_separates_markdown_body() {
        let document = "---\ntitle: Notes\nsummary: A short summary.\n---\n# Heading\n\nBody";
        let metadata = parse_document_metadata(document);
        assert_eq!(metadata.summary.as_deref(), Some("A short summary."));
        assert_eq!(document_body(document), "# Heading\n\nBody");
        assert_eq!(document_body("---\ntitle: Notes\n---"), "");
    }

    #[test]
    fn parses_added_and_modified_git_changes() {
        let changes =
            parse_repository_changes(b"?? projects/new/index.md\0 M projects/old/file.png\0");
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].status, ChangeStatus::Added);
        assert_eq!(changes[1].status, ChangeStatus::Modified);
    }

    #[test]
    fn added_document_wins_over_attachment_changes() {
        let changes = vec![
            RepositoryChange {
                status: ChangeStatus::Modified,
                path: "notes/asset.png".to_owned(),
                previous_path: None,
            },
            RepositoryChange {
                status: ChangeStatus::Added,
                path: "notes/index.md".to_owned(),
                previous_path: None,
            },
        ];
        assert_eq!(
            document_change("notes/index.md", "notes", &changes),
            Some(ChangeStatus::Added)
        );
    }
}
