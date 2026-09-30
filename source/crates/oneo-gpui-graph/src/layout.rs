use std::collections::{HashMap, HashSet};

use crate::{GraphData, GraphEdgeStyle, GraphNode};

const ALPHA_DECAY: f32 = 0.022_762_78;
pub(super) const ALPHA_MIN: f32 = 0.001;
const POSITION_STRENGTH: f32 = 0.1;
const MANY_BODY_STRENGTH: f32 = -1_000.0;
const MANY_BODY_MINIMUM_DISTANCE: f32 = 30.0;
const COLLISION_RADIUS: f32 = 60.0;
const COLLISION_STRENGTH: f32 = 0.5;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct GraphPoint {
    pub(super) x: f32,
    pub(super) y: f32,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct LayoutEdge {
    pub(super) source: usize,
    pub(super) target: usize,
    pub(super) distance: f32,
    pub(super) light_style: GraphEdgeStyle,
    pub(super) dark_style: GraphEdgeStyle,
    strength: f32,
    bias: f32,
}

pub(super) struct LayoutResult {
    pub(super) graph: GraphData,
    pub(super) positions: Vec<GraphPoint>,
    pub(super) edges: Vec<LayoutEdge>,
    pub(super) adjacency: Vec<HashSet<usize>>,
}

pub(super) fn layout_graph(
    graph: GraphData,
    previous_positions: &HashMap<String, GraphPoint>,
    alpha: f32,
) -> LayoutResult {
    let node_indexes = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let raw_edges = graph
        .edges
        .iter()
        .map(|edge| {
            (
                node_indexes[edge.source.as_str()],
                node_indexes[edge.target.as_str()],
                edge,
            )
        })
        .collect::<Vec<_>>();
    let mut semantic_degrees = vec![0usize; graph.nodes.len()];
    for &(source, target, _) in &raw_edges {
        semantic_degrees[source] += 1;
        semantic_degrees[target] += 1;
    }
    let edges = raw_edges
        .into_iter()
        .map(|(source, target, edge)| LayoutEdge {
            source,
            target,
            distance: edge.distance,
            light_style: edge.light_style,
            dark_style: edge.dark_style,
            strength: edge.strength
                / semantic_degrees[source]
                    .min(semantic_degrees[target])
                    .max(1) as f32,
            bias: semantic_degrees[source] as f32
                / (semantic_degrees[source] + semantic_degrees[target]).max(1) as f32,
        })
        .collect::<Vec<_>>();
    let mut adjacency = vec![HashSet::new(); graph.nodes.len()];
    for edge in &edges {
        adjacency[edge.source].insert(edge.target);
        adjacency[edge.target].insert(edge.source);
    }
    let mut positions = initial_positions(&graph.nodes, &edges, previous_positions);
    simulate_layout(&edges, &mut positions, alpha);
    LayoutResult {
        graph,
        positions,
        edges,
        adjacency,
    }
}

fn initial_positions(
    nodes: &[GraphNode],
    edges: &[LayoutEdge],
    previous_positions: &HashMap<String, GraphPoint>,
) -> Vec<GraphPoint> {
    let mut clusters = nodes
        .iter()
        .filter_map(|node| node.cluster.as_deref())
        .collect::<Vec<_>>();
    clusters.sort_unstable();
    clusters.dedup();
    let cluster_indexes = clusters
        .iter()
        .enumerate()
        .map(|(index, cluster)| (*cluster, index))
        .collect::<HashMap<_, _>>();
    let node_count = nodes.len().max(1);
    let outer_radius = ((node_count as f32 * 60.0 * 60.0) / std::f32::consts::PI).sqrt();
    let mut positions = nodes
        .iter()
        .map(|node| {
            previous_positions
                .get(&node.id)
                .copied()
                .unwrap_or_else(|| {
                    seeded_position(
                        &node.id,
                        node.cluster
                            .as_deref()
                            .and_then(|cluster| cluster_indexes.get(cluster).copied()),
                        clusters.len(),
                        outer_radius,
                        node_count,
                    )
                })
        })
        .collect::<Vec<_>>();

    for (index, node) in nodes.iter().enumerate() {
        if previous_positions.contains_key(&node.id) {
            continue;
        }
        let neighbours = edges.iter().filter_map(|edge| {
            let neighbour = if edge.source == index {
                edge.target
            } else if edge.target == index {
                edge.source
            } else {
                return None;
            };
            previous_positions.get(&nodes[neighbour].id).copied()
        });
        let mut count = 0.0;
        let center = neighbours.fold(GraphPoint::default(), |mut center, position| {
            count += 1.0;
            center.x += position.x;
            center.y += position.y;
            center
        });
        if count > 0.0 {
            let angle = seeded_angle(&node.id);
            positions[index] = GraphPoint {
                x: center.x / count + angle.cos() * 24.0,
                y: center.y / count + angle.sin() * 24.0,
            };
        }
    }
    positions
}

fn simulate_layout(edges: &[LayoutEdge], positions: &mut [GraphPoint], alpha: f32) {
    if positions.len() < 2 {
        return;
    }
    let mut velocity = vec![GraphPoint::default(); positions.len()];
    let iterations = ((ALPHA_MIN / alpha.max(ALPHA_MIN)).ln() / (1.0 - ALPHA_DECAY).ln())
        .ceil()
        .max(1.0) as usize;
    advance_layout(
        edges,
        positions,
        &mut velocity,
        None,
        alpha,
        0.0,
        iterations,
    );
}

pub(super) fn advance_layout(
    edges: &[LayoutEdge],
    positions: &mut [GraphPoint],
    velocity: &mut [GraphPoint],
    pinned: Option<usize>,
    alpha: f32,
    alpha_target: f32,
    iterations: usize,
) -> f32 {
    advance_layout_with_options(
        edges,
        positions,
        velocity,
        alpha,
        AdvanceLayoutOptions {
            pinned,
            alpha_target,
            iterations,
            time_scale: 1.0,
        },
    )
}

pub(super) fn advance_layout_scaled(
    edges: &[LayoutEdge],
    positions: &mut [GraphPoint],
    velocity: &mut [GraphPoint],
    alpha: f32,
    time_scale: f32,
) -> f32 {
    advance_layout_with_options(
        edges,
        positions,
        velocity,
        alpha,
        AdvanceLayoutOptions {
            pinned: None,
            alpha_target: 0.0,
            iterations: 1,
            time_scale,
        },
    )
}

struct AdvanceLayoutOptions {
    pinned: Option<usize>,
    alpha_target: f32,
    iterations: usize,
    time_scale: f32,
}

fn advance_layout_with_options(
    edges: &[LayoutEdge],
    positions: &mut [GraphPoint],
    velocity: &mut [GraphPoint],
    mut alpha: f32,
    options: AdvanceLayoutOptions,
) -> f32 {
    debug_assert_eq!(velocity.len(), positions.len());
    let time_scale = options.time_scale.max(f32::EPSILON);
    let alpha_decay = 1.0 - (1.0 - ALPHA_DECAY).powf(time_scale);
    let velocity_decay = 0.6_f32.powf(time_scale);
    for _ in 0..options.iterations {
        alpha += (options.alpha_target - alpha) * alpha_decay;
        let scaled_alpha = alpha * time_scale;
        apply_position_force(positions, velocity, scaled_alpha);
        apply_link_force(edges, positions, velocity, scaled_alpha);
        apply_many_body_force(positions, velocity, scaled_alpha);
        apply_collision_force(positions, velocity, time_scale);
        for index in 0..positions.len() {
            if options.pinned == Some(index) {
                velocity[index] = GraphPoint::default();
                continue;
            }
            velocity[index].x *= velocity_decay;
            velocity[index].y *= velocity_decay;
            positions[index].x += velocity[index].x * time_scale;
            positions[index].y += velocity[index].y * time_scale;
        }
    }
    alpha
}

fn apply_position_force(positions: &[GraphPoint], velocity: &mut [GraphPoint], alpha: f32) {
    let strength = POSITION_STRENGTH * alpha;
    for (position, velocity) in positions.iter().zip(velocity) {
        velocity.x -= position.x * strength;
        velocity.y -= position.y * strength;
    }
}

fn apply_link_force(
    edges: &[LayoutEdge],
    positions: &[GraphPoint],
    velocity: &mut [GraphPoint],
    alpha: f32,
) {
    for edge in edges {
        let delta_x = positions[edge.target].x + velocity[edge.target].x
            - positions[edge.source].x
            - velocity[edge.source].x;
        let delta_y = positions[edge.target].y + velocity[edge.target].y
            - positions[edge.source].y
            - velocity[edge.source].y;
        let (delta_x, delta_y, distance) =
            nonzero_delta(delta_x, delta_y, edge.source, edge.target);
        let target_distance = edge.distance;
        let scale = (distance - target_distance) / distance * alpha * edge.strength;
        let force_x = delta_x * scale;
        let force_y = delta_y * scale;
        velocity[edge.target].x -= force_x * edge.bias;
        velocity[edge.target].y -= force_y * edge.bias;
        velocity[edge.source].x += force_x * (1.0 - edge.bias);
        velocity[edge.source].y += force_y * (1.0 - edge.bias);
    }
}

fn apply_many_body_force(positions: &[GraphPoint], velocity: &mut [GraphPoint], alpha: f32) {
    const MINIMUM_DISTANCE_SQUARED: f32 = MANY_BODY_MINIMUM_DISTANCE * MANY_BODY_MINIMUM_DISTANCE;
    for source in 0..positions.len() {
        for target in source + 1..positions.len() {
            let delta_x = positions[target].x - positions[source].x;
            let delta_y = positions[target].y - positions[source].y;
            let (delta_x, delta_y, _) = nonzero_delta(delta_x, delta_y, source, target);
            let mut distance_squared = delta_x * delta_x + delta_y * delta_y;
            if distance_squared < MINIMUM_DISTANCE_SQUARED {
                distance_squared = (MINIMUM_DISTANCE_SQUARED * distance_squared).sqrt();
            }
            let scale = MANY_BODY_STRENGTH * alpha / distance_squared.max(f32::EPSILON);
            let force_x = delta_x * scale;
            let force_y = delta_y * scale;
            velocity[source].x += force_x;
            velocity[source].y += force_y;
            velocity[target].x -= force_x;
            velocity[target].y -= force_y;
        }
    }
}

fn apply_collision_force(positions: &[GraphPoint], velocity: &mut [GraphPoint], time_scale: f32) {
    const DIAMETER: f32 = COLLISION_RADIUS * 2.0;
    let anticipated = positions
        .iter()
        .zip(velocity.iter())
        .map(|(position, velocity)| GraphPoint {
            x: position.x + velocity.x * time_scale,
            y: position.y + velocity.y * time_scale,
        })
        .collect::<Vec<_>>();
    let mut cells = HashMap::<(i32, i32), Vec<usize>>::new();
    for (index, position) in anticipated.iter().enumerate() {
        cells
            .entry((
                (position.x / DIAMETER).floor() as i32,
                (position.y / DIAMETER).floor() as i32,
            ))
            .or_default()
            .push(index);
    }
    for source in 0..anticipated.len() {
        let cell_x = (anticipated[source].x / DIAMETER).floor() as i32;
        let cell_y = (anticipated[source].y / DIAMETER).floor() as i32;
        for x in cell_x - 1..=cell_x + 1 {
            for y in cell_y - 1..=cell_y + 1 {
                for &target in cells.get(&(x, y)).into_iter().flatten() {
                    if target <= source {
                        continue;
                    }
                    let delta_x = anticipated[target].x - anticipated[source].x;
                    let delta_y = anticipated[target].y - anticipated[source].y;
                    let (delta_x, delta_y, distance) =
                        nonzero_delta(delta_x, delta_y, source, target);
                    if distance >= DIAMETER {
                        continue;
                    }
                    let scale =
                        (DIAMETER - distance) / distance * COLLISION_STRENGTH * 0.5 * time_scale;
                    let force_x = delta_x * scale;
                    let force_y = delta_y * scale;
                    velocity[source].x -= force_x;
                    velocity[source].y -= force_y;
                    velocity[target].x += force_x;
                    velocity[target].y += force_y;
                }
            }
        }
    }
}

fn nonzero_delta(
    mut delta_x: f32,
    mut delta_y: f32,
    source: usize,
    target: usize,
) -> (f32, f32, f32) {
    if delta_x.abs() < f32::EPSILON && delta_y.abs() < f32::EPSILON {
        let angle = ((source.wrapping_mul(31) ^ target.wrapping_mul(17)) as f32 + 0.5) * 1.618;
        delta_x = angle.cos() * 1.0e-6;
        delta_y = angle.sin() * 1.0e-6;
    }
    let distance = (delta_x * delta_x + delta_y * delta_y).sqrt();
    (delta_x, delta_y, distance)
}

fn seeded_position(
    id: &str,
    cluster_index: Option<usize>,
    cluster_count: usize,
    outer_radius: f32,
    node_count: usize,
) -> GraphPoint {
    let hash = hash_value(id);
    let local_angle = ((hash & 0xffff) as f32 / 0x10000 as f32) * std::f32::consts::TAU;
    let radial_sample = (((hash >> 16) & 0xffff) as f32 + 0.5) / 0x10000 as f32;
    let Some(cluster_index) = cluster_index.filter(|_| cluster_count > 1) else {
        let radius = radial_sample.sqrt() * outer_radius;
        return GraphPoint {
            x: local_angle.cos() * radius,
            y: local_angle.sin() * radius,
        };
    };
    let cluster_angle = cluster_index as f32 / cluster_count as f32 * std::f32::consts::TAU;
    let cluster_ring = outer_radius * 0.72;
    let nodes_per_cluster = (node_count as f32 / cluster_count as f32).max(1.0);
    let cluster_radius = ((nodes_per_cluster * 60.0 * 60.0) / std::f32::consts::PI)
        .sqrt()
        .mul_add(0.95, 0.0)
        .max(64.0);
    let local_radius = radial_sample.sqrt() * cluster_radius;
    GraphPoint {
        x: cluster_angle.cos() * cluster_ring + local_angle.cos() * local_radius,
        y: cluster_angle.sin() * cluster_ring + local_angle.sin() * local_radius,
    }
}

pub(super) fn graph_bounds(positions: &[GraphPoint]) -> Option<(GraphPoint, GraphPoint)> {
    let first = *positions.first()?;
    Some(
        positions
            .iter()
            .skip(1)
            .fold((first, first), |(mut minimum, mut maximum), position| {
                minimum.x = minimum.x.min(position.x);
                minimum.y = minimum.y.min(position.y);
                maximum.x = maximum.x.max(position.x);
                maximum.y = maximum.y.max(position.y);
                (minimum, maximum)
            }),
    )
}

fn hash_value(value: &str) -> u32 {
    value.chars().fold(2_166_136_261, |hash, character| {
        (hash ^ character as u32).wrapping_mul(16_777_619)
    })
}

fn seeded_angle(value: &str) -> f32 {
    (hash_value(value) as f32 / u32::MAX as f32) * std::f32::consts::TAU
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str) -> GraphNode {
        GraphNode::new(id, id)
    }

    #[test]
    fn deterministic_positions_use_explicit_clusters() {
        let nodes = [
            node("one").with_cluster("group"),
            node("two").with_cluster("group"),
        ];
        let first = initial_positions(&nodes, &[], &HashMap::new());
        let second = initial_positions(&nodes, &[], &HashMap::new());
        assert_eq!(first, second);
    }

    #[test]
    fn new_connected_node_starts_near_an_existing_neighbour() {
        let nodes = [node("one"), node("two")];
        let edges = [LayoutEdge {
            source: 0,
            target: 1,
            distance: 200.0,
            light_style: GraphEdgeStyle::default(),
            dark_style: GraphEdgeStyle::default(),
            strength: 1.0,
            bias: 0.5,
        }];
        let previous = HashMap::from([("one".to_owned(), GraphPoint { x: 50.0, y: 80.0 })]);
        let positions = initial_positions(&nodes, &edges, &previous);
        let delta_x = positions[1].x - previous["one"].x;
        let delta_y = positions[1].y - previous["one"].y;
        assert!((delta_x * delta_x + delta_y * delta_y).sqrt() <= 24.1);
    }

    #[test]
    fn pinned_node_pulls_its_neighbour_during_drag_reheat() {
        let edges = [LayoutEdge {
            source: 0,
            target: 1,
            distance: 200.0,
            light_style: GraphEdgeStyle::default(),
            dark_style: GraphEdgeStyle::default(),
            strength: 1.0,
            bias: 0.5,
        }];
        let mut positions = [
            GraphPoint { x: 400.0, y: 0.0 },
            GraphPoint { x: 0.0, y: 0.0 },
        ];
        let mut velocity = [GraphPoint::default(); 2];

        advance_layout(&edges, &mut positions, &mut velocity, Some(0), 0.4, 0.25, 1);

        assert_eq!(positions[0], GraphPoint { x: 400.0, y: 0.0 });
        assert!(positions[1].x > 0.0);
    }

    #[test]
    fn released_node_settles_incrementally_with_decaying_alpha() {
        let edges = [LayoutEdge {
            source: 0,
            target: 1,
            distance: 200.0,
            light_style: GraphEdgeStyle::default(),
            dark_style: GraphEdgeStyle::default(),
            strength: 1.0,
            bias: 0.5,
        }];
        let mut positions = [
            GraphPoint { x: 400.0, y: 0.0 },
            GraphPoint { x: 0.0, y: 0.0 },
        ];
        let mut velocity = [GraphPoint::default(); 2];
        let drag_alpha =
            advance_layout(&edges, &mut positions, &mut velocity, Some(0), 0.4, 0.25, 1);
        let release_start = positions;
        let mut alpha = advance_layout(
            &edges,
            &mut positions,
            &mut velocity,
            None,
            drag_alpha,
            0.0,
            1,
        );

        assert!(alpha < drag_alpha);
        assert!(alpha > ALPHA_MIN);
        assert_ne!(positions, release_start);
        let first_frame = positions;

        let mut frames = 1;
        while alpha > ALPHA_MIN && frames < 1_000 {
            alpha = advance_layout(&edges, &mut positions, &mut velocity, None, alpha, 0.0, 1);
            frames += 1;
        }

        assert!(frames > 100);
        assert!(alpha <= ALPHA_MIN);
        assert_ne!(positions, first_frame);
    }

    #[test]
    fn frame_scaled_alpha_decay_is_refresh_rate_independent() {
        let mut full_frame_positions = [GraphPoint::default()];
        let mut full_frame_velocity = [GraphPoint::default()];
        let full_frame_alpha = advance_layout_scaled(
            &[],
            &mut full_frame_positions,
            &mut full_frame_velocity,
            0.4,
            1.0,
        );

        let mut half_frame_positions = [GraphPoint::default()];
        let mut half_frame_velocity = [GraphPoint::default()];
        let half_frame_alpha = advance_layout_scaled(
            &[],
            &mut half_frame_positions,
            &mut half_frame_velocity,
            0.4,
            0.5,
        );
        let half_frame_alpha = advance_layout_scaled(
            &[],
            &mut half_frame_positions,
            &mut half_frame_velocity,
            half_frame_alpha,
            0.5,
        );

        assert!((full_frame_alpha - half_frame_alpha).abs() < 0.000_001);
    }

    #[test]
    fn reordering_nodes_keeps_positions_by_id() {
        let previous = HashMap::from([
            ("first".to_owned(), GraphPoint { x: 12.0, y: 45.0 }),
            ("second".to_owned(), GraphPoint { x: -30.0, y: 10.0 }),
        ]);
        let positions = initial_positions(&[node("second"), node("first")], &[], &previous);
        assert_eq!(positions, vec![previous["second"], previous["first"]]);
    }

    #[test]
    fn custom_edge_parameters_reach_the_force_layout_without_domain_kinds() {
        let graph = GraphData::new(
            vec![node("one"), node("two")],
            vec![
                crate::GraphEdge::new("one", "two").with_distance(70.0),
                crate::GraphEdge::new("one", "two")
                    .with_distance(220.0)
                    .with_strength(0.5),
            ],
        )
        .unwrap();
        let result = layout_graph(graph, &HashMap::new(), 1.0);
        assert_eq!(result.edges[0].distance, 70.0);
        assert_eq!(result.edges[1].distance, 220.0);
        assert_eq!(result.edges[0].strength * 0.5, result.edges[1].strength);
        assert!(
            result
                .positions
                .iter()
                .all(|point| point.x.is_finite() && point.y.is_finite())
        );
        assert_eq!(
            result.adjacency,
            vec![HashSet::from([1]), HashSet::from([0])]
        );
    }
}
