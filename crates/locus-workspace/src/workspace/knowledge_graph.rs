use gpui::{
    Context, Entity, EventEmitter, FocusHandle, Hsla, Render, Subscription, Window, hsla,
    prelude::*, rgb,
};
use std::collections::HashMap;

use locus_library::{
    DocumentGraph, DocumentGraphEdgeKind, DocumentGraphNode, DocumentGraphNodeKind,
};
use locus_preferences::localization::Language;
use oneo_gpui_graph::{
    GraphData, GraphEdge, GraphEdgeStyle, GraphEvent, GraphLabels, GraphNode, GraphNodeColors,
    GraphView,
};

pub(super) enum KnowledgeGraphEvent {
    OpenNote(String),
}

pub(super) struct KnowledgeGraph {
    source_graph: DocumentGraph,
    show_attachments: bool,
    view: Entity<GraphView>,
    _subscription: Subscription,
}

impl EventEmitter<KnowledgeGraphEvent> for KnowledgeGraph {}

impl KnowledgeGraph {
    pub(super) fn new(
        graph: DocumentGraph,
        show_attachments: bool,
        language: Language,
        cx: &mut Context<Self>,
    ) -> Self {
        let data = graph_data(&graph, show_attachments);
        let view = cx.new(|cx| GraphView::new(data, graph_labels(language), cx));
        let subscription = cx.subscribe(&view, |this, _, event, cx| {
            let GraphEvent::NodeActivated(id) = event;
            if let Some(path) = note_path(&this.source_graph, id) {
                cx.emit(KnowledgeGraphEvent::OpenNote(path.to_owned()));
            }
        });
        Self {
            source_graph: graph,
            show_attachments,
            view,
            _subscription: subscription,
        }
    }

    pub(super) fn focus_handle(&self, cx: &gpui::App) -> FocusHandle {
        self.view.read(cx).focus_handle()
    }

    pub(super) fn set_graph(&mut self, graph: DocumentGraph, cx: &mut Context<Self>) {
        if self.source_graph == graph {
            return;
        }
        self.source_graph = graph;
        let data = graph_data(&self.source_graph, self.show_attachments);
        self.view.update(cx, |view, cx| view.set_graph(data, cx));
    }

    pub(super) fn replace_graph(&mut self, graph: DocumentGraph, cx: &mut Context<Self>) {
        self.source_graph = graph;
        let data = graph_data(&self.source_graph, self.show_attachments);
        self.view
            .update(cx, |view, cx| view.replace_graph(data, cx));
    }

    pub(super) fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.view
            .update(cx, |view, cx| view.set_labels(graph_labels(language), cx));
    }

    pub(super) fn set_show_attachments(&mut self, show: bool, cx: &mut Context<Self>) {
        if self.show_attachments == show {
            return;
        }
        self.show_attachments = show;
        let data = graph_data(&self.source_graph, show);
        self.view.update(cx, |view, cx| view.set_graph(data, cx));
    }
}

impl Render for KnowledgeGraph {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.view.clone()
    }
}

fn note_path<'a>(graph: &'a DocumentGraph, id: &str) -> Option<&'a str> {
    graph
        .nodes
        .iter()
        .find(|node| node.id == id && node.kind == DocumentGraphNodeKind::Document)
        .map(|node| node.path.as_str())
}

fn graph_labels(language: Language) -> GraphLabels {
    GraphLabels::new(
        language.text("workspace.graph.loading"),
        language.text("workspace.graph.empty"),
        move |nodes, edges| {
            language
                .text_with(
                    "workspace.graph.accessibility",
                    &[("nodes", nodes.to_string()), ("edges", edges.to_string())],
                )
                .into()
        },
    )
}

fn graph_data(graph: &DocumentGraph, show_attachments: bool) -> GraphData {
    let graph = if show_attachments {
        graph.clone()
    } else {
        graph.without_attachments()
    };
    let directories = || {
        graph
            .nodes
            .iter()
            .filter(|node| node.kind == DocumentGraphNodeKind::Document)
            .filter_map(|node| top_level_directory(&node.path))
    };
    let light_colors = create_directory_graph_color_map(directories(), false);
    let dark_colors = create_directory_graph_color_map(directories(), true);
    let nodes = graph
        .nodes
        .iter()
        .map(|node| {
            let light = node_colors(node, &light_colors, graph_palette(false));
            let dark = node_colors(node, &dark_colors, graph_palette(true));
            let mut rendered = GraphNode::new(node.id.clone(), node.title.clone())
                .with_radius(graph_node_radius(node))
                .with_colors(
                    GraphNodeColors::new(light.fill, light.label),
                    GraphNodeColors::new(dark.fill, dark.label),
                )
                .activatable(node.kind == DocumentGraphNodeKind::Document);
            if let Some((directory, _)) = node.path.split_once('/') {
                rendered = rendered.with_cluster(directory);
            }
            rendered
        })
        .collect();
    let edges = graph
        .edges
        .iter()
        .map(|edge| {
            let (distance, strength) = match edge.kind {
                DocumentGraphEdgeKind::Hierarchy => (160.0, 1.0),
                DocumentGraphEdgeKind::Reference => (200.0, 1.0),
                DocumentGraphEdgeKind::Attachment => (64.0, 0.5),
            };
            GraphEdge::new(edge.source.clone(), edge.target.clone())
                .with_distance(distance)
                .with_strength(strength)
                .with_style(edge_style(edge.kind, false), edge_style(edge.kind, true))
        })
        .collect();
    GraphData::new(nodes, edges)
        .expect("document projection must have unique nodes and valid relationships")
}

fn edge_style(kind: DocumentGraphEdgeKind, dark: bool) -> GraphEdgeStyle {
    let palette = graph_palette(dark);
    let (opacity, width) = match kind {
        DocumentGraphEdgeKind::Hierarchy => (palette.hierarchy_edge_opacity, 1.0),
        DocumentGraphEdgeKind::Reference => (palette.reference_edge_opacity, 0.85),
        DocumentGraphEdgeKind::Attachment => (palette.attachment_edge_opacity, 0.8),
    };
    GraphEdgeStyle::default()
        .with_opacity(opacity)
        .with_width(width)
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct NodeColors {
    fill: Hsla,
    label: Hsla,
}

#[derive(Clone, Copy)]
struct GraphPalette {
    attachment: NodeColors,
    document: NodeColors,
    document_isolated: NodeColors,
    attachment_edge_opacity: f32,
    hierarchy_edge_opacity: f32,
    reference_edge_opacity: f32,
}

fn graph_palette(dark: bool) -> GraphPalette {
    if dark {
        GraphPalette {
            attachment: NodeColors {
                fill: Hsla::from(rgb(0xd4b06a)),
                label: Hsla::from(rgb(0xead7a8)),
            },
            attachment_edge_opacity: 0.045,
            document: NodeColors {
                fill: Hsla::from(rgb(0x9aa7b3)),
                label: Hsla::from(rgb(0xd4dbe2)),
            },
            document_isolated: NodeColors {
                fill: Hsla::from(rgb(0x7d8894)),
                label: Hsla::from(rgb(0xb8c0c8)),
            },
            hierarchy_edge_opacity: 0.07,
            reference_edge_opacity: 0.055,
        }
    } else {
        GraphPalette {
            attachment: NodeColors {
                fill: Hsla::from(rgb(0xe0b35a)),
                label: Hsla::from(rgb(0x8a6a2a)),
            },
            attachment_edge_opacity: 0.0375,
            document: NodeColors {
                fill: Hsla::from(rgb(0x626a70)),
                label: Hsla::from(rgb(0x3f474d)),
            },
            document_isolated: NodeColors {
                fill: Hsla::from(rgb(0x7f878d)),
                label: Hsla::from(rgb(0x596168)),
            },
            hierarchy_edge_opacity: 0.06,
            reference_edge_opacity: 0.0475,
        }
    }
}

fn create_directory_graph_color_map<'a>(
    directories: impl Iterator<Item = &'a str>,
    dark: bool,
) -> HashMap<String, NodeColors> {
    let mut directories = directories
        .map(str::trim)
        .filter(|directory| !directory.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    directories.sort();
    directories.dedup();
    let count = directories.len();
    directories
        .into_iter()
        .enumerate()
        .map(|(index, directory)| {
            let hue = (208.0 + index as f32 * 360.0 / count as f32) % 360.0;
            let hue = (hue * 10.0).round() / 10.0;
            let hash = graph_color_hash(&directory);
            let colors = if dark {
                NodeColors {
                    fill: hsla(
                        hue / 360.0,
                        (34 + ((hash >> 8) % 18)) as f32 / 100.0,
                        (58 + ((hash >> 16) % 12)) as f32 / 100.0,
                        1.0,
                    ),
                    label: hsla(hue / 360.0, 0.24, 0.84, 1.0),
                }
            } else {
                NodeColors {
                    fill: hsla(
                        hue / 360.0,
                        (34 + ((hash >> 8) % 18)) as f32 / 100.0,
                        (56 + ((hash >> 16) % 12)) as f32 / 100.0,
                        1.0,
                    ),
                    label: hsla(hue / 360.0, 0.3, 0.34, 1.0),
                }
            };
            (directory, colors)
        })
        .collect()
}

fn node_colors(
    node: &DocumentGraphNode,
    directory_colors: &HashMap<String, NodeColors>,
    palette: GraphPalette,
) -> NodeColors {
    match node.kind {
        DocumentGraphNodeKind::Attachment => palette.attachment,
        DocumentGraphNodeKind::Document => top_level_directory(&node.path)
            .and_then(|directory| directory_colors.get(directory).copied())
            .unwrap_or({
                if node.degree == 0 {
                    palette.document_isolated
                } else {
                    palette.document
                }
            }),
    }
}

fn graph_color_hash(value: &str) -> u32 {
    value.chars().fold(2_166_136_261, |hash, character| {
        let scalar = character as u32;
        let code_unit = if scalar > 0xffff {
            0xd800 + ((scalar - 0x10000) >> 10)
        } else {
            scalar
        };
        (hash ^ code_unit).wrapping_mul(16_777_619)
    })
}

fn graph_node_radius(node: &DocumentGraphNode) -> f32 {
    match node.kind {
        DocumentGraphNodeKind::Attachment => 4.0,
        DocumentGraphNodeKind::Document => {
            (2.5 * ((node.degree + 1) as f32).sqrt()).clamp(6.0, 22.0)
        }
    }
}

fn top_level_directory(path: &str) -> Option<&str> {
    let normalized = path.trim_start_matches("./");
    let directory = normalized.split('/').next()?;
    (!directory.is_empty() && directory != "index.md").then_some(directory)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn document(id: &str, degree: usize) -> DocumentGraphNode {
        DocumentGraphNode {
            id: id.to_owned(),
            kind: DocumentGraphNodeKind::Document,
            path: id.to_owned(),
            title: id.to_owned(),
            directory: String::new(),
            inbound_count: 0,
            outbound_count: 0,
            degree,
        }
    }

    #[test]
    fn attachments_keep_the_product_orange_palette() {
        assert_eq!(
            graph_palette(false).attachment,
            NodeColors {
                fill: Hsla::from(rgb(0xe0b35a)),
                label: Hsla::from(rgb(0x8a6a2a)),
            }
        );
        assert_eq!(
            graph_palette(true).attachment,
            NodeColors {
                fill: Hsla::from(rgb(0xd4b06a)),
                label: Hsla::from(rgb(0xead7a8)),
            }
        );
    }

    #[test]
    fn first_level_directories_share_one_stable_generated_color() {
        let colors =
            create_directory_graph_color_map(["projects", "areas", "projects"].into_iter(), false);
        let projects = colors["projects"];
        let projects_hash = graph_color_hash("projects");

        assert_eq!(colors.len(), 2);
        assert_eq!(projects.fill.h, 28.0 / 360.0);
        assert_eq!(
            projects.fill.s,
            (34 + ((projects_hash >> 8) % 18)) as f32 / 100.0
        );
        assert_eq!(
            projects.fill.l,
            (56 + ((projects_hash >> 16) % 12)) as f32 / 100.0
        );

        let palette = graph_palette(false);
        let parent = DocumentGraphNode {
            id: "projects".into(),
            kind: DocumentGraphNodeKind::Document,
            path: "projects".into(),
            title: "Projects".into(),
            directory: String::new(),
            inbound_count: 0,
            outbound_count: 1,
            degree: 1,
        };
        let child = DocumentGraphNode {
            id: "projects/locus".into(),
            path: "projects/locus".into(),
            title: "Locus".into(),
            ..parent.clone()
        };
        assert_eq!(
            node_colors(&parent, &colors, palette),
            node_colors(&child, &colors, palette)
        );
    }
    #[test]
    fn node_size_follows_document_degree() {
        assert_eq!(graph_node_radius(&document("isolated", 0)), 6.0);
        assert!(graph_node_radius(&document("connected", 24)) > 6.0);
        assert_eq!(graph_node_radius(&document("hub", 10_000)), 22.0);
    }

    #[test]
    fn document_projection_maps_into_a_generic_graph_and_activation_returns_a_note_path() {
        use std::{
            fs,
            path::PathBuf,
            time::{SystemTime, UNIX_EPOCH},
        };
        struct LibraryFixture(PathBuf);
        impl Drop for LibraryFixture {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        let root = std::env::temp_dir().join(format!(
            "locus-graph-contract-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("projects/app")).unwrap();
        let fixture = LibraryFixture(root);
        fs::write(
            fixture.0.join("projects/index.md"),
            "---\ntitle: Projects\n---\n[[projects/app]]\n",
        )
        .unwrap();
        fs::write(
            fixture.0.join("projects/app/index.md"),
            "---\ntitle: App\n---\n",
        )
        .unwrap();
        fs::write(fixture.0.join("projects/app/guide.txt"), "Attachment").unwrap();
        let source = DocumentGraph::from_tree(&locus_library::LibraryTree::load(Some(&fixture.0)));
        let visible = graph_data(&source, true);
        assert_eq!(visible.nodes().len(), 3);
        assert_eq!(visible.edges().len(), 3);
        for original in &source.nodes {
            let node = visible
                .nodes()
                .iter()
                .find(|node| node.id() == original.id)
                .unwrap();
            assert_eq!(node.label(), original.title);
            assert_eq!(node.radius(), graph_node_radius(original));
            assert_eq!(
                node.is_activatable(),
                original.kind == DocumentGraphNodeKind::Document
            );
            if node.is_activatable() {
                assert_eq!(note_path(&source, node.id()), Some(original.path.as_str()));
            } else {
                assert_eq!(note_path(&source, node.id()), None);
                assert_eq!(
                    node.colors(false).unwrap().fill(),
                    graph_palette(false).attachment.fill
                );
                assert_eq!(
                    node.colors(true).unwrap().fill(),
                    graph_palette(true).attachment.fill
                );
            }
        }
        for (original, edge) in source.edges.iter().zip(visible.edges()) {
            let (distance, strength) = match original.kind {
                DocumentGraphEdgeKind::Hierarchy => (160.0, 1.0),
                DocumentGraphEdgeKind::Reference => (200.0, 1.0),
                DocumentGraphEdgeKind::Attachment => (64.0, 0.5),
            };
            assert_eq!((edge.distance(), edge.strength()), (distance, strength));
            assert_eq!(edge.style(false), edge_style(original.kind, false));
        }
        let filtered = graph_data(&source, false);
        assert_eq!(filtered.nodes().len(), 2);
        assert_eq!(filtered.edges().len(), 2);
        assert!(filtered.nodes().iter().all(GraphNode::is_activatable));
        assert_eq!(note_path(&source, "missing"), None);
    }
}
