use super::note_atmosphere::{
    AtmosphereBackdrop, AtmosphereShape, NoteAtmosphere, Pattern, derive_note_atmosphere,
};
use super::{Locus, SelectedNote};
use crate::document_library::{LIBRARY_REFERENCE_SCHEME, LibraryNode, library_reference_path};
use crate::theme::Theme;
use crate::ui::{
    Icon, LocusMarkdownExtensions, Markdown, MarkdownLabels, MarkdownPresentation,
    MarkdownTypography, document_icon_image, toolbar_icon_button,
};
use gpui::{
    AnyElement, BorderStyle, Bounds, Div, Edges, FontWeight, MouseButton, ObjectFit, PathBuilder,
    Pixels, ScrollHandle, Window, canvas, div, linear_color_stop, linear_gradient, point,
    prelude::*, px, quad, transparent_black,
};
use gpui_component::scroll::Scrollbar;
use oneo_gpui_app_shell::PanelSide;
const NOTE_MAX_WIDTH: f32 = 832.0;
const ATMOSPHERE_HEIGHT: f32 = 268.0;
const SHAPE_HEIGHT: f32 = 208.0;

impl Locus {
    pub(super) fn render_note(
        &self,
        node: &LibraryNode,
        selected_note: Option<&SelectedNote>,
        left_control_clearance: f32,
        content_width: f32,
        cx: &mut gpui::Context<Self>,
    ) -> Div {
        let theme = Theme::global(cx).clone();
        let preferences = self.settings.read(cx).preferences();
        let markdown_text_font_family = preferences.resolved_ui_font_family().to_owned();
        let markdown_text_size = preferences.ui_font_size;
        let markdown_code_font_family = preferences.resolved_code_font_family().to_owned();
        let markdown_code_text_size = preferences.code_font_size;
        let language = preferences.language;
        let atmosphere = derive_note_atmosphere(&node.title, theme.is_dark());
        let locus = cx.weak_entity();
        let titlebar =
            self.render_note_titlebar(node, left_control_clearance, content_width, &theme, cx);
        let selected_note = selected_note.filter(|note| note.path == node.path);

        let mut article = div()
            .mx_auto()
            .w_full()
            .min_w_0()
            .max_w(px(NOTE_MAX_WIDTH))
            .px(px(16.0))
            .pt(px(72.0))
            .pb(px(56.0))
            .flex()
            .flex_col()
            .gap(px(22.0));
        if let Some(metadata) = note_metadata(node, atmosphere.panel, &theme) {
            article = article.child(metadata);
        }
        if selected_note
            .and_then(|note| note.error.as_deref())
            .is_some()
        {
            article = article.child(
                div()
                    .rounded(px(8.0))
                    .border_1()
                    .border_color(theme.danger.opacity(0.35))
                    .px(px(12.0))
                    .py(px(9.0))
                    .text_sm()
                    .text_color(theme.danger)
                    .child(language.text("workspace.note.read_error")),
            );
        }
        if let Some(document) = selected_note
            .and_then(|note| note.document.as_ref())
            .filter(|document| !document.is_empty())
        {
            let extensions =
                LocusMarkdownExtensions::new(&node.attachments, &self.library_tree, language);
            let labels = MarkdownLabels {
                copy_code: language.text("workspace.note.copy_code").into(),
                collapse_code: language.text("workspace.note.collapse_code").into(),
                expand_code: language.text("workspace.note.expand_code").into(),
            };
            article = article.child(
                Markdown::new(
                    document,
                    &node.directory,
                    MarkdownPresentation {
                        theme,
                        typography: MarkdownTypography {
                            text_font_family: &markdown_text_font_family,
                            text_size: markdown_text_size,
                            code_font_family: &markdown_code_font_family,
                            code_text_size: markdown_code_text_size,
                        },
                        labels,
                    },
                    self.note_text_selection.clone(),
                    self.note_text_focus.clone(),
                    self.content_pane.entity_id(),
                )
                .with_extensions(&extensions)
                .with_code_blocks(self.note_code_blocks.clone())
                .with_link_handler(move |url, _, cx| {
                    if !url.starts_with(LIBRARY_REFERENCE_SCHEME) {
                        return false;
                    }
                    if let Some(path) = library_reference_path(url) {
                        let path = path.to_owned();
                        let _ = locus.update(cx, |this, cx| this.open_library_reference(&path, cx));
                    }
                    true
                })
                .render(),
            );
        }

        let selection_for_scroll = self.note_text_selection.clone();
        let scroll_handle = self.note_scroll_handle.clone();
        let scroll = div()
            .id("note-scroll")
            .absolute()
            .inset_0()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.note_scroll_handle)
            .on_mouse_move(move |event, window, _| {
                if !selection_for_scroll.is_selecting() {
                    return;
                }
                let bounds = scroll_handle.bounds();
                let edge = px(28.0);
                let step = px(14.0);
                let current = scroll_handle.offset();
                let next_y = if event.position.y < bounds.top() + edge {
                    (current.y + step).min(Pixels::ZERO)
                } else if event.position.y > bounds.bottom() - edge {
                    (current.y - step).max(-scroll_handle.max_offset().y)
                } else {
                    current.y
                };
                if next_y != current.y {
                    scroll_handle.set_offset(point(current.x, next_y));
                    window.refresh();
                }
            })
            .child(
                div()
                    .relative()
                    .w_full()
                    .child(atmosphere_background(atmosphere))
                    .child(article),
            );
        div()
            .relative()
            .size_full()
            .overflow_hidden()
            .child(scroll)
            .child(titlebar)
            .child(
                div()
                    .absolute()
                    .top(px(super::TITLEBAR_HEIGHT))
                    .right_0()
                    .bottom_0()
                    .w(Scrollbar::width())
                    .child(Scrollbar::vertical(&self.note_scroll_handle).viewport_from_layout()),
            )
    }

    pub(super) fn open_library_reference(&mut self, path: &str, cx: &mut gpui::Context<Self>) {
        let Some((has_children, is_note)) = self
            .library_tree
            .node(path)
            .map(|node| (!node.children.is_empty(), node.document_path.is_some()))
        else {
            return;
        };
        if !is_note {
            return;
        }
        self.select_library_node(path.to_owned(), has_children, cx);
        self.scroll_to_selected_library_node = true;
        cx.notify();
    }

    fn render_note_titlebar(
        &self,
        node: &LibraryNode,
        left_control_clearance: f32,
        content_width: f32,
        theme: &Theme,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement + use<> {
        let copied = self.copied_library_node.as_deref() == Some(node.path.as_str());
        let language = self.language(cx);
        let (left_padding, right_padding) = note_titlebar_padding(
            content_width,
            self.app_shell.workspace().rendered_width(PanelSide::Left),
            left_control_clearance,
        );
        let copy_button = toolbar_icon_button(
            "copy-note-reference",
            if copied {
                language.text("workspace.note.reference_copied")
            } else {
                language.text("workspace.note.copy_reference")
            },
            if copied { Icon::Check } else { Icon::Copy },
            theme,
        )
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(|_, window, cx| {
            window.dispatch_action(Box::new(crate::CopyCurrentNoteReference), cx)
        });
        let edit_button = toolbar_icon_button(
            "edit-note",
            language.text("workspace.note.edit"),
            Icon::File,
            theme,
        )
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(|_, window, cx| window.dispatch_action(Box::new(crate::EditCurrentNote), cx));
        let open_directory_button = toolbar_icon_button(
            "open-note-directory",
            language.text("workspace.note.open_directory"),
            Icon::FolderOpen,
            theme,
        )
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(|_, window, cx| {
            window.dispatch_action(Box::new(crate::OpenCurrentNoteDirectory), cx)
        });
        let note_menu =
            self.render_note_menu(node, format!("note-menu-page-{}", node.path), theme, cx);

        self.window_drag_region("note-titlebar-region", cx)
            .occlude()
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .h(px(super::TITLEBAR_HEIGHT))
            .flex()
            .items_center()
            .child(note_titlebar_background(
                self.note_scroll_handle.clone(),
                theme.background,
                theme.border,
            ))
            .child(
                div()
                    .mx_auto()
                    .w_full()
                    .max_w(px(NOTE_MAX_WIDTH))
                    .h_full()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .pl(px(left_padding))
                    .pr(px(right_padding))
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .when_some(note_icon(node), |title, note_icon| title.child(note_icon))
                            .child(
                                div()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_ellipsis()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(node.title.clone()),
                            ),
                    )
                    .when(self.app_shell.fps_counter().visible(), |header| {
                        header.child(self.render_fps_counter(cx))
                    })
                    .child(copy_button)
                    .child(edit_button)
                    .child(open_directory_button)
                    .child(note_menu),
            )
    }
}

fn note_titlebar_padding(
    content_width: f32,
    left_sidebar_width: f32,
    left_control_clearance: f32,
) -> (f32, f32) {
    let centered_gutter = ((content_width - NOTE_MAX_WIDTH) / 2.0).max(0.0);
    let left = (left_control_clearance - left_sidebar_width - centered_gutter).max(16.0);
    let right_control_clearance =
        super::TITLEBAR_CONTROL_INSET + super::TITLEBAR_CONTROL_SIZE + super::TITLEBAR_CONTROL_GAP;
    let right = (right_control_clearance - centered_gutter).max(16.0);
    (left, right)
}

fn note_metadata(node: &LibraryNode, panel_color: gpui::Hsla, theme: &Theme) -> Option<Div> {
    let summary = node
        .summary
        .as_ref()
        .filter(|summary| !summary.trim().is_empty());
    if summary.is_none() && node.additional_metadata.is_empty() {
        return None;
    }

    let mut panel = div()
        .w_full()
        .rounded(px(8.0))
        .overflow_hidden()
        .bg(panel_color)
        .flex()
        .flex_col();
    if let Some(summary) = summary {
        panel = panel.child(
            div()
                .px(px(14.0))
                .py(px(8.0))
                .text_sm()
                .line_height(px(21.0))
                .child(summary.clone()),
        );
    }
    if !node.additional_metadata.is_empty() {
        let mut fields = div().py(px(4.0)).flex().flex_col();
        for (key, value) in &node.additional_metadata {
            fields = fields.child(
                div()
                    .w_full()
                    .min_w_0()
                    .flex()
                    .items_start()
                    .text_xs()
                    .line_height(px(18.0))
                    .child(
                        div()
                            .w(px(112.0))
                            .min_w_0()
                            .flex_none()
                            .px(px(14.0))
                            .py(px(6.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.muted_foreground)
                            .whitespace_normal()
                            .child(key.clone()),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .px(px(14.0))
                            .py(px(6.0))
                            .text_color(theme.foreground)
                            .whitespace_normal()
                            .child(format_yaml_metadata_value(value)),
                    ),
            );
        }
        panel = panel.child(fields);
    }
    Some(panel)
}

fn format_yaml_metadata_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(value) => value.clone(),
        serde_json::Value::Null => "null".to_owned(),
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            serde_json::to_string_pretty(value).expect("JSON values are serializable")
        }
    }
}

fn note_titlebar_background(
    handle: ScrollHandle,
    color: gpui::Hsla,
    border_color: gpui::Hsla,
) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            if handle.offset().y < px(-0.5) {
                window.paint_quad(quad(
                    bounds,
                    Pixels::ZERO,
                    color,
                    Edges {
                        bottom: px(1.0),
                        ..Default::default()
                    },
                    border_color,
                    BorderStyle::default(),
                ));
            }
        },
    )
    .absolute()
    .inset_0()
}

fn atmosphere_background(atmosphere: NoteAtmosphere) -> Div {
    let painted = atmosphere.clone();
    div()
        .absolute()
        .inset_0()
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .h(px(ATMOSPHERE_HEIGHT))
                .bg(linear_gradient(
                    180.0,
                    linear_color_stop(atmosphere.wash, 0.0),
                    linear_color_stop(atmosphere.wash.opacity(0.0), 1.0),
                )),
        )
        .child(
            canvas(
                |_, _, _| (),
                move |bounds, _, window, _| paint_atmosphere(bounds, &painted, window),
            )
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .h(px(ATMOSPHERE_HEIGHT)),
        )
}

fn paint_atmosphere(bounds: Bounds<Pixels>, atmosphere: &NoteAtmosphere, window: &mut Window) {
    paint_backdrop(bounds, &atmosphere.backdrop, window);
    let available_width = (f32::from(bounds.size.width) - 48.0).min(NOTE_MAX_WIDTH);
    let scale = (available_width / 360.0).min(SHAPE_HEIGHT / 100.0);
    let origin_x =
        f32::from(bounds.origin.x) + (f32::from(bounds.size.width) - 360.0 * scale) / 2.0;
    let origin_y = f32::from(bounds.origin.y);
    for shape in &atmosphere.shapes {
        paint_shape(shape, origin_x, origin_y, scale, window);
    }
}

fn paint_backdrop(bounds: Bounds<Pixels>, backdrop: &AtmosphereBackdrop, window: &mut Window) {
    match backdrop {
        AtmosphereBackdrop::None => {}
        AtmosphereBackdrop::Dots {
            spacing,
            offset_x,
            offset_y,
            radius,
            color,
        } => {
            let mut y = f32::from(bounds.origin.y) - *spacing + *offset_y;
            while y < f32::from(bounds.bottom()) {
                let fade = vertical_fade(y - f32::from(bounds.origin.y));
                let mut builder = PathBuilder::fill();
                let mut x = f32::from(bounds.origin.x) - *spacing + *offset_x;
                while x < f32::from(bounds.right()) {
                    let center = point(px(x + radius), px(y + radius));
                    builder.move_to(point(center.x + px(*radius), center.y));
                    builder.arc_to(
                        point(px(*radius), px(*radius)),
                        px(0.0),
                        false,
                        true,
                        point(center.x - px(*radius), center.y),
                    );
                    builder.arc_to(
                        point(px(*radius), px(*radius)),
                        px(0.0),
                        false,
                        true,
                        point(center.x + px(*radius), center.y),
                    );
                    builder.close();
                    x += spacing;
                }
                if let Ok(path) = builder.build() {
                    window.paint_path(path, color.opacity(fade));
                }
                y += spacing;
            }
        }
        AtmosphereBackdrop::Grid(pattern) => {
            paint_pattern(bounds, pattern, PatternKind::Grid, 0.0, window)
        }
        AtmosphereBackdrop::Lines(pattern) => {
            paint_pattern(bounds, pattern, PatternKind::Lines, 0.0, window)
        }
        AtmosphereBackdrop::Diagonal(pattern) => {
            paint_pattern(bounds, pattern, PatternKind::Diagonal, 0.0, window)
        }
        AtmosphereBackdrop::Diamonds(pattern) => {
            paint_pattern(bounds, pattern, PatternKind::Diamonds, 0.0, window)
        }
        AtmosphereBackdrop::Waves { pattern, amplitude } => {
            paint_pattern(bounds, pattern, PatternKind::Waves, *amplitude, window)
        }
    }
}

#[derive(Clone, Copy)]
enum PatternKind {
    Grid,
    Lines,
    Diagonal,
    Diamonds,
    Waves,
}

fn paint_pattern(
    bounds: Bounds<Pixels>,
    pattern: &Pattern,
    kind: PatternKind,
    amplitude: f32,
    window: &mut Window,
) {
    let spacing = pattern.spacing;
    let center = (
        f32::from(bounds.origin.x + bounds.size.width / 2.0),
        f32::from(bounds.origin.y + bounds.size.height / 2.0),
    );
    let mut y = f32::from(bounds.origin.y) - spacing * 3.0 + pattern.offset_y;
    while y < f32::from(bounds.bottom()) + spacing * 3.0 {
        let fade = vertical_fade(y - f32::from(bounds.origin.y));
        let mut builder = PathBuilder::stroke(px(pattern.stroke_width));
        let mut x = f32::from(bounds.origin.x) - spacing * 3.0 + pattern.offset_x;
        while x < f32::from(bounds.right()) + spacing * 3.0 {
            if matches!(kind, PatternKind::Waves) {
                let values = [
                    (x, y + spacing / 2.0),
                    (x + spacing / 4.0, y + spacing / 2.0 - amplitude),
                    (x + spacing / 2.0, y + spacing / 2.0),
                    (x + spacing * 3.0 / 4.0, y + spacing / 2.0 + amplitude),
                    (x + spacing, y + spacing / 2.0),
                ]
                .map(|value| rotate_point(value, center, pattern.rotate.to_radians()));
                builder.move_to(point(px(values[0].0), px(values[0].1)));
                builder.curve_to(
                    point(px(values[2].0), px(values[2].1)),
                    point(px(values[1].0), px(values[1].1)),
                );
                builder.curve_to(
                    point(px(values[4].0), px(values[4].1)),
                    point(px(values[3].0), px(values[3].1)),
                );
                x += spacing;
                continue;
            }
            let points: Vec<(f32, f32)> = match kind {
                PatternKind::Grid => vec![(x, y + spacing), (x, y), (x + spacing, y)],
                PatternKind::Lines => vec![(x, y), (x + spacing, y)],
                PatternKind::Diagonal => vec![(x, y), (x + spacing, y + spacing)],
                PatternKind::Diamonds => vec![
                    (x, y + spacing / 2.0),
                    (x + spacing / 2.0, y),
                    (x + spacing, y + spacing / 2.0),
                    (x + spacing / 2.0, y + spacing),
                    (x, y + spacing / 2.0),
                ],
                PatternKind::Waves => unreachable!(),
            };
            for (index, value) in points.into_iter().enumerate() {
                let value = rotate_point(value, center, pattern.rotate.to_radians());
                if index == 0 {
                    builder.move_to(point(px(value.0), px(value.1)));
                } else {
                    builder.line_to(point(px(value.0), px(value.1)));
                }
            }
            x += spacing;
        }
        if let Ok(path) = builder.build() {
            window.paint_path(path, pattern.color.opacity(fade));
        }
        y += spacing;
    }
}

fn paint_shape(
    shape: &AtmosphereShape,
    origin_x: f32,
    origin_y: f32,
    scale: f32,
    window: &mut Window,
) {
    let map = |x: f32, y: f32| (origin_x + x * scale, origin_y + y * scale);
    let center_y = match shape {
        AtmosphereShape::Disk { cy, .. }
        | AtmosphereShape::Ring { cy, .. }
        | AtmosphereShape::Arc { cy, .. } => *cy,
        AtmosphereShape::RoundedRect { y, height, .. } => *y + *height / 2.0,
        AtmosphereShape::Triangle { points, .. } => (points[0].1 + points[1].1 + points[2].1) / 3.0,
    };
    let alpha = shape_fade(center_y) * shape_horizontal_fade(shape_center_x(shape));
    match shape {
        AtmosphereShape::Disk {
            cx,
            cy,
            radius,
            color,
        } => {
            let (x, y) = map(cx - radius, cy - radius);
            let r = radius * scale;
            window.paint_quad(quad(
                Bounds::new(point(px(x), px(y)), gpui::size(px(r * 2.0), px(r * 2.0))),
                px(r),
                color.opacity(alpha),
                Pixels::ZERO,
                transparent_black(),
                BorderStyle::default(),
            ));
        }
        AtmosphereShape::Ring {
            cx,
            cy,
            radius,
            stroke_width,
            color,
        } => {
            let (x, y) = map(cx - radius, cy - radius);
            let r = radius * scale;
            window.paint_quad(quad(
                Bounds::new(point(px(x), px(y)), gpui::size(px(r * 2.0), px(r * 2.0))),
                px(r),
                transparent_black(),
                px(stroke_width * scale),
                color.opacity(alpha),
                BorderStyle::default(),
            ));
        }
        AtmosphereShape::RoundedRect {
            x,
            y,
            width,
            height,
            radius,
            rotate,
            color,
        } => {
            let (left, top) = map(*x, *y);
            let center = (left + width * scale / 2.0, top + height * scale / 2.0);
            let corners = [
                (left, top),
                (left + width * scale, top),
                (left + width * scale, top + height * scale),
                (left, top + height * scale),
            ];
            let mut builder = PathBuilder::fill();
            for (index, p) in corners
                .into_iter()
                .map(|p| rotate_point(p, center, rotate.to_radians()))
                .enumerate()
            {
                if index == 0 {
                    builder.move_to(point(px(p.0), px(p.1)))
                } else {
                    builder.line_to(point(px(p.0), px(p.1)))
                }
            }
            builder.close();
            if let Ok(path) = builder.build() {
                window.paint_path(path, color.opacity(alpha))
            }
            let _ = radius;
        }
        AtmosphereShape::Arc {
            cx,
            cy,
            radius,
            start,
            sweep,
            stroke_width,
            color,
        } => {
            let start_rad = start.to_radians();
            let end_rad = (start + sweep).to_radians();
            let (center_x, center_y) = map(*cx, *cy);
            let r = radius * scale;
            let mut builder = PathBuilder::stroke(px(stroke_width * scale));
            builder.move_to(point(
                px(center_x + start_rad.cos() * r),
                px(center_y + start_rad.sin() * r),
            ));
            builder.arc_to(
                point(px(r), px(r)),
                px(0.0),
                sweep.abs() > 180.0,
                *sweep >= 0.0,
                point(
                    px(center_x + end_rad.cos() * r),
                    px(center_y + end_rad.sin() * r),
                ),
            );
            if let Ok(path) = builder.build() {
                window.paint_path(path, color.opacity(alpha))
            }
        }
        AtmosphereShape::Triangle { points, color } => {
            let mut builder = PathBuilder::fill();
            for (index, p) in points.iter().map(|p| map(p.0, p.1)).enumerate() {
                if index == 0 {
                    builder.move_to(point(px(p.0), px(p.1)))
                } else {
                    builder.line_to(point(px(p.0), px(p.1)))
                }
            }
            builder.close();
            if let Ok(path) = builder.build() {
                window.paint_path(path, color.opacity(alpha))
            }
        }
    }
}

fn shape_center_x(shape: &AtmosphereShape) -> f32 {
    match shape {
        AtmosphereShape::Disk { cx, .. }
        | AtmosphereShape::Ring { cx, .. }
        | AtmosphereShape::Arc { cx, .. } => *cx,
        AtmosphereShape::RoundedRect { x, width, .. } => *x + *width / 2.0,
        AtmosphereShape::Triangle { points, .. } => (points[0].0 + points[1].0 + points[2].0) / 3.0,
    }
}
fn vertical_fade(y: f32) -> f32 {
    if y <= ATMOSPHERE_HEIGHT * 0.7 {
        1.0
    } else {
        (1.0 - (y - ATMOSPHERE_HEIGHT * 0.7) / (ATMOSPHERE_HEIGHT * 0.3)).clamp(0.0, 1.0)
    }
}
fn shape_fade(y: f32) -> f32 {
    if y <= 75.0 {
        1.0
    } else {
        (1.0 - (y - 75.0) / 25.0).clamp(0.0, 1.0)
    }
}
fn shape_horizontal_fade(x: f32) -> f32 {
    if x <= 122.4 {
        0.0
    } else if x >= 187.2 {
        1.0
    } else {
        (x - 122.4) / 64.8
    }
}
fn rotate_point(value: (f32, f32), center: (f32, f32), angle: f32) -> (f32, f32) {
    let x = value.0 - center.0;
    let y = value.1 - center.1;
    (
        center.0 + x * angle.cos() - y * angle.sin(),
        center.1 + x * angle.sin() + y * angle.cos(),
    )
}

fn note_icon(node: &LibraryNode) -> Option<AnyElement> {
    if let Some(image) = &node.icon_image {
        let fallback = node.icon.clone();
        return Some(
            document_icon_image(image)
                .size(px(22.0))
                .flex_none()
                .rounded(px(5.0))
                .object_fit(ObjectFit::Cover)
                .with_fallback(move || {
                    fallback
                        .clone()
                        .map(emoji_icon)
                        .unwrap_or_else(|| div().size(px(22.0)).into_any_element())
                })
                .into_any_element(),
        );
    }
    node.icon.clone().map(emoji_icon)
}
fn emoji_icon(value: String) -> AnyElement {
    div()
        .size(px(22.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(17.0))
        .line_height(px(22.0))
        .child(value)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_note_titlebar_stays_aligned_when_sidebars_are_hidden() {
        assert_eq!(note_titlebar_padding(1200.0, 0.0, 120.0), (16.0, 16.0));
    }

    #[test]
    fn narrow_note_titlebar_only_avoids_controls_that_overlap_content() {
        assert_eq!(note_titlebar_padding(700.0, 0.0, 120.0), (120.0, 45.0));
        assert_eq!(note_titlebar_padding(700.0, 250.0, 120.0), (16.0, 45.0));
    }

    #[test]
    fn formats_yaml_metadata_like_the_previous_reader() {
        assert_eq!(
            format_yaml_metadata_value(&serde_json::Value::String("active".to_owned())),
            "active"
        );
        assert_eq!(format_yaml_metadata_value(&serde_json::Value::Null), "null");
        assert_eq!(
            format_yaml_metadata_value(&serde_json::json!({ "name": "Oneo" })),
            "{\n  \"name\": \"Oneo\"\n}"
        );
    }
}
