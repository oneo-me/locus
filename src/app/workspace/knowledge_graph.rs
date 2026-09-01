use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::{
    BorderStyle, Bounds, ColorSpace, Context, Edges, EventEmitter, FocusHandle, FontWeight, Hsla,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PathBuilder, PinchEvent, Pixels,
    Point, Render, Role, ScrollDelta, ScrollWheelEvent, SharedString, Task, TextAlign, TextRun,
    Window, canvas, div, font, hsla, linear_color_stop, linear_gradient, point, prelude::*, px,
    quad, rgb, size,
};
use gpui_component::ElementExt as _;
use rust_i18n::t;

use crate::document_library::{
    DocumentGraph, DocumentGraphEdgeKind, DocumentGraphNode, DocumentGraphNodeKind,
};
use crate::localization::Language;
use crate::theme::Theme;

mod layout;

use self::layout::{
    ALPHA_MIN, GraphPoint, LayoutEdge, advance_layout, advance_layout_scaled, graph_bounds,
    graph_node_radius, layout_graph, top_level_directory,
};

const MIN_ZOOM: f32 = 1.0 / 128.0;
const MAX_ZOOM: f32 = 8.0;
const FIT_PADDING: f32 = 36.0;
const LABEL_FADE_START: f32 = 0.35;
const LABEL_FADE_END: f32 = 0.65;
const LABEL_MAX_WIDTH: f32 = 132.0;
const CLICK_MOVE_TOLERANCE: f32 = 4.0;
const MOUSE_WHEEL_ZOOM_DISTANCE: f32 = 120.0;
const LAYOUT_TRANSITION_DURATION_MS: usize = 260;
const LAYOUT_TRANSITION_FRAME_MS: usize = 16;
const PHYSICS_FRAME_TIME_MS: f32 = 16.0;

#[derive(Clone, Copy, Debug)]
struct GraphCamera {
    x: f32,
    y: f32,
    zoom: f32,
}

impl Default for GraphCamera {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum PointerTarget {
    Canvas,
    Node(usize),
}

#[derive(Clone, Copy, Debug)]
struct PointerInteraction {
    target: PointerTarget,
    start: Point<Pixels>,
    last: Point<Pixels>,
    moved: bool,
}

#[derive(Clone)]
struct PaintNode {
    position: GraphPoint,
    radius: f32,
    fill: Hsla,
    label_color: Hsla,
    label: SharedString,
    opacity: f32,
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

#[derive(Clone)]
struct PaintEdge {
    source: usize,
    target: usize,
    color: Hsla,
    opacity: f32,
    width: f32,
}

#[derive(Clone)]
struct PaintSnapshot {
    nodes: Vec<PaintNode>,
    edges: Vec<PaintEdge>,
    camera: GraphCamera,
    background: Hsla,
}

pub(super) enum KnowledgeGraphEvent {
    OpenNote(String),
}

pub(super) struct KnowledgeGraph {
    focus_handle: FocusHandle,
    language: Language,
    source_graph: DocumentGraph,
    visible_graph: DocumentGraph,
    show_attachments: bool,
    positions: Vec<GraphPoint>,
    physics_velocity: Vec<GraphPoint>,
    physics_alpha: f32,
    edges: Vec<LayoutEdge>,
    adjacency: Vec<HashSet<usize>>,
    camera: GraphCamera,
    canvas_bounds: Option<Bounds<Pixels>>,
    hovered_node: Option<usize>,
    pointer: Option<PointerInteraction>,
    layout_generation: u64,
    layout_task: Option<Task<()>>,
    settle_generation: u64,
    settle_last_frame: Option<Instant>,
    user_moved_camera: bool,
}

impl EventEmitter<KnowledgeGraphEvent> for KnowledgeGraph {}

impl KnowledgeGraph {
    pub(super) fn new(
        graph: DocumentGraph,
        show_attachments: bool,
        language: Language,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut this = Self {
            focus_handle: cx.focus_handle(),
            language,
            source_graph: graph,
            visible_graph: DocumentGraph::default(),
            show_attachments,
            positions: Vec::new(),
            physics_velocity: Vec::new(),
            physics_alpha: 0.0,
            edges: Vec::new(),
            adjacency: Vec::new(),
            camera: GraphCamera::default(),
            canvas_bounds: None,
            hovered_node: None,
            pointer: None,
            layout_generation: 0,
            layout_task: None,
            settle_generation: 0,
            settle_last_frame: None,
            user_moved_camera: false,
        };
        this.schedule_layout(cx);
        this
    }

    pub(super) fn focus_handle(&self) -> FocusHandle {
        self.focus_handle.clone()
    }

    pub(super) fn set_graph(&mut self, graph: DocumentGraph, cx: &mut Context<Self>) {
        if self.source_graph == graph {
            return;
        }
        self.source_graph = graph;
        self.schedule_layout(cx);
    }

    pub(super) fn replace_graph(&mut self, graph: DocumentGraph, cx: &mut Context<Self>) {
        self.source_graph = graph;
        self.visible_graph = DocumentGraph::default();
        self.positions.clear();
        self.physics_velocity.clear();
        self.physics_alpha = 0.0;
        self.edges.clear();
        self.adjacency.clear();
        self.camera = GraphCamera::default();
        self.hovered_node = None;
        self.pointer = None;
        self.cancel_settle();
        self.user_moved_camera = false;
        self.schedule_layout(cx);
    }

    pub(super) fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        if self.language != language {
            self.language = language;
            cx.notify();
        }
    }

    pub(super) fn set_show_attachments(&mut self, show_attachments: bool, cx: &mut Context<Self>) {
        if self.show_attachments == show_attachments {
            return;
        }
        self.show_attachments = show_attachments;
        self.schedule_layout(cx);
    }

    fn schedule_layout(&mut self, cx: &mut Context<Self>) {
        let alpha = if self.positions.is_empty() { 1.0 } else { 0.35 };
        self.schedule_layout_with_alpha(alpha, cx);
    }

    fn schedule_layout_with_alpha(&mut self, alpha: f32, cx: &mut Context<Self>) {
        self.cancel_settle();
        self.layout_generation = self.layout_generation.wrapping_add(1);
        let generation = self.layout_generation;
        let graph = if self.show_attachments {
            self.source_graph.clone()
        } else {
            self.source_graph.without_attachments()
        };
        let previous_positions = self
            .visible_graph
            .nodes
            .iter()
            .zip(&self.positions)
            .map(|(node, position)| (node.id.clone(), *position))
            .collect::<HashMap<_, _>>();
        self.layout_task = Some(cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { layout_graph(graph, &previous_positions, alpha) })
                .await;
            let animation = this
                .update(cx, |this, cx| {
                    if this.layout_generation != generation {
                        return None;
                    }
                    let current_positions = this
                        .visible_graph
                        .nodes
                        .iter()
                        .zip(&this.positions)
                        .map(|(node, position)| (node.id.clone(), *position))
                        .collect::<HashMap<_, _>>();
                    let target_positions = result.positions;
                    let start_positions = result
                        .graph
                        .nodes
                        .iter()
                        .zip(&target_positions)
                        .map(|(node, target)| {
                            current_positions.get(&node.id).copied().unwrap_or(*target)
                        })
                        .collect::<Vec<_>>();
                    let should_animate =
                        start_positions
                            .iter()
                            .zip(&target_positions)
                            .any(|(start, target)| {
                                (start.x - target.x).abs() > 0.05
                                    || (start.y - target.y).abs() > 0.05
                            });
                    this.visible_graph = result.graph;
                    this.positions = if should_animate {
                        start_positions.clone()
                    } else {
                        target_positions.clone()
                    };
                    this.physics_velocity = vec![GraphPoint::default(); this.positions.len()];
                    this.physics_alpha = 0.0;
                    this.edges = result.edges;
                    this.adjacency = result.adjacency;
                    this.hovered_node = None;
                    this.pointer = None;
                    if !should_animate {
                        this.layout_task = None;
                    }
                    if !this.user_moved_camera {
                        this.fit_graph();
                    }
                    cx.notify();
                    should_animate.then_some((start_positions, target_positions))
                })
                .ok()
                .flatten();
            let Some((start_positions, target_positions)) = animation else {
                return;
            };
            let frame_count = LAYOUT_TRANSITION_DURATION_MS.div_ceil(LAYOUT_TRANSITION_FRAME_MS);
            for frame in 1..=frame_count {
                cx.background_executor()
                    .timer(Duration::from_millis(LAYOUT_TRANSITION_FRAME_MS as u64))
                    .await;
                let progress = frame as f32 / frame_count as f32;
                let progress = 1.0 - (1.0 - progress).powi(3);
                let active = this
                    .update(cx, |this, cx| {
                        if this.layout_generation != generation {
                            return false;
                        }
                        for ((position, start), target) in this
                            .positions
                            .iter_mut()
                            .zip(&start_positions)
                            .zip(&target_positions)
                        {
                            *position = interpolate_graph_point(*start, *target, progress);
                        }
                        if !this.user_moved_camera {
                            this.fit_graph();
                        }
                        if frame == frame_count {
                            this.layout_task = None;
                        }
                        cx.notify();
                        true
                    })
                    .unwrap_or(false);
                if !active {
                    break;
                }
            }
        }));
    }

    fn settle_after_node_release(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.cancel_settle();
        self.physics_alpha = self.physics_alpha.max(0.4);
        self.settle_last_frame = Some(Instant::now());
        let generation = self.settle_generation;
        self.schedule_settle_frame(generation, window, cx);
    }

    fn schedule_settle_frame(
        &mut self,
        generation: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.on_next_frame(window, move |this, window, cx| {
            if this.settle_generation != generation {
                return;
            }
            let now = Instant::now();
            let elapsed = this
                .settle_last_frame
                .replace(now)
                .map(|previous| now.duration_since(previous))
                .unwrap_or(Duration::from_millis(LAYOUT_TRANSITION_FRAME_MS as u64));
            let time_scale =
                (elapsed.as_secs_f32() * 1_000.0 / PHYSICS_FRAME_TIME_MS).clamp(0.25, 2.0);
            this.physics_alpha = advance_layout_scaled(
                &this.edges,
                &mut this.positions,
                &mut this.physics_velocity,
                this.physics_alpha,
                time_scale,
            );
            if this.physics_alpha > ALPHA_MIN {
                this.schedule_settle_frame(generation, window, cx);
            } else {
                this.physics_alpha = 0.0;
                this.physics_velocity.fill(GraphPoint::default());
                this.settle_last_frame = None;
            }
            cx.notify();
        });
    }

    fn cancel_settle(&mut self) {
        self.settle_generation = self.settle_generation.wrapping_add(1);
        self.settle_last_frame = None;
    }

    fn fit_graph(&mut self) {
        let Some(bounds) = self.canvas_bounds else {
            return;
        };
        let Some((minimum, maximum)) = graph_bounds(&self.positions) else {
            self.camera = GraphCamera::default();
            return;
        };
        let width = (maximum.x - minimum.x).max(1.0);
        let height = (maximum.y - minimum.y).max(1.0);
        let available_width = (f32::from(bounds.size.width) - FIT_PADDING * 2.0).max(1.0);
        let available_height = (f32::from(bounds.size.height) - FIT_PADDING * 2.0).max(1.0);
        let zoom = (available_width / width)
            .min(available_height / height)
            .clamp(MIN_ZOOM, MAX_ZOOM.min(1.0));
        self.camera = GraphCamera {
            x: -((minimum.x + maximum.x) / 2.0) * zoom,
            y: -((minimum.y + maximum.y) / 2.0) * zoom,
            zoom,
        };
    }

    fn screen_position(&self, position: GraphPoint) -> Option<Point<Pixels>> {
        let bounds = self.canvas_bounds?;
        Some(point(
            bounds.origin.x
                + bounds.size.width / 2.0
                + px(self.camera.x + position.x * self.camera.zoom),
            bounds.origin.y
                + bounds.size.height / 2.0
                + px(self.camera.y + position.y * self.camera.zoom),
        ))
    }

    fn hit_test(&self, position: Point<Pixels>) -> Option<usize> {
        self.visible_graph
            .nodes
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, node)| {
                let center = self.screen_position(self.positions[index])?;
                let radius = node_screen_radius(node, self.camera.zoom).max(6.0);
                let delta_x = f32::from(position.x - center.x);
                let delta_y = f32::from(position.y - center.y);
                (delta_x * delta_x + delta_y * delta_y <= radius * radius).then_some(index)
            })
    }

    fn handle_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self
            .hit_test(event.position)
            .map(PointerTarget::Node)
            .unwrap_or(PointerTarget::Canvas);
        self.pointer = Some(PointerInteraction {
            target,
            start: event.position,
            last: event.position,
            moved: false,
        });
        if matches!(target, PointerTarget::Node(_)) {
            self.layout_task = None;
            self.cancel_settle();
            self.physics_alpha = self.physics_alpha.max(0.4);
        }
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }

    fn handle_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.dragging() {
            let Some(mut pointer) = self.pointer else {
                return;
            };
            let was_moved = pointer.moved;
            pointer.moved |= f32::from(event.position.x - pointer.start.x).abs()
                > CLICK_MOVE_TOLERANCE
                || f32::from(event.position.y - pointer.start.y).abs() > CLICK_MOVE_TOLERANCE;
            if !pointer.moved {
                self.pointer = Some(pointer);
                return;
            }
            let previous = if was_moved {
                pointer.last
            } else {
                pointer.start
            };
            let delta_x = f32::from(event.position.x - previous.x);
            let delta_y = f32::from(event.position.y - previous.y);
            pointer.last = event.position;
            match pointer.target {
                PointerTarget::Canvas => {
                    self.camera.x += delta_x;
                    self.camera.y += delta_y;
                    self.user_moved_camera = true;
                }
                PointerTarget::Node(index) => {
                    self.user_moved_camera = true;
                    if let Some(position) = self.positions.get_mut(index) {
                        position.x += delta_x / self.camera.zoom;
                        position.y += delta_y / self.camera.zoom;
                    }
                    self.physics_alpha = advance_layout(
                        &self.edges,
                        &mut self.positions,
                        &mut self.physics_velocity,
                        Some(index),
                        self.physics_alpha.max(0.4),
                        0.25,
                        1,
                    );
                }
            }
            self.pointer = Some(pointer);
            cx.notify();
            return;
        }

        let hovered = self.hit_test(event.position);
        if self.hovered_node != hovered {
            self.hovered_node = hovered;
            cx.notify();
        }
    }

    fn handle_mouse_up(
        &mut self,
        event: &MouseUpEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pointer = self.pointer.take();
        let released_node = matches!(
            pointer,
            Some(PointerInteraction {
                target: PointerTarget::Node(_),
                ..
            })
        );
        if event.click_count >= 2
            && let Some(PointerInteraction {
                target: PointerTarget::Node(index),
                moved: false,
                ..
            }) = pointer
            && let Some(node) = self.visible_graph.nodes.get(index)
            && node.kind == DocumentGraphNodeKind::Document
        {
            cx.emit(KnowledgeGraphEvent::OpenNote(node.path.clone()));
        }
        if released_node {
            self.settle_after_node_release(window, cx);
        } else {
            cx.notify();
        }
    }

    fn handle_scroll_wheel(
        &mut self,
        event: &ScrollWheelEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let delta = match event.delta {
            ScrollDelta::Pixels(delta) => f32::from(delta.y),
            ScrollDelta::Lines(delta) => delta.y * 16.0,
        };
        self.zoom_at(event.position, wheel_zoom_factor(delta));
        cx.stop_propagation();
        cx.notify();
    }

    fn handle_pinch(&mut self, event: &PinchEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.zoom_at(event.position, (1.0 + event.delta).max(0.01));
        cx.stop_propagation();
        cx.notify();
    }

    fn zoom_at(&mut self, position: Point<Pixels>, factor: f32) {
        let Some(bounds) = self.canvas_bounds else {
            return;
        };
        let previous_zoom = self.camera.zoom;
        let next_zoom = (previous_zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        if (next_zoom - previous_zoom).abs() < f32::EPSILON {
            return;
        }
        let local_x = f32::from(position.x - bounds.origin.x - bounds.size.width / 2.0);
        let local_y = f32::from(position.y - bounds.origin.y - bounds.size.height / 2.0);
        let graph_x = (local_x - self.camera.x) / previous_zoom;
        let graph_y = (local_y - self.camera.y) / previous_zoom;
        self.camera.x = local_x - graph_x * next_zoom;
        self.camera.y = local_y - graph_y * next_zoom;
        self.camera.zoom = next_zoom;
        self.user_moved_camera = true;
    }

    fn paint_snapshot(&self, theme: &Theme) -> PaintSnapshot {
        let palette = graph_palette(theme.is_dark());
        let directory_colors = create_directory_graph_color_map(
            self.visible_graph
                .nodes
                .iter()
                .filter(|node| node.kind == DocumentGraphNodeKind::Document)
                .filter_map(|node| top_level_directory(&node.path)),
            theme.is_dark(),
        );
        let base_colors = self
            .visible_graph
            .nodes
            .iter()
            .map(|node| node_colors(node, &directory_colors, palette))
            .collect::<Vec<_>>();
        let highlighted = self.hovered_node.map(|index| {
            let mut highlighted = self.adjacency.get(index).cloned().unwrap_or_default();
            highlighted.insert(index);
            highlighted
        });
        let nodes = self
            .visible_graph
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| {
                let opacity = match self.hovered_node {
                    None => 1.0,
                    Some(hovered) if hovered == index => 1.0,
                    Some(hovered)
                        if self
                            .adjacency
                            .get(hovered)
                            .is_some_and(|adjacent| adjacent.contains(&index)) =>
                    {
                        0.8
                    }
                    Some(_) => 0.08,
                };
                let colors = base_colors[index];
                PaintNode {
                    position: self.positions.get(index).copied().unwrap_or_default(),
                    radius: node_screen_radius(node, self.camera.zoom),
                    fill: colors.fill,
                    label_color: colors.label,
                    label: graph_label(&node.title).into(),
                    opacity,
                }
            })
            .collect::<Vec<_>>();
        let edges = self
            .edges
            .iter()
            .map(|edge| {
                let related = highlighted.as_ref().is_none_or(|highlighted| {
                    highlighted.contains(&edge.source) && highlighted.contains(&edge.target)
                });
                let (opacity, width) = match edge.kind {
                    DocumentGraphEdgeKind::Hierarchy => (palette.hierarchy_edge_opacity, 1.0),
                    DocumentGraphEdgeKind::Reference => (palette.reference_edge_opacity, 0.85),
                    DocumentGraphEdgeKind::Attachment => (palette.attachment_edge_opacity, 0.8),
                };
                PaintEdge {
                    source: edge.source,
                    target: edge.target,
                    color: theme.foreground,
                    opacity: if related { opacity } else { 0.009 },
                    width,
                }
            })
            .collect();
        PaintSnapshot {
            nodes,
            edges,
            camera: self.camera,
            background: theme.background,
        }
    }
}

impl Render for KnowledgeGraph {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx).clone();
        let language = self.language;
        let document_count = self
            .visible_graph
            .nodes
            .iter()
            .filter(|node| node.kind == DocumentGraphNodeKind::Document)
            .count();
        if document_count == 0 {
            let message = if self.layout_task.is_some() {
                language.text("workspace.graph.loading")
            } else {
                language.text("workspace.graph.empty")
            };
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .px_4()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(message)
                .into_any_element();
        }

        let snapshot = Arc::new(self.paint_snapshot(&theme));
        let snapshot_for_prepaint = snapshot.clone();
        let font_family = theme.font_family.clone();
        let label_opacity = ((self.camera.zoom - LABEL_FADE_START)
            / (LABEL_FADE_END - LABEL_FADE_START))
            .clamp(0.0, 1.0);
        let graph = cx.entity();
        let hovered_openable = self.hovered_node.is_some_and(|index| {
            self.visible_graph
                .nodes
                .get(index)
                .is_some_and(|node| node.kind == DocumentGraphNodeKind::Document)
        });
        let aria_label = t!(
            "workspace.graph.accessibility",
            locale = language.locale(),
            nodes = self.visible_graph.nodes.len(),
            edges = self.visible_graph.edges.len()
        )
        .into_owned();

        div()
            .id("knowledge-graph")
            .role(Role::Image)
            .aria_label(aria_label)
            .key_context("KnowledgeGraph")
            .track_focus(&self.focus_handle)
            .relative()
            .size_full()
            .overflow_hidden()
            .bg(theme.background)
            .cursor_grab()
            .when(hovered_openable, |graph| graph.cursor_pointer())
            .on_prepaint(move |bounds, _, cx| {
                graph.update(cx, |graph, cx| {
                    if graph.canvas_bounds != Some(bounds) {
                        graph.canvas_bounds = Some(bounds);
                        if !graph.user_moved_camera {
                            graph.fit_graph();
                        }
                        cx.notify();
                    }
                });
            })
            .on_mouse_down(MouseButton::Left, cx.listener(Self::handle_mouse_down))
            .on_mouse_move(cx.listener(Self::handle_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::handle_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::handle_mouse_up))
            .on_scroll_wheel(cx.listener(Self::handle_scroll_wheel))
            .on_pinch(cx.listener(Self::handle_pinch))
            .child(
                canvas(
                    move |bounds, window, _| {
                        let labels = snapshot_for_prepaint
                            .nodes
                            .iter()
                            .map(|node| {
                                let opacity = node.opacity * label_opacity;
                                let center = graph_screen_position(
                                    node.position,
                                    snapshot_for_prepaint.camera,
                                    bounds,
                                );
                                if opacity <= 0.0
                                    || !node_is_visible(
                                        center,
                                        node.radius,
                                        LABEL_MAX_WIDTH / 2.0,
                                        bounds,
                                    )
                                {
                                    return None;
                                }
                                Some(shape_graph_label(
                                    &node.label,
                                    font_family.clone(),
                                    node.label_color.opacity(opacity),
                                    window,
                                ))
                            })
                            .collect::<Vec<_>>();
                        (snapshot_for_prepaint, labels, bounds)
                    },
                    move |_, (snapshot, labels, bounds), window, cx| {
                        paint_graph(&snapshot, &labels, bounds, window, cx);
                    },
                )
                .absolute()
                .inset_0(),
            )
            .into_any_element()
    }
}

fn paint_graph(
    snapshot: &PaintSnapshot,
    labels: &[Option<gpui::ShapedLine>],
    bounds: Bounds<Pixels>,
    window: &mut Window,
    cx: &mut gpui::App,
) {
    for edge in &snapshot.edges {
        let Some(source) = snapshot.nodes.get(edge.source) else {
            continue;
        };
        let Some(target) = snapshot.nodes.get(edge.target) else {
            continue;
        };
        let source = graph_screen_position(source.position, snapshot.camera, bounds);
        let target = graph_screen_position(target.position, snapshot.camera, bounds);
        if !segment_may_cross_bounds(source, target, bounds) {
            continue;
        }
        let mut builder = PathBuilder::stroke(px(edge.width));
        builder.move_to(source);
        builder.line_to(target);
        if let Ok(path) = builder.build() {
            window.paint_path(path, edge.color.opacity(edge.opacity));
        }
    }

    for (index, node) in snapshot.nodes.iter().enumerate() {
        let center = graph_screen_position(node.position, snapshot.camera, bounds);
        if !node_is_visible(center, node.radius, 0.0, bounds) {
            continue;
        }
        paint_graph_node_gradient(
            center,
            node.radius,
            node.fill,
            node.opacity,
            snapshot.background,
            window,
        );
        if let Some(label) = labels.get(index).and_then(Option::as_ref) {
            let origin = point(
                center.x - label.width() / 2.0,
                center.y + px(node.radius + 4.0),
            );
            let _ = label.paint(origin, px(14.0), TextAlign::Left, None, window, cx);
        }
    }
}

fn paint_graph_node_gradient(
    center: Point<Pixels>,
    radius: f32,
    fill: Hsla,
    opacity: f32,
    background: Hsla,
    window: &mut Window,
) {
    let highlight = mix_hsla(
        fill,
        Hsla {
            h: fill.h,
            s: 0.0,
            l: 1.0,
            a: 1.0,
        },
        0.22,
    );
    let rim = mix_hsla(
        fill,
        Hsla {
            h: fill.h,
            s: 0.0,
            l: 0.0,
            a: 1.0,
        },
        0.08,
    );
    let highlight = background.blend(highlight.opacity(opacity));
    let rim = background.blend(rim.opacity(opacity));
    let diameter = radius * 2.0;
    window.paint_quad(quad(
        Bounds::new(
            point(center.x - px(radius), center.y - px(radius)),
            size(px(diameter), px(diameter)),
        ),
        px(radius),
        linear_gradient(
            135.0,
            linear_color_stop(highlight, 0.0),
            linear_color_stop(rim, 1.0),
        )
        .color_space(ColorSpace::Oklab),
        Edges::default(),
        hsla(0.0, 0.0, 0.0, 0.0),
        BorderStyle::default(),
    ));
}

fn graph_screen_position(
    position: GraphPoint,
    camera: GraphCamera,
    bounds: Bounds<Pixels>,
) -> Point<Pixels> {
    point(
        bounds.origin.x + bounds.size.width / 2.0 + px(camera.x + position.x * camera.zoom),
        bounds.origin.y + bounds.size.height / 2.0 + px(camera.y + position.y * camera.zoom),
    )
}

fn node_is_visible(
    center: Point<Pixels>,
    radius: f32,
    padding: f32,
    bounds: Bounds<Pixels>,
) -> bool {
    let extent = radius + padding;
    bounds.intersects(&Bounds::new(
        point(center.x - px(extent), center.y - px(extent)),
        size(px(extent * 2.0), px(extent * 2.0)),
    ))
}

fn segment_may_cross_bounds(
    source: Point<Pixels>,
    target: Point<Pixels>,
    bounds: Bounds<Pixels>,
) -> bool {
    !((source.x < bounds.left() && target.x < bounds.left())
        || (source.x > bounds.right() && target.x > bounds.right())
        || (source.y < bounds.top() && target.y < bounds.top())
        || (source.y > bounds.bottom() && target.y > bounds.bottom()))
}

fn mix_hsla(start: Hsla, end: Hsla, amount: f32) -> Hsla {
    let amount = amount.clamp(0.0, 1.0);
    Hsla {
        h: start.h + (end.h - start.h) * amount,
        s: start.s + (end.s - start.s) * amount,
        l: start.l + (end.l - start.l) * amount,
        a: start.a + (end.a - start.a) * amount,
    }
}

fn interpolate_graph_point(start: GraphPoint, target: GraphPoint, progress: f32) -> GraphPoint {
    GraphPoint {
        x: start.x + (target.x - start.x) * progress,
        y: start.y + (target.y - start.y) * progress,
    }
}

fn shape_graph_label(
    label: &SharedString,
    font_family: SharedString,
    color: Hsla,
    window: &mut Window,
) -> gpui::ShapedLine {
    let shape = |value: SharedString| {
        let mut label_font = font(font_family.clone());
        label_font.weight = FontWeight::NORMAL;
        window.text_system().shape_line(
            value.clone(),
            px(11.0),
            &[TextRun {
                len: value.len(),
                font: label_font,
                color,
                ..Default::default()
            }],
            None,
        )
    };
    let full = shape(label.clone());
    if f32::from(full.width()) <= LABEL_MAX_WIDTH {
        return full;
    }

    let characters = label.chars().collect::<Vec<_>>();
    let mut minimum = 0;
    let mut maximum = characters.len();
    let mut fitted = shape("…".into());
    while minimum < maximum {
        let middle = (minimum + maximum).div_ceil(2);
        let candidate = format!("{}…", characters[..middle].iter().collect::<String>());
        let shaped = shape(candidate.into());
        if f32::from(shaped.width()) <= LABEL_MAX_WIDTH {
            minimum = middle;
            fitted = shaped;
        } else {
            maximum = middle - 1;
        }
    }
    fitted
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

fn node_screen_radius(node: &DocumentGraphNode, zoom: f32) -> f32 {
    let scale = (zoom / 0.4).min(1.0);
    (graph_node_radius(node) * scale).max(0.75)
}

fn graph_label(label: &str) -> String {
    label.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn wheel_zoom_factor(delta: f32) -> f32 {
    (delta * 1.5_f32.ln() / MOUSE_WHEEL_ZOOM_DISTANCE).exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpui_positive_wheel_delta_zooms_in() {
        assert!(wheel_zoom_factor(16.0) > 1.0);
        assert!(wheel_zoom_factor(-16.0) < 1.0);
        assert!((wheel_zoom_factor(16.0) * wheel_zoom_factor(-16.0) - 1.0).abs() < 0.000_1);
    }

    #[test]
    fn settled_layout_positions_transition_without_a_final_jump() {
        let start = GraphPoint { x: 10.0, y: 20.0 };
        let target = GraphPoint { x: 50.0, y: 80.0 };
        assert_eq!(interpolate_graph_point(start, target, 0.0), start);
        assert_eq!(interpolate_graph_point(start, target, 1.0), target);
        assert_eq!(
            interpolate_graph_point(start, target, 0.5),
            GraphPoint { x: 30.0, y: 50.0 }
        );
    }

    #[test]
    fn paint_culling_keeps_only_visible_or_crossing_geometry() {
        let bounds = Bounds::new(point(px(0.0), px(0.0)), size(px(100.0), px(100.0)));

        assert!(node_is_visible(point(px(50.0), px(50.0)), 5.0, 0.0, bounds));
        assert!(!node_is_visible(
            point(px(120.0), px(50.0)),
            5.0,
            0.0,
            bounds
        ));
        assert!(segment_may_cross_bounds(
            point(px(-20.0), px(50.0)),
            point(px(120.0), px(50.0)),
            bounds
        ));
        assert!(!segment_may_cross_bounds(
            point(px(-20.0), px(20.0)),
            point(px(-10.0), px(80.0)),
            bounds
        ));
    }

    #[test]
    fn node_screen_size_uses_one_zoom_curve_for_every_kind() {
        let document = DocumentGraphNode {
            id: "note".into(),
            kind: DocumentGraphNodeKind::Document,
            path: "note".into(),
            title: "Note".into(),
            directory: String::new(),
            inbound_count: 0,
            outbound_count: 0,
            degree: 0,
        };
        let attachment = DocumentGraphNode {
            kind: DocumentGraphNodeKind::Attachment,
            ..document.clone()
        };
        let document_ratio = node_screen_radius(&document, 0.2) / graph_node_radius(&document);
        let attachment_ratio =
            node_screen_radius(&attachment, 0.2) / graph_node_radius(&attachment);
        assert!((document_ratio - attachment_ratio).abs() < f32::EPSILON);
        assert_eq!(
            node_screen_radius(&document, 0.4),
            graph_node_radius(&document)
        );
        assert_eq!(
            node_screen_radius(&document, 2.0),
            graph_node_radius(&document)
        );
    }

    #[test]
    fn attachments_keep_the_legacy_orange_palette() {
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
}
