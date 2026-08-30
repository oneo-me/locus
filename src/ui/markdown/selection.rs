use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    App, Bounds, CursorStyle, DispatchPhase, Element, ElementId, FocusHandle, GlobalElementId,
    Hitbox, HitboxBehavior, Hsla, InspectorElementId, IntoElement, LayoutId, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, SharedString, StyledText, TextAlign,
    TextLayout, Window, fill, point,
};

#[derive(Clone, Default)]
pub struct MarkdownSelection(Rc<RefCell<SelectionState>>);

#[derive(Default)]
struct SelectionState {
    text: Arc<str>,
    range: Range<usize>,
    anchor: usize,
    selecting: bool,
    mode: SelectMode,
    pressed_link: Option<PressedLink>,
}

#[derive(Default)]
enum SelectMode {
    #[default]
    Character,
    Word(Range<usize>),
    Line(Range<usize>),
    All,
}

struct PressedLink {
    url: String,
    range: Range<usize>,
}

impl MarkdownSelection {
    pub fn set_document(&self, text: Arc<str>) {
        let mut state = self.0.borrow_mut();
        if state.text.as_ref() != text.as_ref() {
            *state = SelectionState {
                text,
                ..Default::default()
            };
        } else {
            state.text = text;
        }
    }

    pub fn reset(&self) -> bool {
        let state = self.0.borrow();
        let changed =
            state.range.start < state.range.end || state.selecting || state.pressed_link.is_some();
        let text = state.text.clone();
        drop(state);
        *self.0.borrow_mut() = SelectionState {
            text,
            ..Default::default()
        };
        changed
    }

    pub fn selected_range(&self) -> Range<usize> {
        self.0.borrow().range.clone()
    }

    pub fn selected_text(&self) -> Option<String> {
        let state = self.0.borrow();
        (state.range.start < state.range.end)
            .then(|| state.text.get(state.range.clone()).map(str::to_owned))
            .flatten()
    }

    pub fn select_all(&self) -> bool {
        let mut state = self.0.borrow_mut();
        let range = 0..state.text.len();
        if state.range == range {
            return false;
        }
        state.range = range;
        state.anchor = 0;
        state.selecting = false;
        state.mode = SelectMode::All;
        state.pressed_link = None;
        true
    }

    pub fn is_selecting(&self) -> bool {
        self.0.borrow().selecting
    }

    fn begin(&self, offset: usize, click_count: usize, extend: bool) -> bool {
        let mut state = self.0.borrow_mut();
        let offset = char_boundary_at_or_before(&state.text, offset);
        let previous = state.range.clone();
        state.pressed_link = None;

        if extend && previous.start < previous.end {
            state.mode = SelectMode::Character;
            state.selecting = true;
            set_character_head(&mut state, offset);
            return state.range != previous;
        }

        match click_count {
            1 => {
                state.anchor = offset;
                state.range = offset..offset;
                state.mode = SelectMode::Character;
            }
            2 => {
                let range = surrounding_word_range(&state.text, offset);
                state.anchor = range.start;
                state.range = range.clone();
                state.mode = SelectMode::Word(range);
            }
            3 => {
                let range = surrounding_line_range(&state.text, offset);
                state.anchor = range.start;
                state.range = range.clone();
                state.mode = SelectMode::Line(range);
            }
            _ => {
                state.anchor = 0;
                state.range = 0..state.text.len();
                state.mode = SelectMode::All;
            }
        }
        state.selecting = true;
        state.range != previous
    }

    fn update(&self, offset: usize) -> bool {
        let mut state = self.0.borrow_mut();
        if !state.selecting {
            return false;
        }
        let offset = char_boundary_at_or_before(&state.text, offset);
        let previous = state.range.clone();
        match &state.mode {
            SelectMode::Character => set_character_head(&mut state, offset),
            SelectMode::Word(base) => {
                let base = base.clone();
                let head = surrounding_word_range(&state.text, offset);
                state.range = base.start.min(head.start)..base.end.max(head.end);
            }
            SelectMode::Line(base) => {
                let base = base.clone();
                let head = surrounding_line_range(&state.text, offset);
                state.range = base.start.min(head.start)..base.end.max(head.end);
            }
            SelectMode::All => {}
        }
        state.range != previous
    }

    pub(super) fn finish_drag(&self) -> bool {
        let mut state = self.0.borrow_mut();
        let changed = state.selecting;
        state.selecting = false;
        changed
    }

    pub(super) fn end(&self) -> bool {
        let mut state = self.0.borrow_mut();
        let changed = state.selecting || state.pressed_link.is_some();
        state.selecting = false;
        state.pressed_link = None;
        changed
    }

    fn press_link(&self, url: String, range: Range<usize>) {
        let mut state = self.0.borrow_mut();
        state.selecting = false;
        state.pressed_link = Some(PressedLink { url, range });
    }

    fn release_link(&self, offset: usize) -> Option<String> {
        let mut state = self.0.borrow_mut();
        state.selecting = false;
        let pressed = state.pressed_link.take()?;
        pressed.range.contains(&offset).then_some(pressed.url)
    }
}

fn set_character_head(state: &mut SelectionState, offset: usize) {
    state.range = if offset < state.anchor {
        offset..state.anchor
    } else {
        state.anchor..offset
    };
}

fn char_boundary_at_or_before(text: &str, offset: usize) -> usize {
    let mut offset = offset.min(text.len());
    while offset > 0 && !text.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

fn surrounding_word_range(text: &str, offset: usize) -> Range<usize> {
    if text.is_empty() {
        return 0..0;
    }
    let mut start = char_boundary_at_or_before(text, offset);
    if start == text.len() {
        start = text
            .char_indices()
            .next_back()
            .map_or(0, |(index, _)| index);
    }
    let character = text[start..].chars().next().unwrap();
    if is_cjk(character) {
        return start..start + character.len_utf8();
    }
    let class = character_class(character);
    let mut range_start = start;
    for (index, candidate) in text[..start].char_indices().rev() {
        if is_cjk(candidate) || character_class(candidate) != class {
            break;
        }
        range_start = index;
    }
    let mut range_end = start + character.len_utf8();
    let tail_start = range_end;
    for (relative, candidate) in text[tail_start..].char_indices() {
        if is_cjk(candidate) || character_class(candidate) != class {
            break;
        }
        range_end = tail_start + relative + candidate.len_utf8();
    }
    range_start..range_end
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum CharacterClass {
    Word,
    Whitespace,
    Punctuation,
}

fn character_class(character: char) -> CharacterClass {
    if character.is_whitespace() {
        CharacterClass::Whitespace
    } else if character.is_alphanumeric() || character == '_' {
        CharacterClass::Word
    } else {
        CharacterClass::Punctuation
    }
}

fn is_cjk(character: char) -> bool {
    matches!(
        character as u32,
        0x2e80..=0x2fff
            | 0x3040..=0x30ff
            | 0x31f0..=0x31ff
            | 0x3400..=0x4dbf
            | 0x4e00..=0x9fff
            | 0xac00..=0xd7af
            | 0xf900..=0xfaff
            | 0x20000..=0x323af
    )
}

fn surrounding_line_range(text: &str, offset: usize) -> Range<usize> {
    let offset = char_boundary_at_or_before(text, offset);
    let start = text[..offset].rfind('\n').map_or(0, |index| index + 1);
    let end = text[offset..]
        .find('\n')
        .map_or(text.len(), |index| offset + index);
    start..end
}

pub struct SelectableText {
    id: ElementId,
    value: SharedString,
    text: StyledText,
    document_range: Range<usize>,
    links: Vec<(Range<usize>, String)>,
    selection_color: Hsla,
    interaction: TextInteraction,
}

#[derive(Clone)]
pub(super) struct TextInteraction {
    pub selection: MarkdownSelection,
    pub focus: FocusHandle,
    pub notify_entity: gpui::EntityId,
}

impl SelectableText {
    pub fn new(
        id: impl Into<ElementId>,
        value: SharedString,
        text: StyledText,
        document_range: Range<usize>,
        links: Vec<(Range<usize>, String)>,
        selection_color: Hsla,
        interaction: TextInteraction,
    ) -> Self {
        Self {
            id: id.into(),
            value,
            text,
            document_range,
            links,
            selection_color,
            interaction,
        }
    }
}

impl IntoElement for SelectableText {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for SelectableText {
    type RequestLayoutState = ();
    type PrepaintState = Hitbox;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn a11y_role(&self) -> Option<gpui::accesskit::Role> {
        Some(gpui::accesskit::Role::Label)
    }

    fn write_a11y_info(&self, node: &mut gpui::accesskit::Node) {
        node.set_value(self.value.to_string());
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        self.text.request_layout(None, inspector_id, window, cx)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: gpui::Bounds<gpui::Pixels>,
        state: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.text
            .prepaint(None, inspector_id, bounds, state, window, cx);
        window.insert_hitbox(bounds, HitboxBehavior::Normal)
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: gpui::Bounds<gpui::Pixels>,
        _: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let layout = self.text.layout().clone();
        let document_range = self.document_range.clone();
        let links = std::mem::take(&mut self.links);
        let selection = self.interaction.selection.clone();
        let focus = self.interaction.focus.clone();
        let notify_entity = self.interaction.notify_entity;

        paint_selection(
            &layout,
            &document_range,
            &selection.selected_range(),
            self.selection_color,
            window,
        );

        let cursor = (hitbox.is_hovered(window))
            .then(|| local_index(&layout, window.mouse_position()))
            .flatten()
            .and_then(|index| link_at(&links, index))
            .map_or(CursorStyle::IBeam, |_| CursorStyle::PointingHand);
        window.set_cursor_style(cursor, hitbox);

        window.on_mouse_event({
            let hitbox = hitbox.clone();
            let layout = layout.clone();
            let document_range = document_range.clone();
            let selection = selection.clone();
            let links = links.clone();
            move |event: &MouseDownEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble
                    || event.button != MouseButton::Left
                    || !hitbox.is_hovered(window)
                {
                    return;
                }
                let Some(local) = local_index(&layout, event.position) else {
                    return;
                };
                let local = local.min(document_range.len());
                let global = document_range.start + local;
                window.focus(&focus, cx);
                if let Some((range, url)) = link_at(&links, local) {
                    selection.press_link(
                        url.to_owned(),
                        document_range.start + range.start..document_range.start + range.end,
                    );
                } else {
                    selection.begin(global, event.click_count, event.modifiers.shift);
                }
                window.prevent_default();
                cx.notify(notify_entity);
            }
        });

        window.on_mouse_event({
            let hitbox = hitbox.clone();
            let layout = layout.clone();
            let document_range = document_range.clone();
            let selection = selection.clone();
            move |event: &MouseMoveEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble
                    || !hitbox.is_hovered(window)
                    || !selection.is_selecting()
                {
                    return;
                }
                let Some(local) = local_index(&layout, event.position) else {
                    return;
                };
                let global = document_range.start + local.min(document_range.len());
                if selection.update(global) {
                    cx.notify(notify_entity);
                }
            }
        });

        window.on_mouse_event({
            let hitbox = hitbox.clone();
            let layout = layout.clone();
            let document_range = document_range.clone();
            let selection = selection.clone();
            move |event: &MouseUpEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble
                    || event.button != MouseButton::Left
                    || !hitbox.is_hovered(window)
                {
                    return;
                }
                let url = local_index(&layout, event.position).and_then(|local| {
                    selection.release_link(document_range.start + local.min(document_range.len()))
                });
                if let Some(url) = url {
                    cx.open_url(&url);
                } else {
                    selection.end();
                }
                cx.notify(notify_entity);
            }
        });

        self.text
            .paint(None, inspector_id, bounds, &mut (), &mut (), window, cx);
    }
}

fn local_index(layout: &gpui::TextLayout, position: gpui::Point<gpui::Pixels>) -> Option<usize> {
    match layout.index_for_position(position) {
        Ok(index) | Err(index) => Some(index),
    }
}

fn link_at(links: &[(Range<usize>, String)], index: usize) -> Option<&(Range<usize>, String)> {
    links.iter().find(|(range, _)| range.contains(&index))
}

fn paint_selection(
    layout: &TextLayout,
    document_range: &Range<usize>,
    selection_range: &Range<usize>,
    color: Hsla,
    window: &mut Window,
) {
    let start = selection_range.start.max(document_range.start);
    let end = selection_range.end.min(document_range.end);
    if start >= end {
        return;
    }

    let selection = start - document_range.start..end - document_range.start;
    let bounds = layout.bounds();
    let line_height = layout.line_height();
    let text_align = window.text_style().text_align;
    let mut logical_start = 0;
    let mut row_top = bounds.top();

    for line in layout.line_layouts() {
        let logical_end = logical_start + line.len();
        if logical_start >= selection.end {
            break;
        }
        if logical_end > selection.start {
            let unwrapped = &line.unwrapped_layout;
            let row_ends = line
                .wrap_boundaries()
                .iter()
                .map(|boundary| {
                    let glyph = &unwrapped.runs[boundary.run_ix].glyphs[boundary.glyph_ix];
                    (logical_start + glyph.index, glyph.position.x)
                })
                .chain([(logical_end, unwrapped.width)]);
            let mut row_start = logical_start;
            let mut row_start_x = Pixels::ZERO;

            for (row_end, row_end_x) in row_ends {
                let selected_start = selection.start.max(row_start);
                let selected_end = selection.end.min(row_end);
                if selected_start < selected_end {
                    let row_width = row_end_x - row_start_x;
                    let alignment_offset = match text_align {
                        TextAlign::Left => Pixels::ZERO,
                        TextAlign::Center => {
                            ((bounds.size.width - row_width) / 2.0).max(Pixels::ZERO)
                        }
                        TextAlign::Right => (bounds.size.width - row_width).max(Pixels::ZERO),
                    };
                    let x_for_index = |index| {
                        bounds.left()
                            + alignment_offset
                            + unwrapped.x_for_index(index - logical_start)
                            - row_start_x
                    };
                    window.paint_quad(fill(
                        Bounds::from_corners(
                            point(x_for_index(selected_start), row_top),
                            point(x_for_index(selected_end), row_top + line_height),
                        ),
                        color,
                    ));
                }
                row_start = row_end;
                row_start_x = row_end_x;
                row_top += line_height;
            }
        } else {
            row_top += line.size(line_height).height;
        }
        logical_start = logical_end + 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_words_and_cjk_characters() {
        assert_eq!(surrounding_word_range("hello world", 7), 6..11);
        assert_eq!(surrounding_word_range("中文内容", 3), 3..6);
    }

    #[test]
    fn drags_a_selection_across_document_text() {
        let selection = MarkdownSelection::default();
        selection.set_document(Arc::from("first\nsecond"));
        selection.begin(2, 1, false);
        selection.update(10);
        selection.end();
        assert_eq!(selection.selected_text().as_deref(), Some("rst\nseco"));
    }
}
