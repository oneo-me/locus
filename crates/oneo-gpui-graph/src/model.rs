use std::collections::HashSet;
use std::fmt;

use gpui::Hsla;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GraphNodeColors {
    pub(crate) fill: Hsla,
    pub(crate) label: Hsla,
}

impl GraphNodeColors {
    pub fn new(fill: Hsla, label: Hsla) -> Self {
        Self { fill, label }
    }

    pub fn fill(&self) -> Hsla {
        self.fill
    }

    pub fn label(&self) -> Hsla {
        self.label
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GraphNode {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) cluster: Option<String>,
    pub(crate) radius: f32,
    pub(crate) colors: Option<(GraphNodeColors, GraphNodeColors)>,
    pub(crate) activatable: bool,
}

impl GraphNode {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            cluster: None,
            radius: 6.0,
            colors: None,
            activatable: true,
        }
    }

    pub fn with_cluster(mut self, cluster: impl Into<String>) -> Self {
        self.cluster = Some(cluster.into());
        self
    }

    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn with_colors(mut self, light: GraphNodeColors, dark: GraphNodeColors) -> Self {
        self.colors = Some((light, dark));
        self
    }

    pub fn activatable(mut self, activatable: bool) -> Self {
        self.activatable = activatable;
        self
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn cluster(&self) -> Option<&str> {
        self.cluster.as_deref()
    }

    pub fn radius(&self) -> f32 {
        self.radius
    }

    pub fn colors(&self, dark: bool) -> Option<GraphNodeColors> {
        self.colors
            .map(|(light, dark_colors)| if dark { dark_colors } else { light })
    }

    pub fn is_activatable(&self) -> bool {
        self.activatable
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GraphEdgeStyle {
    pub(crate) color: Option<Hsla>,
    pub(crate) opacity: f32,
    pub(crate) width: f32,
}

impl Default for GraphEdgeStyle {
    fn default() -> Self {
        Self {
            color: None,
            opacity: 0.2,
            width: 1.0,
        }
    }
}

impl GraphEdgeStyle {
    pub fn with_color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }

    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn color(&self) -> Option<Hsla> {
        self.color
    }

    pub fn opacity(&self) -> f32 {
        self.opacity
    }

    pub fn width(&self) -> f32 {
        self.width
    }

    fn is_valid(&self) -> bool {
        self.opacity.is_finite()
            && (0.0..=1.0).contains(&self.opacity)
            && self.width.is_finite()
            && self.width > 0.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GraphEdge {
    pub(crate) source: String,
    pub(crate) target: String,
    pub(crate) distance: f32,
    pub(crate) strength: f32,
    pub(crate) light_style: GraphEdgeStyle,
    pub(crate) dark_style: GraphEdgeStyle,
}

impl GraphEdge {
    pub fn new(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            distance: 160.0,
            strength: 1.0,
            light_style: GraphEdgeStyle::default(),
            dark_style: GraphEdgeStyle::default(),
        }
    }

    pub fn with_distance(mut self, distance: f32) -> Self {
        self.distance = distance;
        self
    }

    pub fn with_strength(mut self, strength: f32) -> Self {
        self.strength = strength;
        self
    }

    pub fn with_style(mut self, light: GraphEdgeStyle, dark: GraphEdgeStyle) -> Self {
        self.light_style = light;
        self.dark_style = dark;
        self
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn distance(&self) -> f32 {
        self.distance
    }

    pub fn strength(&self) -> f32 {
        self.strength
    }

    pub fn style(&self, dark: bool) -> GraphEdgeStyle {
        if dark {
            self.dark_style
        } else {
            self.light_style
        }
    }
}

/// A validated graph snapshot. IDs are opaque and stable across updates.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GraphData {
    pub(crate) nodes: Vec<GraphNode>,
    pub(crate) edges: Vec<GraphEdge>,
}

impl GraphData {
    pub fn new(nodes: Vec<GraphNode>, edges: Vec<GraphEdge>) -> Result<Self, GraphError> {
        let mut ids = HashSet::new();
        for node in &nodes {
            if node.id.is_empty() || !node.radius.is_finite() || node.radius <= 0.0 {
                return Err(GraphError::InvalidNode(node.id.clone()));
            }
            if !ids.insert(node.id.as_str()) {
                return Err(GraphError::DuplicateNode(node.id.clone()));
            }
        }
        for edge in &edges {
            for id in [&edge.source, &edge.target] {
                if !ids.contains(id.as_str()) {
                    return Err(GraphError::MissingNode(id.clone()));
                }
            }
            if edge.source == edge.target {
                return Err(GraphError::SelfEdge(edge.source.clone()));
            }
            if !edge.distance.is_finite()
                || edge.distance <= 0.0
                || !edge.strength.is_finite()
                || edge.strength < 0.0
                || !edge.light_style.is_valid()
                || !edge.dark_style.is_valid()
            {
                return Err(GraphError::InvalidEdge {
                    source: edge.source.clone(),
                    target: edge.target.clone(),
                });
            }
        }
        Ok(Self { nodes, edges })
    }

    pub fn nodes(&self) -> &[GraphNode] {
        &self.nodes
    }

    pub fn edges(&self) -> &[GraphEdge] {
        &self.edges
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GraphError {
    InvalidNode(String),
    DuplicateNode(String),
    MissingNode(String),
    SelfEdge(String),
    InvalidEdge { source: String, target: String },
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNode(id) => write!(
                f,
                "node {id:?} requires a nonempty ID and a finite positive radius"
            ),
            Self::DuplicateNode(id) => write!(f, "duplicate node ID {id:?}"),
            Self::MissingNode(id) => write!(f, "edge references missing node {id:?}"),
            Self::SelfEdge(id) => write!(f, "self edges are not supported for node {id:?}"),
            Self::InvalidEdge { source, target } => write!(
                f,
                "invalid geometry or style for edge {source:?} to {target:?}"
            ),
        }
    }
}

impl std::error::Error for GraphError {}
