use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use regex::Regex;

use super::{LibraryNode, LibraryTree, load_note};

static CODE_REGION_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)```.*?```|~~~.*?~~~|`[^`\n]+`").expect("valid code region pattern")
});
static WIKI_LINK_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[\[([^\]]+)\]\]").expect("valid wiki link pattern"));

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentGraphNodeKind {
    Document,
    Attachment,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentGraphNode {
    pub id: String,
    pub kind: DocumentGraphNodeKind,
    pub path: String,
    pub title: String,
    pub directory: String,
    pub inbound_count: usize,
    pub outbound_count: usize,
    pub degree: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentGraphEdgeKind {
    Hierarchy,
    Reference,
    Attachment,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentGraphEdge {
    pub kind: DocumentGraphEdgeKind,
    pub source: String,
    pub target: String,
    pub count: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DocumentGraph {
    pub nodes: Vec<DocumentGraphNode>,
    pub edges: Vec<DocumentGraphEdge>,
}

struct GraphDocument<'a> {
    node: &'a LibraryNode,
    parent_path: Option<&'a str>,
}

impl DocumentGraph {
    pub fn from_tree(tree: &LibraryTree) -> Self {
        let mut documents = Vec::new();
        collect_documents(&tree.nodes, None, &mut documents);
        let document_paths = documents
            .iter()
            .map(|document| document.node.path.as_str())
            .collect::<HashSet<_>>();
        let mut edges = Vec::new();

        for document in &documents {
            if let Some(parent_path) = document.parent_path
                && document_paths.contains(parent_path)
            {
                edges.push(DocumentGraphEdge {
                    kind: DocumentGraphEdgeKind::Hierarchy,
                    source: parent_path.to_owned(),
                    target: document.node.path.clone(),
                    count: 1,
                });
            }
        }

        let mut reference_counts = HashMap::<(String, String), usize>::new();
        for document in &documents {
            let Some(document_path) = document.node.document_path.as_deref() else {
                continue;
            };
            let Ok(content) = load_note(document_path) else {
                continue;
            };
            for target in extract_reference_paths(&content) {
                if target != document.node.path && document_paths.contains(target.as_str()) {
                    *reference_counts
                        .entry((document.node.path.clone(), target))
                        .or_default() += 1;
                }
            }
        }
        let mut references = reference_counts.into_iter().collect::<Vec<_>>();
        references.sort_by(|left, right| left.0.cmp(&right.0));
        edges.extend(
            references
                .into_iter()
                .map(|((source, target), count)| DocumentGraphEdge {
                    kind: DocumentGraphEdgeKind::Reference,
                    source,
                    target,
                    count,
                }),
        );

        let mut attachment_nodes = Vec::new();
        for document in &documents {
            for attachment in &document.node.attachments {
                let id = format!(
                    "attachment:{}:{}",
                    utf8_percent_encode(&document.node.path, NON_ALPHANUMERIC),
                    utf8_percent_encode(&attachment.name, NON_ALPHANUMERIC)
                );
                attachment_nodes.push(DocumentGraphNode {
                    id: id.clone(),
                    kind: DocumentGraphNodeKind::Attachment,
                    path: format!("{}/{}", document.node.path, attachment.name),
                    title: attachment.name.clone(),
                    directory: document.node.path.clone(),
                    inbound_count: 0,
                    outbound_count: 0,
                    degree: 0,
                });
                edges.push(DocumentGraphEdge {
                    kind: DocumentGraphEdgeKind::Attachment,
                    source: document.node.path.clone(),
                    target: id,
                    count: 1,
                });
            }
        }

        edges.sort_by(|left, right| {
            edge_kind_order(left.kind)
                .cmp(&edge_kind_order(right.kind))
                .then_with(|| left.source.cmp(&right.source))
                .then_with(|| left.target.cmp(&right.target))
        });
        let (inbound, outbound, neighbours) = graph_counts(&edges);
        let mut nodes = documents
            .into_iter()
            .map(|document| DocumentGraphNode {
                id: document.node.path.clone(),
                kind: DocumentGraphNodeKind::Document,
                path: document.node.path.clone(),
                title: document.node.title.clone(),
                directory: parent_path(&document.node.path)
                    .unwrap_or_default()
                    .to_owned(),
                inbound_count: inbound
                    .get(&document.node.path)
                    .copied()
                    .unwrap_or_default(),
                outbound_count: outbound
                    .get(&document.node.path)
                    .copied()
                    .unwrap_or_default(),
                degree: neighbours
                    .get(&document.node.path)
                    .map(HashSet::len)
                    .unwrap_or_default(),
            })
            .collect::<Vec<_>>();
        nodes.sort_by(|left, right| left.path.cmp(&right.path));
        for mut attachment in attachment_nodes {
            attachment.inbound_count = inbound.get(&attachment.id).copied().unwrap_or_default();
            attachment.outbound_count = outbound.get(&attachment.id).copied().unwrap_or_default();
            attachment.degree = neighbours
                .get(&attachment.id)
                .map(HashSet::len)
                .unwrap_or_default();
            nodes.push(attachment);
        }

        Self { nodes, edges }
    }

    pub fn without_attachments(&self) -> Self {
        let nodes = self
            .nodes
            .iter()
            .filter(|node| node.kind == DocumentGraphNodeKind::Document)
            .cloned()
            .collect::<Vec<_>>();
        let ids = nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect::<HashSet<_>>();
        let edges = self
            .edges
            .iter()
            .filter(|edge| {
                edge.kind != DocumentGraphEdgeKind::Attachment
                    && ids.contains(edge.source.as_str())
                    && ids.contains(edge.target.as_str())
            })
            .cloned()
            .collect();
        Self { nodes, edges }
    }
}

fn collect_documents<'a>(
    nodes: &'a [LibraryNode],
    parent: Option<&'a str>,
    documents: &mut Vec<GraphDocument<'a>>,
) {
    for node in nodes {
        if node.document_path.is_some() && node.included_in_graph {
            documents.push(GraphDocument {
                node,
                parent_path: parent,
            });
        }
        collect_documents(&node.children, Some(&node.path), documents);
    }
}

fn extract_reference_paths(content: &str) -> Vec<String> {
    let content = CODE_REGION_PATTERN.replace_all(content, " ");
    WIKI_LINK_PATTERN
        .captures_iter(&content)
        .filter_map(|captures| normalize_reference(captures.get(1)?.as_str()))
        .collect()
}

fn normalize_reference(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() || raw.contains(['|', '\n', '\\', '\0']) || raw.starts_with('/') {
        return None;
    }
    let path = raw.strip_prefix("locus://").unwrap_or(raw);
    let suffix_start = path.len().saturating_sub("/index.md".len());
    let path = match path.get(suffix_start..) {
        Some(suffix) if suffix.eq_ignore_ascii_case("/index.md") => {
            path.get(..suffix_start).unwrap_or(path)
        }
        _ => path,
    };
    let path = if path.eq_ignore_ascii_case("index.md") {
        ""
    } else {
        path
    };
    let segments = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if segments.is_empty()
        || segments
            .iter()
            .any(|segment| matches!(*segment, "." | ".."))
    {
        return None;
    }
    Some(segments.join("/"))
}

fn parent_path(path: &str) -> Option<&str> {
    path.rsplit_once('/').map(|(parent, _)| parent)
}

fn edge_kind_order(kind: DocumentGraphEdgeKind) -> u8 {
    match kind {
        DocumentGraphEdgeKind::Hierarchy => 0,
        DocumentGraphEdgeKind::Reference => 1,
        DocumentGraphEdgeKind::Attachment => 2,
    }
}

type GraphCounts = (
    HashMap<String, usize>,
    HashMap<String, usize>,
    HashMap<String, HashSet<String>>,
);

fn graph_counts(edges: &[DocumentGraphEdge]) -> GraphCounts {
    let mut inbound = HashMap::new();
    let mut outbound = HashMap::new();
    let mut neighbours = HashMap::<String, HashSet<String>>::new();
    for edge in edges {
        *inbound.entry(edge.target.clone()).or_default() += 1;
        *outbound.entry(edge.source.clone()).or_default() += 1;
        neighbours
            .entry(edge.source.clone())
            .or_default()
            .insert(edge.target.clone());
        neighbours
            .entry(edge.target.clone())
            .or_default()
            .insert(edge.source.clone());
    }
    (inbound, outbound, neighbours)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestLibrary(PathBuf);

    impl TestLibrary {
        fn new() -> Self {
            let id = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("locus-document-graph-{}-{id}", std::process::id()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn note(&self, path: &str, title: &str, body: &str) {
            let directory = self.0.join(path);
            fs::create_dir_all(&directory).unwrap();
            fs::write(
                directory.join("index.md"),
                format!("---\ntitle: {title}\n---\n{body}"),
            )
            .unwrap();
        }
    }

    impl Drop for TestLibrary {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn builds_hierarchy_references_and_attachments() {
        let library = TestLibrary::new();
        library.note(
            "projects",
            "Projects",
            "[[projects/locus]] [[projects/locus]] [Locus](locus://projects/locus)",
        );
        library.note("projects/locus", "Locus", "");
        fs::write(library.0.join("projects/locus/diagram.png"), [1, 2, 3]).unwrap();

        let graph = DocumentGraph::from_tree(&LibraryTree::load(Some(&library.0)));

        assert_eq!(
            graph
                .nodes
                .iter()
                .map(|node| (node.id.as_str(), node.kind))
                .collect::<Vec<_>>(),
            [
                ("projects", DocumentGraphNodeKind::Document),
                ("projects/locus", DocumentGraphNodeKind::Document),
                (
                    "attachment:projects%2Flocus:diagram%2Epng",
                    DocumentGraphNodeKind::Attachment
                ),
            ]
        );
        assert!(graph.edges.iter().any(|edge| {
            edge.kind == DocumentGraphEdgeKind::Hierarchy
                && edge.source == "projects"
                && edge.target == "projects/locus"
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.kind == DocumentGraphEdgeKind::Reference
                && edge.source == "projects"
                && edge.target == "projects/locus"
                && edge.count == 2
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.kind == DocumentGraphEdgeKind::Attachment
                && edge.target == "attachment:projects%2Flocus:diagram%2Epng"
        }));
    }

    #[test]
    fn ignores_references_in_code_and_to_missing_or_self_documents() {
        let library = TestLibrary::new();
        library.note(
            "notes",
            "Notes",
            "`[[missing]]`\n```md\n[[other]]\n```\n[[notes]]\n[[../outside]]",
        );

        let graph = DocumentGraph::from_tree(&LibraryTree::load(Some(&library.0)));

        assert!(graph.edges.is_empty());
    }

    #[test]
    fn attachment_filter_preserves_document_relationships() {
        let library = TestLibrary::new();
        library.note("parent", "Parent", "[[parent/child]]");
        library.note("parent/child", "Child", "");
        fs::write(library.0.join("parent/file.txt"), "attachment").unwrap();

        let graph =
            DocumentGraph::from_tree(&LibraryTree::load(Some(&library.0))).without_attachments();

        assert!(
            graph
                .nodes
                .iter()
                .all(|node| node.kind == DocumentGraphNodeKind::Document)
        );
        assert!(
            graph
                .edges
                .iter()
                .all(|edge| edge.kind != DocumentGraphEdgeKind::Attachment)
        );
    }

    #[test]
    fn opted_out_note_and_its_attachments_are_absent_from_the_graph() {
        let library = TestLibrary::new();
        library.note("visible", "Visible", "[[hidden]]");
        library.note("hidden", "Hidden", "[[visible]]");
        fs::write(
            library.0.join("hidden/index.md"),
            "---\ntitle: Hidden\ngraph: false\n---\n[[visible]]",
        )
        .unwrap();
        fs::write(library.0.join("hidden/file.txt"), "attachment").unwrap();

        let graph = DocumentGraph::from_tree(&LibraryTree::load(Some(&library.0)));

        assert_eq!(
            graph
                .nodes
                .iter()
                .map(|node| node.id.as_str())
                .collect::<Vec<_>>(),
            ["visible"]
        );
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn rebuilding_tracks_created_and_deleted_notes() {
        let library = TestLibrary::new();
        library.note("one", "One", "");
        let initial = DocumentGraph::from_tree(&LibraryTree::load(Some(&library.0)));
        assert_eq!(initial.nodes.len(), 1);

        library.note("two", "Two", "[[one]]");
        let after_creation = DocumentGraph::from_tree(&LibraryTree::load(Some(&library.0)));
        assert_eq!(
            after_creation
                .nodes
                .iter()
                .map(|node| node.id.as_str())
                .collect::<Vec<_>>(),
            ["one", "two"]
        );
        assert_eq!(after_creation.edges.len(), 1);

        fs::remove_dir_all(library.0.join("one")).unwrap();
        let after_deletion = DocumentGraph::from_tree(&LibraryTree::load(Some(&library.0)));
        assert_eq!(
            after_deletion
                .nodes
                .iter()
                .map(|node| node.id.as_str())
                .collect::<Vec<_>>(),
            ["two"]
        );
        assert!(after_deletion.edges.is_empty());
    }
}
