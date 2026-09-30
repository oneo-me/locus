use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::LibraryTree;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShortcutKind {
    Website,
    Github,
    Directory,
}

impl ShortcutKind {
    pub const ALL: [Self; 3] = [Self::Website, Self::Github, Self::Directory];

    pub fn key(self) -> &'static str {
        match self {
            Self::Website => "website",
            Self::Github => "github",
            Self::Directory => "directory",
        }
    }
}

/// A whole shortcut group, including explicit null and invalid configuration boundaries.
#[derive(Clone, Debug)]
pub struct ResolvedShortcuts {
    source_path: String,
    source_title: String,
    source_directory: PathBuf,
    values: serde_json::Map<String, Value>,
    disabled: bool,
    errors: Vec<String>,
}

impl ResolvedShortcuts {
    pub(crate) fn new(path: &str, title: &str, directory: &Path, raw: &Value) -> Self {
        let mut result = Self {
            source_path: path.to_owned(),
            source_title: title.to_owned(),
            source_directory: directory.to_owned(),
            values: Default::default(),
            disabled: raw.is_null(),
            errors: Vec::new(),
        };
        if raw.is_null() {
            return result;
        }
        let Some(fields) = raw.as_object() else {
            result
                .errors
                .push("shortcuts must be a mapping or null".into());
            return result;
        };
        for kind in ShortcutKind::ALL {
            let key = kind.key();
            let Some(value) = fields.get(key).filter(|value| !value.is_null()) else {
                continue;
            };
            let Some(value) = value
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                result.errors.push(format!(
                    "shortcuts.{key} must be a non-empty string or null"
                ));
                continue;
            };
            let valid = if kind == ShortcutKind::Directory {
                !value.contains('\0')
                    && (!value.starts_with('~') || value == "~" || value.starts_with("~/"))
            } else {
                url::Url::parse(value).is_ok_and(|url| {
                    matches!(url.scheme(), "http" | "https") && url.host_str().is_some()
                })
            };
            if valid {
                result
                    .values
                    .insert(key.into(), Value::String(value.into()));
            } else {
                result.errors.push(format!(
                    "shortcuts.{key} is not a valid {}",
                    if kind == ShortcutKind::Directory {
                        "directory path"
                    } else {
                        "HTTP(S) URL"
                    }
                ));
            }
        }
        result
    }

    pub fn source_path(&self) -> &str {
        &self.source_path
    }

    pub fn source_title(&self) -> &str {
        &self.source_title
    }

    pub fn target(&self, kind: ShortcutKind) -> Option<&str> {
        self.values.get(kind.key()).and_then(Value::as_str)
    }

    pub fn errors(&self) -> &[String] {
        &self.errors
    }

    /// Resolve against the defining note, never the selected descendant or process directory.
    pub fn directory_path(&self) -> Result<Option<PathBuf>, String> {
        let Some(raw) = self.target(ShortcutKind::Directory) else {
            return Ok(None);
        };
        if raw == "~" || raw.starts_with("~/") {
            let home = dirs::home_dir().ok_or("Home directory is unavailable")?;
            return Ok(Some(home.join(raw.strip_prefix("~/").unwrap_or(""))));
        }
        let path = Path::new(raw);
        Ok(Some(if path.is_absolute() {
            path.to_owned()
        } else {
            self.source_directory.join(path)
        }))
    }

    /// Keep directory values as authored: full MCP must not expose resolved host paths.
    pub fn mcp_value(&self, current_path: &str) -> Value {
        json!({
            "source": crate::access::canonical_uri(&self.source_path.split('/').map(str::to_owned).collect::<Vec<_>>()),
            "inherited": self.source_path != current_path,
            "disabled": self.disabled,
            "values": self.values,
            "errors": self.errors,
        })
    }
}

impl LibraryTree {
    pub fn resolved_shortcuts(&self, path: &str) -> Option<ResolvedShortcuts> {
        let mut current = path;
        loop {
            if let Some(node) = self.node(current)
                && let Some((_, raw)) = node
                    .additional_metadata
                    .iter()
                    .find(|(key, _)| key == "shortcuts")
            {
                return Some(ResolvedShortcuts::new(
                    &node.path,
                    &node.title,
                    &node.directory,
                    raw,
                ));
            }
            current = current.rsplit_once('/')?.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LibraryAccess;

    #[test]
    fn inheritance_is_whole_group_and_preserves_boundaries_and_source_paths() {
        let root = tempfile::tempdir().unwrap();
        let access = LibraryAccess::new(Some(root.path().to_owned()));
        let parent = "locus://project%25%23";
        access.create_note(parent, "---\ntitle: Project\nshortcuts:\n  website: https://example.com\n  github: https://github.com/example/project\n  directory: ./source\n---\nParent").unwrap();
        access
            .create_directory("locus://project%25%23/group")
            .unwrap();
        access
            .create_note("locus://project%25%23/group/child", "Child")
            .unwrap();
        access
            .create_note(
                "locus://project%25%23/override",
                "---\nshortcuts:\n  github: https://github.com/example/other\n---\n",
            )
            .unwrap();
        access
            .create_note(
                "locus://project%25%23/blocked",
                "---\nshortcuts: null\n---\n",
            )
            .unwrap();
        access
            .create_note("locus://project%25%23/blocked/child", "Child")
            .unwrap();
        access
            .create_note(
                "locus://project%25%23/blocked/own",
                "---\nshortcuts:\n  website: https://own.example.com\n---\n",
            )
            .unwrap();
        access
            .create_note("locus://project%25%23/empty", "---\nshortcuts: {}\n---\n")
            .unwrap();
        access
            .create_note("locus://unrelated", "Unrelated")
            .unwrap();
        let tree = LibraryTree::load(Some(root.path()));
        let child = tree.resolved_shortcuts("project%#/group/child").unwrap();
        assert_eq!(child.source_title(), "Project");
        assert_eq!(
            child.directory_path().unwrap().unwrap(),
            root.path().join("project%#/./source")
        );
        assert_eq!(child.mcp_value("project%#/group/child")["source"], parent);
        assert_eq!(child.mcp_value("project%#/group/child")["inherited"], true);
        let own = tree.resolved_shortcuts("project%#/override").unwrap();
        assert!(own.target(ShortcutKind::Website).is_none());
        assert!(own.target(ShortcutKind::Directory).is_none());
        assert_eq!(own.mcp_value("project%#/override")["inherited"], false);
        let blocked = tree.resolved_shortcuts("project%#/blocked/child").unwrap();
        assert_eq!(
            blocked.mcp_value("project%#/blocked/child")["disabled"],
            true
        );
        assert!(blocked.values.is_empty());
        assert!(
            tree.resolved_shortcuts("project%#/empty")
                .unwrap()
                .values
                .is_empty()
        );
        assert!(
            tree.resolved_shortcuts("project%#/blocked/own")
                .unwrap()
                .target(ShortcutKind::Website)
                .is_some()
        );
        assert!(tree.resolved_shortcuts("unrelated").is_none());
        for path in [
            "project%#",
            "project%#/group/child",
            "project%#/override",
            "project%#/blocked/child",
            "project%#/empty",
        ] {
            let expected = tree.resolved_shortcuts(path).unwrap().mcp_value(path);
            let uri = crate::access::canonical_uri(
                &path.split('/').map(str::to_owned).collect::<Vec<_>>(),
            );
            let actual = access.describe(&uri).unwrap();
            assert_eq!(actual["resolvedShortcuts"], expected);
            assert!(
                !actual["resolvedShortcuts"]
                    .to_string()
                    .contains(root.path().to_str().unwrap())
            );
        }
        let description = access
            .describe("locus://project%25%23/group/child")
            .unwrap();
        assert!(description["metadata"].get("shortcuts").is_none());
        assert_eq!(
            std::fs::read_to_string(root.path().join("project%#/group/child/index.md")).unwrap(),
            "Child"
        );
        // A parent edit is reflected by both consumers without modifying descendants.
        std::fs::write(
            root.path().join("project%#/index.md"),
            "---\nshortcuts: null\n---\n",
        )
        .unwrap();
        assert_eq!(
            LibraryTree::load(Some(root.path()))
                .resolved_shortcuts("project%#/group/child")
                .unwrap()
                .mcp_value("project%#/group/child")["disabled"],
            true
        );
        assert_eq!(
            access
                .describe("locus://project%25%23/group/child")
                .unwrap()["resolvedShortcuts"]["disabled"],
            true
        );
    }

    #[test]
    fn validates_targets_without_falling_back_or_expanding_shell_expressions() {
        let raw =
            json!({"website": "javascript:alert(1)", "github": 5, "directory": "./$(touch nope)"});
        let shortcuts =
            ResolvedShortcuts::new("project", "Project", Path::new("/notes/project"), &raw);
        assert_eq!(shortcuts.errors().len(), 2);
        assert_eq!(
            shortcuts.directory_path().unwrap().unwrap(),
            Path::new("/notes/project/./$(touch nope)")
        );
        for raw in [json!(false), json!([]), json!("bad")] {
            assert!(
                !ResolvedShortcuts::new("p", "P", Path::new("/p"), &raw)
                    .errors()
                    .is_empty()
            );
        }
        for target in ["file:///tmp/test", "ftp://example.com", "https://", ""] {
            let result =
                ResolvedShortcuts::new("p", "P", Path::new("/p"), &json!({"website": target}));
            assert!(result.target(ShortcutKind::Website).is_none());
            assert!(!result.errors().is_empty());
        }
        let home = ResolvedShortcuts::new(
            "p",
            "P",
            Path::new("/p"),
            &json!({"directory": "~/Projects/example"}),
        );
        if let Some(directory) = dirs::home_dir() {
            assert_eq!(
                home.directory_path().unwrap().unwrap(),
                directory.join("Projects/example")
            );
        }
        let absolute = std::env::temp_dir();
        let result =
            ResolvedShortcuts::new("p", "P", Path::new("/p"), &json!({"directory": absolute}));
        assert_eq!(result.directory_path().unwrap().unwrap(), absolute);
    }
}
