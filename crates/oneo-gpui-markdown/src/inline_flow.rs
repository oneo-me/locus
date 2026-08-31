use std::{
    ops::Range,
    sync::{Arc, Mutex},
};

use gpui::{
    AnyElement, App, AvailableSpace, Bounds, Element, ElementId, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, LineFragment, Pixels, SharedString, Size,
    StyledText, TextRun, TextStyle, WhiteSpace, Window, point, size,
};

use crate::selection::{SelectableText, TextInteraction};

const ELEMENT_LEN: usize = 1;

pub(super) struct InlineTextItem {
    pub id: SharedString,
    pub text: SharedString,
    pub runs: Vec<TextRun>,
    pub document_range: Range<usize>,
    pub links: Vec<(Range<usize>, String)>,
}

pub(super) enum InlineFlowItem {
    Text(InlineTextItem),
    Element(AnyElement),
}

pub(super) struct InlineFlow {
    id: ElementId,
    items: Vec<InlineFlowItem>,
    selection_color: gpui::Hsla,
    interaction: TextInteraction,
}

#[derive(Clone, Default)]
pub(super) struct InlineFlowLayoutState {
    layout: Arc<Mutex<Option<InlineFlowLayout>>>,
}

#[derive(Clone, Default)]
struct InlineFlowLayout {
    fragments: Vec<PositionedFragment>,
    size: Size<Pixels>,
}

#[derive(Clone)]
enum PositionedFragment {
    Text {
        item_index: usize,
        origin: gpui::Point<Pixels>,
        size: Size<Pixels>,
        source_range: Range<usize>,
    },
    Element {
        item_index: usize,
        origin: gpui::Point<Pixels>,
        size: Size<Pixels>,
    },
}

struct FragmentLayout {
    item_index: usize,
    kind: FragmentKind,
    size: Size<Pixels>,
    source_range: Range<usize>,
}

enum FragmentKind {
    Text,
    Element,
}

impl InlineFlow {
    pub(super) fn new(
        id: impl Into<ElementId>,
        items: Vec<InlineFlowItem>,
        selection_color: gpui::Hsla,
        interaction: TextInteraction,
    ) -> Self {
        Self {
            id: id.into(),
            items,
            selection_color,
            interaction,
        }
    }
}

impl IntoElement for InlineFlow {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for InlineFlow {
    type RequestLayoutState = InlineFlowLayoutState;
    type PrepaintState = Vec<AnyElement>;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let element_sizes = self
            .items
            .iter_mut()
            .map(|item| match item {
                InlineFlowItem::Text(_) => None,
                InlineFlowItem::Element(element) => Some(element.layout_as_root(
                    size(AvailableSpace::MaxContent, AvailableSpace::MinContent),
                    window,
                    cx,
                )),
            })
            .collect::<Vec<_>>();
        let items = measure_items(&self.items);
        let state = InlineFlowLayoutState::default();
        let layout = state.layout.clone();
        let layout_id = window.request_measured_layout(
            Default::default(),
            move |known, available, window, _| {
                let text_style = window.text_style();
                let wrap_width = if text_style.white_space == WhiteSpace::Normal {
                    known.width.or(match available.width {
                        AvailableSpace::Definite(width) => Some(width),
                        _ => None,
                    })
                } else {
                    None
                };
                let measured = layout_flow(&items, &element_sizes, &text_style, wrap_width, window);
                let size = measured.size;
                if let Ok(mut layout) = layout.lock() {
                    *layout = Some(measured);
                }
                size
            },
        );

        (layout_id, state)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let fragments = state
            .layout
            .lock()
            .ok()
            .and_then(|layout| layout.as_ref().map(|layout| layout.fragments.clone()))
            .unwrap_or_default();
        let mut text_elements = Vec::new();

        for fragment in fragments {
            match fragment {
                PositionedFragment::Text {
                    item_index,
                    origin,
                    size: fragment_size,
                    source_range,
                } => {
                    let InlineFlowItem::Text(item) = &self.items[item_index] else {
                        continue;
                    };
                    let text = SharedString::from(item.text[source_range.clone()].to_owned());
                    let runs = slice_runs(&item.runs, source_range.clone());
                    let links = slice_ranges(&item.links, source_range.clone());
                    let document_range = item.document_range.start + source_range.start
                        ..item.document_range.start + source_range.end;
                    let mut element = SelectableText::new(
                        format!("{}-{}", item.id, source_range.start),
                        text.clone(),
                        StyledText::new(text).with_runs(runs),
                        document_range,
                        links,
                        self.selection_color,
                        self.interaction.clone(),
                    )
                    .into_any_element();
                    element.prepaint_as_root(
                        bounds.origin + origin,
                        size(
                            AvailableSpace::Definite(fragment_size.width),
                            AvailableSpace::Definite(fragment_size.height),
                        ),
                        window,
                        cx,
                    );
                    text_elements.push(element);
                }
                PositionedFragment::Element {
                    item_index,
                    origin,
                    size: fragment_size,
                } => {
                    let InlineFlowItem::Element(element) = &mut self.items[item_index] else {
                        continue;
                    };
                    element.prepaint_as_root(
                        bounds.origin + origin,
                        size(
                            AvailableSpace::Definite(fragment_size.width),
                            AvailableSpace::Definite(fragment_size.height),
                        ),
                        window,
                        cx,
                    );
                }
            }
        }

        text_elements
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        text_elements: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        for element in text_elements {
            element.paint(window, cx);
        }
        for item in &mut self.items {
            if let InlineFlowItem::Element(element) = item {
                element.paint(window, cx);
            }
        }
    }
}

#[derive(Clone)]
enum MeasureItem {
    Text {
        text: SharedString,
        runs: Vec<TextRun>,
    },
    Element,
}

impl MeasureItem {
    fn len(&self) -> usize {
        match self {
            Self::Text { text, .. } => text.len(),
            Self::Element => ELEMENT_LEN,
        }
    }
}

fn measure_items(items: &[InlineFlowItem]) -> Vec<MeasureItem> {
    items
        .iter()
        .map(|item| match item {
            InlineFlowItem::Text(item) => MeasureItem::Text {
                text: item.text.clone(),
                runs: item.runs.clone(),
            },
            InlineFlowItem::Element(_) => MeasureItem::Element,
        })
        .collect()
}

fn layout_flow(
    items: &[MeasureItem],
    element_sizes: &[Option<Size<Pixels>>],
    text_style: &TextStyle,
    wrap_width: Option<Pixels>,
    window: &mut Window,
) -> InlineFlowLayout {
    let line_height = window.line_height();
    let font_size = text_style.font_size.to_pixels(window.rem_size());
    let ranges = line_ranges(items, element_sizes, text_style, wrap_width, window);
    let mut fragments = Vec::new();
    let mut max_width = Pixels::ZERO;
    let mut y = Pixels::ZERO;

    for line_range in ranges {
        let mut line_fragments = Vec::new();
        let mut line_width = Pixels::ZERO;
        let mut actual_height = line_height;
        let mut item_start = 0;

        for (item_index, item) in items.iter().enumerate() {
            let item_end = item_start + item.len();
            if item_end <= line_range.start {
                item_start = item_end;
                continue;
            }
            if item_start >= line_range.end {
                break;
            }
            match item {
                MeasureItem::Text { text, runs } => {
                    let start = line_range.start.max(item_start) - item_start;
                    let end = line_range.end.min(item_end) - item_start;
                    if start < end {
                        let subtext = SharedString::from(text[start..end].to_owned());
                        let subruns = slice_runs(runs, start..end);
                        let shaped = window
                            .text_system()
                            .shape_line(subtext, font_size, &subruns, None);
                        let size = size(shaped.width(), line_height);
                        line_width += size.width;
                        line_fragments.push(FragmentLayout {
                            item_index,
                            kind: FragmentKind::Text,
                            size,
                            source_range: start..end,
                        });
                    }
                }
                MeasureItem::Element => {
                    if line_range.start <= item_start && item_end <= line_range.end {
                        let mut size = element_sizes[item_index].unwrap_or_default();
                        if let Some(wrap_width) = wrap_width {
                            size.width = size.width.min(wrap_width);
                        }
                        line_width += size.width;
                        actual_height = actual_height.max(size.height);
                        line_fragments.push(FragmentLayout {
                            item_index,
                            kind: FragmentKind::Element,
                            size,
                            source_range: 0..ELEMENT_LEN,
                        });
                    }
                }
            }
            item_start = item_end;
        }

        let mut x = Pixels::ZERO;
        for fragment in line_fragments {
            let origin = point(x, y + (actual_height - fragment.size.height) / 2.0);
            let positioned = match fragment.kind {
                FragmentKind::Text => PositionedFragment::Text {
                    item_index: fragment.item_index,
                    origin,
                    size: fragment.size,
                    source_range: fragment.source_range,
                },
                FragmentKind::Element => PositionedFragment::Element {
                    item_index: fragment.item_index,
                    origin,
                    size: fragment.size,
                },
            };
            x += fragment.size.width;
            fragments.push(positioned);
        }
        max_width = max_width.max(line_width);
        y += actual_height;
    }

    InlineFlowLayout {
        fragments,
        size: size(max_width, y),
    }
}

fn line_ranges(
    items: &[MeasureItem],
    element_sizes: &[Option<Size<Pixels>>],
    text_style: &TextStyle,
    wrap_width: Option<Pixels>,
    window: &mut Window,
) -> Vec<Range<usize>> {
    let total_len = items.iter().map(MeasureItem::len).sum::<usize>();
    let mut hard_lines = Vec::new();
    let mut line_start = 0;
    let mut item_start = 0;
    for item in items {
        if let MeasureItem::Text { text, .. } = item {
            for (newline, _) in text.match_indices('\n') {
                let newline = item_start + newline;
                hard_lines.push(line_start..newline);
                line_start = newline + 1;
            }
        }
        item_start += item.len();
    }
    hard_lines.push(line_start..total_len);

    let Some(wrap_width) = wrap_width else {
        return hard_lines;
    };
    let font_size = text_style.font_size.to_pixels(window.rem_size());
    let mut wrapper = window
        .text_system()
        .line_wrapper(text_style.font(), font_size);
    let mut ranges = Vec::new();

    for hard_line in hard_lines {
        let mut item_start = 0;
        let fragments = items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                let item_end = item_start + item.len();
                let fragment = if item_end <= hard_line.start || item_start >= hard_line.end {
                    None
                } else {
                    match item {
                        MeasureItem::Text { text, .. } => {
                            let start = hard_line.start.max(item_start) - item_start;
                            let end = hard_line.end.min(item_end) - item_start;
                            (start < end).then(|| LineFragment::text(&text[start..end]))
                        }
                        MeasureItem::Element => (hard_line.start <= item_start
                            && item_end <= hard_line.end)
                            .then(|| {
                                let width = element_sizes[index]
                                    .unwrap_or_default()
                                    .width
                                    .min(wrap_width);
                                LineFragment::element(width, ELEMENT_LEN)
                            }),
                    }
                };
                item_start = item_end;
                fragment
            })
            .collect::<Vec<_>>();
        let boundaries = wrapper
            .wrap_line(&fragments, wrap_width)
            .map(|boundary| hard_line.start + boundary.ix.min(hard_line.len()))
            .collect::<Vec<_>>();
        let mut start = hard_line.start;
        for end in boundaries {
            if start < end {
                ranges.push(start..end);
            }
            start = end;
        }
        if start < hard_line.end || hard_line.is_empty() {
            ranges.push(start..hard_line.end);
        }
    }

    ranges
}

fn slice_runs(runs: &[TextRun], range: Range<usize>) -> Vec<TextRun> {
    let mut result = Vec::new();
    let mut start = 0;
    for run in runs {
        let end = start + run.len;
        let clipped_start = start.max(range.start);
        let clipped_end = end.min(range.end);
        if clipped_start < clipped_end {
            let mut clipped = run.clone();
            clipped.len = clipped_end - clipped_start;
            result.push(clipped);
        }
        start = end;
    }
    result
}

fn slice_ranges(
    ranges: &[(Range<usize>, String)],
    source: Range<usize>,
) -> Vec<(Range<usize>, String)> {
    ranges
        .iter()
        .filter_map(|(range, value)| {
            let start = range.start.max(source.start);
            let end = range.end.min(source.end);
            (start < end).then(|| (start - source.start..end - source.start, value.to_owned()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slices_text_runs_at_fragment_boundaries() {
        let runs = vec![
            TextRun {
                len: 4,
                ..Default::default()
            },
            TextRun {
                len: 5,
                ..Default::default()
            },
        ];

        let sliced = slice_runs(&runs, 2..7);

        assert_eq!(sliced.iter().map(|run| run.len).collect::<Vec<_>>(), [2, 3]);
    }

    #[test]
    fn rebases_link_ranges_for_text_fragments() {
        let ranges = vec![(2..8, "https://example.com".to_owned())];

        assert_eq!(
            slice_ranges(&ranges, 5..10),
            vec![(0..3, "https://example.com".to_owned())]
        );
    }
}
