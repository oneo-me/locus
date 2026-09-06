//! A GPUI relationship graph with force layout, pan, zoom, node dragging, and activation events.
//!
//! Keep an `Entity<GraphView>` in the owning view and give its container a bounded size.
//! The host initializes GPUI Component and places `Root` at the window boundary.
//! This crate depends only on GPUI and GPUI Component; IDs, labels, clusters, and colors
//! have no document, filesystem, or application meaning.
//!
//! ```
//! use oneo_gpui_graph::{GraphData, GraphEdge, GraphNode};
//!
//! let data = GraphData::new(
//!     vec![GraphNode::new("service", "API"), GraphNode::new("store", "Database")],
//!     vec![GraphEdge::new("service", "store").with_distance(120.0)],
//! )?;
//! assert_eq!(data.nodes()[0].id(), "service");
//! # Ok::<(), oneo_gpui_graph::GraphError>(())
//! ```
//!
//! Create the entity with `cx.new(|cx| GraphView::new(data, GraphLabels::default(), cx))`.
//! Subscribe to `GraphEvent::NodeActivated(id)` with `cx.subscribe`, retain the subscription,
//! and resolve the opaque ID in the host. Double-click activates an activatable node;
//! dragging never activates it. The viewport owns pointer focus; hosts can use `focus_handle`
//! to route focus and provide their own keyboard navigation or equivalent node list.
//! The viewport uses the `RelationshipGraph` key context.
//!
//! `set_graph` retains the camera and seeds layout from matching IDs. `replace_graph` resets
//! the camera and transient state for a different dataset. New snapshots cancel stale work;
//! a pending snapshot keeps the previous scene until ready and cannot be discarded by a node drag.
//! Layout runs in the background, with frame-driven settling after drag release. With GPUI
//! reduced motion enabled, final layouts are applied without transition or frame-driven settling.
//!
//! `GraphData::new` rejects empty/duplicate IDs, missing endpoints, self edges, nonfinite or
//! nonpositive radii/distances/widths, negative strengths, and opacities outside `0..=1`.
//! Disconnected graphs, isolated nodes, and parallel edges are accepted. Edges are straight,
//! undirected visual connections; parallel edges share the same segment. An empty snapshot
//! renders the supplied empty label. Validation errors belong to the host's error presentation.
//!
//! Nodes default to radius 6 and theme foreground colors. Edges default to distance 160,
//! strength multiplier 1, width 1, and foreground opacity 0.2. Supply light/dark node colors
//! and edge styles to express categories; cluster keys only seed layout and imply no hierarchy.
//! Force layout uses pairwise repulsion and is intended for moderate graph sizes, not unbounded datasets.
//!
//! Graph distances, radii, hit tolerances, and label metrics use logical pixels in the graph
//! coordinate system, independently of application UI zoom. Camera zoom is bounded to
//! `1/128..=8`; node radii stop growing above zoom 0.4, labels fade in across 0.35..0.65,
//! and labels are truncated to 132 logical pixels. Labels use the current theme font family.
//!
//! Run the standalone host with `cargo run -p oneo-gpui-graph --example relationships`.

mod labels;
mod layout;
mod model;
mod paint;
mod view;

pub use labels::GraphLabels;
pub use model::{GraphData, GraphEdge, GraphEdgeStyle, GraphError, GraphNode, GraphNodeColors};
pub use view::{GraphEvent, GraphView};
