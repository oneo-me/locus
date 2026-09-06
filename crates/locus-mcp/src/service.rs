use std::borrow::Cow;
use std::path::{Path, PathBuf};

use rmcp::handler::server::{router::tool::ToolRouter, wrapper::Parameters};
use rmcp::model::{
    CallToolResult, ContentBlock, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
};
use rmcp::{ServerHandler, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

use locus_library::{LibraryNode, LibraryTree, load_note};

const MCP_INSTRUCTIONS: &str = "Locus exposes list_articles, search_articles, and resolve_article for discovering a local Markdown library. If the user provides a library-relative article path, call resolve_article directly; do not list or search first. Use search_articles for titles or keywords; use list_articles to browse children. Each result includes an article path for hierarchy navigation plus absolute file and directory paths. Edit content with the client file tools: each article is a directory containing index.md; YAML frontmatter holds title, icon, summary, archived state, and optional graph participation; sibling regular files are attachments. Do not recreate CRUD or attachment tools through MCP.";

#[derive(Clone)]
pub(super) struct LocusMcp {
    root: Option<PathBuf>,
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ListArticlesRequest {
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
        description = "Lists direct child articles for a parent identifier with cursor pagination. Each result includes article metadata, Markdown body lineCount, attachmentCount, childCount, and absolute local file and directory paths for native editing.",
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
        description = "Searches Markdown article titles, public metadata, and body content. Each result includes attachmentCount plus absolute file and directory paths for native editing.",
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
        description = "Resolves a known library-relative article path to public metadata, attachmentCount, and absolute local file and directory paths. Accepts an optional trailing /index.md. Use this instead of search when the path is already known.",
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
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
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
        Some("icon.png")
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
    let mut path = raw.trim().replace('\\', "/");
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
    use super::*;

    #[test]
    fn article_paths_accept_index_suffix_and_reject_traversal() {
        assert_eq!(
            normalize_article_path("projects/locus/index.md", false).unwrap(),
            "projects/locus"
        );
        assert!(normalize_article_path("../outside", false).is_err());
        assert_eq!(normalize_article_path("", true).unwrap(), "");
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
