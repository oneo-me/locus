use crate::layout::GraphPoint;
use gpui::{
    BorderStyle, Bounds, ColorSpace, Edges, FontWeight, Hsla, PathBuilder, Pixels, Point,
    SharedString, TextAlign, TextRun, Window, font, hsla, linear_color_stop, linear_gradient,
    point, px, quad, size,
};

pub(super) const LABEL_MAX_WIDTH: f32 = 132.0;

#[derive(Clone, Copy, Debug)]
pub(super) struct GraphCamera {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) zoom: f32,
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

#[derive(Clone)]
pub(super) struct PaintNode {
    pub(super) position: GraphPoint,
    pub(super) radius: f32,
    pub(super) fill: Hsla,
    pub(super) label_color: Hsla,
    pub(super) label: SharedString,
    pub(super) opacity: f32,
}

#[derive(Clone)]
pub(super) struct PaintEdge {
    pub(super) source: usize,
    pub(super) target: usize,
    pub(super) color: Hsla,
    pub(super) opacity: f32,
    pub(super) width: f32,
}

#[derive(Clone)]
pub(super) struct PaintSnapshot {
    pub(super) nodes: Vec<PaintNode>,
    pub(super) edges: Vec<PaintEdge>,
    pub(super) camera: GraphCamera,
    pub(super) background: Hsla,
}

pub(super) fn paint_graph(
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

pub(super) fn graph_screen_position(
    position: GraphPoint,
    camera: GraphCamera,
    bounds: Bounds<Pixels>,
) -> Point<Pixels> {
    point(
        bounds.origin.x + bounds.size.width / 2.0 + px(camera.x + position.x * camera.zoom),
        bounds.origin.y + bounds.size.height / 2.0 + px(camera.y + position.y * camera.zoom),
    )
}

pub(super) fn node_is_visible(
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

pub(super) fn segment_may_cross_bounds(
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

pub(super) fn interpolate_graph_point(
    start: GraphPoint,
    target: GraphPoint,
    progress: f32,
) -> GraphPoint {
    GraphPoint {
        x: start.x + (target.x - start.x) * progress,
        y: start.y + (target.y - start.y) * progress,
    }
}

pub(super) fn shape_graph_label(
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

pub(super) fn node_screen_radius(node: &crate::GraphNode, zoom: f32) -> f32 {
    let scale = (zoom / 0.4).min(1.0);
    (node.radius * scale).max(0.75)
}

pub(super) fn graph_label(label: &str) -> String {
    label.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn node_radii_share_the_zoom_curve_and_labels_are_normalized() {
        let small = crate::GraphNode::new("small", "Small").with_radius(4.0);
        let large = crate::GraphNode::new("large", "Large").with_radius(12.0);
        assert_eq!(node_screen_radius(&small, 0.2), 2.0);
        assert_eq!(node_screen_radius(&large, 0.2), 6.0);
        assert_eq!(node_screen_radius(&large, 2.0), 12.0);
        assert_eq!(graph_label("  A\n B\tC  "), "A B C");
    }
}
