use std::borrow::Cow;
use std::path::{Path, PathBuf};

use rmcp::handler::server::{router::tool::ToolRouter, wrapper::Parameters};
use rmcp::model::{
    CallToolResult, ContentBlock, Implementation, ProtocolVersion, ServerCapabilities, ServerConfig,
};
use rmcp::{ServerHandler, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use locus_library::{
    LIBRARY_REFERENCE_SCHEME, LibraryNode, LibraryTree, library_reference_path, load_note,
};

const MCP_INSTRUCTIONS: &str = "Read-only discovery of the selected local Markdown library. Use the Locus skill for authoring guidance.";

#[derive(Clone)]
pub(super) struct LocusMcp {
    root: Option<PathBuf>,
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ListArticlesRequest {
    /// Library-relative article path or locus:// address; empty or locus:// lists the root.
    #[serde(default)]
    parent: String,
    cursor: Option<usize>,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SearchArticlesRequest {
    query: String,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ResolveArticleRequest {
    /// Library-relative article path or locus:// address, optionally ending in /index.md.
    path: String,
}

#[tool_router(router = tool_router)]
impl LocusMcp {
    pub(super) fn new(root: Option<PathBuf>) -> Self {
        Self {
            root,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        name = "list_articles",
        description = "List child articles with metadata and local paths.",
        annotations(
            title = "List child articles",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn list_articles(
        &self,
        Parameters(request): Parameters<ListArticlesRequest>,
    ) -> CallToolResult {
        tool_result(self.list_articles_value(request))
    }

    #[tool(
        name = "search_articles",
        description = "Search articles by keyword, returning metadata and local paths.",
        annotations(
            title = "Search articles",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn search_articles(
        &self,
        Parameters(request): Parameters<SearchArticlesRequest>,
    ) -> CallToolResult {
        tool_result(self.search_articles_value(request))
    }

    #[tool(
        name = "resolve_article",
        description = "Resolve an article path to metadata and local paths.",
        annotations(
            title = "Resolve an article path",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    fn resolve_article(
        &self,
        Parameters(request): Parameters<ResolveArticleRequest>,
    ) -> CallToolResult {
        tool_result(self.resolve_article_value(request))
    }

    fn list_articles_value(&self, request: ListArticlesRequest) -> Result<Value, String> {
        let root = self.root()?;
        let tree = LibraryTree::load(Some(root));
        ensure_tree_loaded(&tree)?;
        let parent = normalize_article_path(&request.parent, true)?;
        if !parent.is_empty()
            && !tree
                .node(&parent)
                .is_some_and(|node| node.document_path.is_some())
        {
            return Err(format!("文章不存在：{parent}"));
        }
        let cursor = request.cursor.unwrap_or(0);
        let limit = request.limit.unwrap_or(50);
        if !(1..=200).contains(&limit) {
            return Err("limit 必须是 1 到 200 之间的整数。".to_owned());
        }
        let mut documents = Vec::new();
        collect_documents(&tree.nodes, &mut documents);
        let children = documents
            .iter()
            .copied()
            .filter(|node| article_parent(&node.path) == parent)
            .collect::<Vec<_>>();
        let page = children
            .iter()
            .skip(cursor)
            .take(limit)
            .copied()
            .collect::<Vec<_>>();
        let articles = page
            .into_iter()
            .map(|node| article_value(root, node, Some(child_count(&documents, &node.path))))
            .collect::<Result<Vec<_>, _>>()?;
        let next_cursor =
            (cursor + articles.len() < children.len()).then_some(cursor + articles.len());
        Ok(json!({
            "parent": parent,
            "articles": articles,
            "nextCursor": next_cursor,
        }))
    }

    fn search_articles_value(&self, request: SearchArticlesRequest) -> Result<Value, String> {
        let root = self.root()?;
        let tree = LibraryTree::load(Some(root));
        ensure_tree_loaded(&tree)?;
        let query = request.query.trim();
        if query.is_empty() {
            return Err("query 不能为空。".to_owned());
        }
        let limit = request.limit.unwrap_or(20);
        if !(1..=100).contains(&limit) {
            return Err("limit 必须是 1 到 100 之间的整数。".to_owned());
        }
        let needle = query.to_lowercase();
        let mut documents = Vec::new();
        collect_documents(&tree.nodes, &mut documents);
        let mut total = 0;
        let mut results = Vec::new();
        for node in documents {
            let document_path = node.document_path.as_deref().expect("document node");
            let body = load_note(document_path)
                .map_err(|error| format!("无法读取 {}：{error}", document_path.display()))?;
            let searchable = format!(
                "{}\n{}\n{}\n{}\n{}",
                node.path,
                node.title,
                node.summary.as_deref().unwrap_or_default(),
                node.attachments
                    .iter()
                    .map(|attachment| attachment.name.as_str())
                    .collect::<Vec<_>>()
                    .join(" "),
                body
            )
            .to_lowercase();
            if !searchable.contains(&needle) {
                continue;
            }
            total += 1;
            if results.len() < limit {
                let mut article = article_value(root, node, None)?;
                article["excerpt"] = Value::String(create_excerpt(&body, query));
                results.push(article);
            }
        }
        Ok(json!({
            "query": query,
            "total": total,
            "results": results,
        }))
    }

    fn resolve_article_value(&self, request: ResolveArticleRequest) -> Result<Value, String> {
        let root = self.root()?;
        let tree = LibraryTree::load(Some(root));
        ensure_tree_loaded(&tree)?;
        let path = normalize_article_path(&request.path, false)?;
        let node = tree
            .node(&path)
            .filter(|node| node.document_path.is_some())
            .ok_or_else(|| format!("文章不存在：{path}"))?;
        let mut documents = Vec::new();
        collect_documents(&tree.nodes, &mut documents);
        article_value(root, node, Some(child_count(&documents, &path)))
    }

    fn root(&self) -> Result<&Path, String> {
        self.root
            .as_deref()
            .ok_or_else(|| "尚未选择文档库。".to_owned())
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for LocusMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("locus", env!("CARGO_PKG_VERSION")))
            .with_protocol_version(ProtocolVersion::V_2025_11_25)
            .with_instructions(MCP_INSTRUCTIONS)
    }

    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        Cow::Owned(vec![ProtocolVersion::V_2025_11_25])
    }
}

fn tool_result(result: Result<Value, String>) -> CallToolResult {
    match result {
        Ok(value) => {
            let text = serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string());
            let mut result = CallToolResult::success(vec![ContentBlock::text(text)]);
            result.structured_content = Some(value);
            result
        }
        Err(error) => CallToolResult::error(vec![ContentBlock::text(error)]),
    }
}

fn ensure_tree_loaded(tree: &LibraryTree) -> Result<(), String> {
    tree.error.clone().map_or(Ok(()), Err)
}

fn collect_documents<'a>(nodes: &'a [LibraryNode], documents: &mut Vec<&'a LibraryNode>) {
    for node in nodes {
        if node.document_path.is_some() {
            documents.push(node);
        }
        collect_documents(&node.children, documents);
    }
}

fn article_value(
    root: &Path,
    node: &LibraryNode,
    child_count: Option<usize>,
) -> Result<Value, String> {
    let document_path = node.document_path.as_deref().expect("document node");
    let body = load_note(document_path)
        .map_err(|error| format!("无法读取 {}：{error}", document_path.display()))?;
    let icon = if node.icon_image.is_some() {
        Some("assets/icon.png")
    } else {
        node.icon.as_deref()
    };
    let mut value = json!({
        "archived": false,
        "path": node.path,
        "title": node.title,
        "icon": icon,
        "summary": node.summary,
        "lineCount": body.lines().count(),
        "attachmentCount": node.attachments.len(),
        "file": document_path.to_string_lossy(),
        "directory": root.join(&node.path).to_string_lossy(),
    });
    if let Some(child_count) = child_count {
        value["childCount"] = json!(child_count);
    }
    Ok(value)
}

fn child_count(documents: &[&LibraryNode], parent: &str) -> usize {
    documents
        .iter()
        .filter(|node| article_parent(&node.path) == parent)
        .count()
}

fn article_parent(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(parent, _)| parent)
}

fn normalize_article_path(raw: &str, allow_empty: bool) -> Result<String, String> {
    let raw = raw.trim();
    let mut path = if raw == LIBRARY_REFERENCE_SCHEME {
        String::new()
    } else if raw.starts_with(LIBRARY_REFERENCE_SCHEME) {
        library_reference_path(raw).ok_or_else(|| "文章路径无效。".to_owned())?
    } else {
        raw.replace('\\', "/")
    };
    path = path.trim_matches('/').to_owned();
    if path == "index.md" {
        path.clear();
    } else if let Some(without_index) = path.strip_suffix("/index.md") {
        path = without_index.trim_end_matches('/').to_owned();
    }
    if path.is_empty() {
        return allow_empty
            .then_some(path)
            .ok_or_else(|| "文章路径不能为空。".to_owned());
    }
    if path
        .split('/')
        .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return Err("文章路径无效。".to_owned());
    }
    Ok(path)
}

fn create_excerpt(content: &str, query: &str) -> String {
    let plain = content
        .chars()
        .map(|character| {
            if "#>*_`~[]()!-".contains(character) || character.is_whitespace() {
                ' '
            } else {
                character
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if plain.is_empty() {
        return String::new();
    }
    let lowercase = plain.to_lowercase();
    let index = lowercase.find(&query.to_lowercase()).unwrap_or(0);
    let start = plain
        .char_indices()
        .map(|(index, _)| index)
        .take_while(|candidate| *candidate <= index.saturating_sub(70))
        .last()
        .unwrap_or(0);
    let end_target = (start + 180).min(plain.len());
    let end = plain
        .char_indices()
        .map(|(index, _)| index)
        .find(|candidate| *candidate >= end_target)
        .unwrap_or(plain.len());
    format!(
        "{}{}{}",
        if start > 0 { "…" } else { "" },
        &plain[start..end],
        if end < plain.len() { "…" } else { "" }
    )
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
    fn custom_metadata_is_preserved_without_shortcut_discovery_or_inheritance() {
        let root = tempfile::tempdir().unwrap();
        let original = "---\ntitle: Project\nshortcuts:\n  website: https://example.com\n  directory: ./source\n---\nBody";
        let parent = root.path().join("project/index.md");
        write_fixture(&parent, original).unwrap();
        write_fixture(
            root.path().join("project/child/index.md"),
            "---\ntitle: Child\n---\n",
        )
        .unwrap();
        let service = LocusMcp::new(Some(root.path().to_owned()));
        for path in ["project", "project/child"] {
            let result = service
                .resolve_article_value(ResolveArticleRequest { path: path.into() })
                .unwrap();
            assert!(result.get("shortcuts").is_none());
            assert!(result.get("resolvedShortcuts").is_none());
        }
        let listed = service
            .list_articles_value(ListArticlesRequest {
                parent: "project".into(),
                cursor: None,
                limit: None,
            })
            .unwrap();
        let searched = service
            .search_articles_value(SearchArticlesRequest {
                query: "Child".into(),
                limit: None,
            })
            .unwrap();
        for result in [&listed["articles"][0], &searched["results"][0]] {
            assert!(result.get("shortcuts").is_none());
            assert!(result.get("resolvedShortcuts").is_none());
        }
        let access = locus_library::LibraryAccess::new(Some(root.path().to_path_buf()));
        let own = access.describe("locus://project").unwrap();
        assert_eq!(
            own["metadata"]["shortcuts"]["website"],
            "https://example.com"
        );
        assert!(own.get("resolvedShortcuts").is_none());
        let child = access.describe("locus://project/child").unwrap();
        assert!(child["metadata"].get("shortcuts").is_none());
        assert!(child.get("resolvedShortcuts").is_none());
        assert_eq!(std::fs::read_to_string(parent).unwrap(), original);
    }

    #[test]
    fn canvas_is_excluded_from_discovery_attachment_counts() {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("note");
        std::fs::create_dir(&directory).unwrap();
        write_fixture(directory.join("index.md"), "---\ntitle: Note\n---\nBody").unwrap();
        write_fixture(directory.join("canvas/index.html"), "<p>Canvas</p>").unwrap();
        write_fixture(directory.join("assets/file.txt"), "Attachment").unwrap();
        let service = LocusMcp::new(Some(root.path().to_path_buf()));
        let resolved = service
            .resolve_article_value(ResolveArticleRequest {
                path: "note".into(),
            })
            .unwrap();
        assert_eq!(resolved["attachmentCount"], 1);
        let listed = service
            .list_articles_value(ListArticlesRequest {
                parent: String::new(),
                cursor: None,
                limit: None,
            })
            .unwrap();
        assert_eq!(listed["articles"][0]["attachmentCount"], 1);
        let searched = service
            .search_articles_value(SearchArticlesRequest {
                query: "Note".into(),
                limit: None,
            })
            .unwrap();
        assert_eq!(searched["results"][0]["attachmentCount"], 1);
    }

    #[test]
    fn encoded_addresses_resolve_and_list_the_same_files_as_relative_paths() {
        let root = tempfile::tempdir().unwrap();
        let parent = "旅行 计划%#";
        let child = format!("{parent}/literal%2e");
        std::fs::create_dir_all(root.path().join(&child)).unwrap();
        for path in [parent, &child] {
            write_fixture(root.path().join(path).join("index.md"), "Body").unwrap();
        }
        let service = LocusMcp::new(Some(root.path().to_path_buf()));
        let resolved = service
            .resolve_article_value(ResolveArticleRequest {
                path: "locus://旅行%20计划%25%23/literal%252e/index.md".into(),
            })
            .unwrap();
        let relative = service
            .resolve_article_value(ResolveArticleRequest {
                path: child.clone(),
            })
            .unwrap();
        assert_eq!(resolved, relative);
        assert_eq!(resolved["path"], child);
        let listed = service
            .list_articles_value(ListArticlesRequest {
                parent: "locus://旅行%20计划%25%23".into(),
                cursor: None,
                limit: None,
            })
            .unwrap();
        assert_eq!(listed["parent"], parent);
        assert_eq!(listed["articles"][0], resolved);
    }

    #[test]
    fn article_paths_accept_relative_paths_and_library_addresses() {
        for path in [
            "projects/locus",
            "projects/locus/index.md",
            "locus://projects/locus",
            " locus://projects/locus/index.md ",
        ] {
            assert_eq!(
                normalize_article_path(path, false).unwrap(),
                "projects/locus"
            );
        }
        assert_eq!(
            normalize_article_path("locus://旅行%20计划/100%25%23", false).unwrap(),
            "旅行 计划/100%#"
        );
        assert_eq!(
            normalize_article_path("locus://literal%252e", false).unwrap(),
            "literal%2e"
        );
        assert_eq!(
            normalize_article_path("literal%2e", false).unwrap(),
            "literal%2e"
        );
        for root in ["", "locus://", "index.md", "locus://index.md"] {
            assert_eq!(normalize_article_path(root, true).unwrap(), "");
            assert!(normalize_article_path(root, false).is_err());
        }
    }

    #[test]
    fn article_paths_reject_invalid_addresses_and_traversal() {
        for path in [
            "../outside",
            "locus://projects/../outside",
            "locus://projects/%2e%2e/outside",
            "locus:///projects/locus",
            "locus://projects//locus",
            "locus://projects%2flocus",
            "locus://projects%5clocus",
            "locus://note%00",
            "locus://%FF",
            "locus://note?query",
            "locus://note#fragment",
        ] {
            for allow_empty in [false, true] {
                assert!(normalize_article_path(path, allow_empty).is_err(), "{path}");
            }
        }
    }

    #[test]
    fn excerpt_is_compact_and_marks_truncation() {
        let content = format!("{} target {}", "a".repeat(100), "b".repeat(200));
        let excerpt = create_excerpt(&content, "target");
        assert!(excerpt.starts_with('…'));
        assert!(excerpt.ends_with('…'));
        assert!(excerpt.contains("target"));
    }
}
