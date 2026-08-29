use std::path::{Path, PathBuf};
use std::process::Command;

use super::ChangeStatus;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RepositoryChange {
    status: ChangeStatus,
    path: String,
    previous_path: Option<String>,
}

pub(super) fn read_changes(root: &Path) -> Result<Vec<RepositoryChange>, String> {
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
        .env("GIT_OPTIONAL_LOCKS", "0")
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

pub(super) fn document_change(
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

#[cfg(test)]
mod tests {
    use super::*;

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
