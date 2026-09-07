use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui_kit::component::ElementExt as _;
use gpui_kit::{
    Bounds, Context, EventEmitter, FocusHandle, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, PinchEvent, Pixels, Point, Render, Role, ScrollDelta, ScrollWheelEvent, Task,
    Window, canvas, div, prelude::*,
};

use crate::paint::*;
use crate::{GraphData, GraphLabels, GraphNodeColors};
use gpui_kit::component::Theme;

use crate::layout::{
    ALPHA_MIN, GraphPoint, LayoutEdge, advance_layout, advance_layout_scaled, graph_bounds,
    layout_graph,
};

const MIN_ZOOM: f32 = 1.0 / 128.0;
const MAX_ZOOM: f32 = 8.0;
const FIT_PADDING: f32 = 36.0;
const LABEL_FADE_START: f32 = 0.35;
const LABEL_FADE_END: f32 = 0.65;
const CLICK_MOVE_TOLERANCE: f32 = 4.0;
const MOUSE_WHEEL_ZOOM_DISTANCE: f32 = 120.0;
const LAYOUT_TRANSITION_DURATION_MS: usize = 260;
const LAYOUT_TRANSITION_FRAME_MS: usize = 16;
const PHYSICS_FRAME_TIME_MS: f32 = 16.0;

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GraphEvent {
    NodeActivated(String),
}

/// Owns a graph viewport, background layout, pointer interaction, and animation tasks.
pub struct GraphView {
    focus_handle: FocusHandle,
    labels: GraphLabels,
    source_graph: GraphData,
    visible_graph: GraphData,
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

impl EventEmitter<GraphEvent> for GraphView {}

impl GraphView {
    pub fn new(graph: GraphData, labels: GraphLabels, cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            focus_handle: cx.focus_handle(),
            labels,
            source_graph: graph,
            visible_graph: GraphData::default(),
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

    pub fn focus_handle(&self) -> FocusHandle {
        self.focus_handle.clone()
    }

    /// Updates by stable ID while retaining the viewport and seeding from current positions.
    pub fn set_graph(&mut self, graph: GraphData, cx: &mut Context<Self>) {
        if self.source_graph == graph {
            return;
        }
        self.source_graph = graph;
        self.schedule_layout(cx);
    }

    /// Replaces the dataset and resets the viewport, layout, and transient interaction.
    pub fn replace_graph(&mut self, graph: GraphData, cx: &mut Context<Self>) {
        self.source_graph = graph;
        self.visible_graph = GraphData::default();
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

    pub fn set_labels(&mut self, labels: GraphLabels, cx: &mut Context<Self>) {
        self.labels = labels;
        cx.notify();
    }

    fn schedule_layout(&mut self, cx: &mut Context<Self>) {
        let alpha = if self.positions.is_empty() { 1.0 } else { 0.35 };
        self.schedule_layout_with_alpha(alpha, cx);
    }

    fn schedule_layout_with_alpha(&mut self, alpha: f32, cx: &mut Context<Self>) {
        self.cancel_settle();
        self.layout_generation = self.layout_generation.wrapping_add(1);
        let generation = self.layout_generation;
        let graph = self.source_graph.clone();
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
                    let should_animate = !cx.reduce_motion()
                        && start_positions
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
        cx.notify();
    }

    fn settle_after_node_release(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.cancel_settle();
        if cx.reduce_motion() {
            self.schedule_layout_with_alpha(self.physics_alpha.max(0.4), cx);
            return;
        }
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
            if cx.reduce_motion() {
                this.schedule_layout_with_alpha(this.physics_alpha, cx);
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
        Some(graph_screen_position(
            position,
            self.camera,
            self.canvas_bounds?,
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
        if matches!(target, PointerTarget::Node(_)) && self.source_graph != self.visible_graph {
            return;
        }
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
            && node.activatable
        {
            cx.emit(GraphEvent::NodeActivated(node.id.clone()));
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
                let colors = node
                    .colors(theme.is_dark())
                    .unwrap_or_else(|| GraphNodeColors::new(theme.foreground, theme.foreground));
                PaintNode {
                    position: self.positions.get(index).copied().unwrap_or_default(),
                    radius: node_screen_radius(node, self.camera.zoom),
                    fill: colors.fill,
                    label_color: colors.label,
                    label: graph_label(&node.label).into(),
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
                let style = if theme.is_dark() {
                    edge.dark_style
                } else {
                    edge.light_style
                };
                PaintEdge {
                    source: edge.source,
                    target: edge.target,
                    color: style.color.unwrap_or(theme.foreground),
                    opacity: if related {
                        style.opacity
                    } else {
                        style.opacity.min(0.009)
                    },
                    width: style.width,
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

impl Render for GraphView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx).clone();
        if self.visible_graph.nodes.is_empty() {
            let message = if self.layout_task.is_some() {
                self.labels.loading.clone()
            } else {
                self.labels.empty.clone()
            };
            return div()
                .id("relationship-graph")
                .role(Role::Image)
                .aria_label((self.labels.accessibility)(0, 0))
                .track_focus(&self.focus_handle)
                .bg(theme.background)
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
                .is_some_and(|node| node.activatable)
        });
        let aria_label = (self.labels.accessibility)(
            self.visible_graph.nodes.len(),
            self.visible_graph.edges.len(),
        );

        div()
            .id("relationship-graph")
            .role(Role::Image)
            .aria_label(aria_label)
            .key_context("RelationshipGraph")
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

fn wheel_zoom_factor(delta: f32) -> f32 {
    (delta * 1.5_f32.ln() / MOUSE_WHEEL_ZOOM_DISTANCE).exp()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{point, px};

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

    fn single_node(activatable: bool) -> GraphData {
        GraphData::new(
            vec![crate::GraphNode::new("service:api", "API").activatable(activatable)],
            vec![],
        )
        .unwrap()
    }

    #[gpui_kit::test]
    fn latest_snapshot_wins_and_replacement_resets_the_viewport(cx: &mut gpui_kit::TestAppContext) {
        cx.update(|cx| cx.set_reduce_motion(true));
        let view = cx.new(|cx| GraphView::new(single_node(true), GraphLabels::default(), cx));
        view.update(cx, |view, cx| {
            view.set_graph(
                GraphData::new(vec![crate::GraphNode::new("old", "Old")], vec![]).unwrap(),
                cx,
            );
            view.replace_graph(
                GraphData::new(vec![crate::GraphNode::new("latest", "Latest")], vec![]).unwrap(),
                cx,
            );
        });
        cx.run_until_parked();
        view.read_with(cx, |view, _| {
            assert_eq!(view.visible_graph.nodes[0].id, "latest");
            assert!(view.layout_task.is_none());
        });
        view.update(cx, |view, cx| {
            view.camera.x = 75.0;
            view.camera.zoom = 3.0;
            view.user_moved_camera = true;
            view.set_graph(single_node(true), cx);
            assert_eq!(view.camera.x, 75.0);
            view.replace_graph(GraphData::default(), cx);
            assert!(view.positions.is_empty());
            assert_eq!(view.camera.zoom, 1.0);
            assert!(!view.user_moved_camera);
        });
        cx.run_until_parked();
        view.read_with(cx, |view, _| assert!(view.visible_graph.nodes.is_empty()));
    }

    #[gpui_kit::test]
    fn pointer_activation_dragging_and_focus_use_the_public_event(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            cx.set_reduce_motion(true);
        });
        let view = cx.new(|cx| GraphView::new(single_node(true), GraphLabels::default(), cx));
        let mut events = cx.events(&view);
        let (_, cx) = cx
            .add_window_view(|window, cx| gpui_kit::component::Root::new(view.clone(), window, cx));
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let center = view.read_with(cx, |view, _| {
            view.screen_position(view.positions[0]).unwrap()
        });
        cx.simulate_mouse_down(center, MouseButton::Left, Default::default());
        cx.simulate_event(MouseUpEvent {
            position: center,
            button: MouseButton::Left,
            click_count: 2,
            ..Default::default()
        });
        assert_eq!(
            events.try_recv().unwrap(),
            GraphEvent::NodeActivated("service:api".to_owned())
        );
        assert!(events.try_recv().is_err());
        cx.update(|window, cx| assert!(view.read(cx).focus_handle.is_focused(window)));

        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let center = view.read_with(cx, |view, _| {
            view.screen_position(view.positions[0]).unwrap()
        });
        let dragged = point(center.x + px(35.0), center.y + px(20.0));
        cx.simulate_mouse_down(center, MouseButton::Left, Default::default());
        cx.simulate_mouse_move(dragged, MouseButton::Left, Default::default());
        view.read_with(cx, |view, _| assert!(view.user_moved_camera));
        cx.simulate_event(MouseUpEvent {
            position: dragged,
            button: MouseButton::Left,
            click_count: 2,
            ..Default::default()
        });
        assert!(events.try_recv().is_err());
        cx.run_until_parked();
        view.read_with(cx, |view, _| {
            assert!(view.layout_task.is_none());
            assert!(view.settle_last_frame.is_none());
        });

        view.update(cx, |view, cx| view.replace_graph(single_node(false), cx));
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let center = view.read_with(cx, |view, _| {
            view.screen_position(view.positions[0]).unwrap()
        });
        cx.simulate_mouse_down(center, MouseButton::Left, Default::default());
        cx.simulate_event(MouseUpEvent {
            position: center,
            button: MouseButton::Left,
            click_count: 2,
            ..Default::default()
        });
        assert!(events.try_recv().is_err());
    }

    #[gpui_kit::test]
    fn zoom_is_anchored_and_clamped_and_pending_updates_survive_input(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            cx.set_reduce_motion(true);
        });
        let view = cx.new(|cx| GraphView::new(single_node(true), GraphLabels::default(), cx));
        let (_, cx) = cx
            .add_window_view(|window, cx| gpui_kit::component::Root::new(view.clone(), window, cx));
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let center = view.read_with(cx, |view, _| {
            view.screen_position(view.positions[0]).unwrap()
        });
        cx.simulate_event(ScrollWheelEvent {
            position: center,
            delta: ScrollDelta::Lines(point(0.0, 1.0)),
            ..Default::default()
        });
        view.read_with(cx, |view, _| {
            assert!(view.camera.zoom > 1.0);
            let after = view.screen_position(view.positions[0]).unwrap();
            assert!(f32::from(after.x - center.x).abs() < 0.001);
            assert!(f32::from(after.y - center.y).abs() < 0.001);
        });
        view.update(cx, |view, _| {
            view.zoom_at(center, 1.0e6);
            assert_eq!(view.camera.zoom, MAX_ZOOM);
            view.zoom_at(center, 1.0e-9);
            assert_eq!(view.camera.zoom, MIN_ZOOM);
        });
        cx.update(|window, cx| {
            view.update(cx, |view, cx| {
                view.set_graph(
                    GraphData::new(vec![crate::GraphNode::new("replacement", "New")], vec![])
                        .unwrap(),
                    cx,
                );
                view.handle_mouse_down(
                    &MouseDownEvent {
                        position: center,
                        button: MouseButton::Left,
                        ..Default::default()
                    },
                    window,
                    cx,
                );
                assert!(view.layout_task.is_some());
            })
        });
        cx.run_until_parked();
        view.read_with(cx, |view, _| {
            assert_eq!(view.visible_graph.nodes[0].id, "replacement")
        });
    }
}
