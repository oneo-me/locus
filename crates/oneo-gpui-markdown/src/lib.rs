mod code_blocks;
mod inline_flow;
mod parser;
mod selection;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use gpui_kit::{
    AnyElement, ClipboardItem, FocusHandle, FontStyle, FontWeight, KeyDownEvent, MouseButton,
    ObjectFit, Role, SharedString, StrikethroughStyle, StyledImage, StyledText, TextRun,
    Transformation, UnderlineStyle, div, font, img, linear_color_stop, linear_gradient, prelude::*,
    px, radians, relative,
};

pub use self::code_blocks::MarkdownCodeBlocks;
use self::inline_flow::{InlineFlow, InlineFlowItem, InlineTextItem};
pub use self::parser::{
    Block, InlineRun, InlineStyle, ListItem, TableAlign, parse as parse_blocks,
};
use self::selection::{LinkHandler, SelectableText};
pub use self::selection::{MarkdownSelection, SelectableCodeStyle, selectable_code_text};
use gpui_kit::component::{
    Icon, IconName, Sizable as _, Theme,
    button::{Button, ButtonVariants as _},
};

pub trait MarkdownExtensions {
    fn render_code_block(
        &self,
        _language: &str,
        _source: &str,
        _theme: &Theme,
    ) -> Option<AnyElement> {
        None
    }

    fn render_image(&self, _url: &str, _theme: &Theme) -> Option<AnyElement> {
        None
    }

    fn render_link_icon(&self, _url: &str, _theme: &Theme) -> Option<AnyElement> {
        None
    }
}

#[derive(Clone)]
pub struct MarkdownLabels {
    pub copy_code: SharedString,
    pub collapse_code: SharedString,
    pub expand_code: SharedString,
}

#[derive(Clone)]
pub struct MarkdownDocument {
    source: Arc<str>,
    blocks: Arc<[Block]>,
    selection_text: Arc<str>,
}

impl MarkdownDocument {
    pub fn parse(source: &str) -> Self {
        Self::parse_with_hidden_code_blocks(source, &[])
    }

    pub fn parse_with_hidden_code_blocks(source: &str, hidden_languages: &[&str]) -> Self {
        let mut blocks = parser::parse(source);
        let selection_text = parser::index_selection_text(&mut blocks, hidden_languages);
        Self {
            source: source.into(),
            blocks: blocks.into(),
            selection_text: selection_text.into(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    pub fn synchronize_interaction_state(
        &self,
        selection: &MarkdownSelection,
        code_blocks: &MarkdownCodeBlocks,
    ) {
        selection.set_document(self.selection_text.clone());
        code_blocks.set_document(self.source.clone());
    }
}

pub struct Markdown<'a> {
    document: &'a MarkdownDocument,
    base_directory: &'a Path,
    theme: Theme,
    text_font_family: &'a str,
    text_size: f32,
    code_font_family: &'a str,
    code_text_size: f32,
    labels: MarkdownLabels,
    extensions: Option<&'a dyn MarkdownExtensions>,
    selection: MarkdownSelection,
    code_blocks: MarkdownCodeBlocks,
    focus: FocusHandle,
    notify_entity: gpui_kit::EntityId,
    link_handler: Option<LinkHandler>,
}

pub struct MarkdownTypography<'a> {
    pub text_font_family: &'a str,
    pub text_size: f32,
    pub code_font_family: &'a str,
    pub code_text_size: f32,
}

pub struct MarkdownPresentation<'a> {
    pub theme: Theme,
    pub typography: MarkdownTypography<'a>,
    pub labels: MarkdownLabels,
}

impl<'a> Markdown<'a> {
    pub fn new(
        document: &'a MarkdownDocument,
        base_directory: &'a Path,
        presentation: MarkdownPresentation<'a>,
        selection: MarkdownSelection,
        focus: FocusHandle,
        notify_entity: gpui_kit::EntityId,
    ) -> Self {
        let MarkdownPresentation {
            theme,
            typography,
            labels,
        } = presentation;
        Self {
            document,
            base_directory,
            theme,
            text_font_family: typography.text_font_family,
            text_size: typography.text_size,
            code_font_family: typography.code_font_family,
            code_text_size: typography.code_text_size,
            labels,
            extensions: None,
            selection,
            code_blocks: MarkdownCodeBlocks::default(),
            focus,
            notify_entity,
            link_handler: None,
        }
    }

    pub fn with_extensions(mut self, extensions: &'a dyn MarkdownExtensions) -> Self {
        self.extensions = Some(extensions);
        self
    }

    pub fn with_code_blocks(mut self, code_blocks: MarkdownCodeBlocks) -> Self {
        self.code_blocks = code_blocks;
        self
    }

    pub fn with_link_handler(
        mut self,
        handler: impl Fn(&str, &mut gpui_kit::Window, &mut gpui_kit::App) -> bool + 'static,
    ) -> Self {
        self.link_handler = Some(std::rc::Rc::new(handler));
        self
    }

    pub fn render(self) -> AnyElement {
        let mut context = RenderContext {
            base_directory: self.base_directory,
            theme: self.theme,
            text_font_family: self.text_font_family,
            text_size: self.text_size,
            code_font_family: self.code_font_family,
            code_text_size: self.code_text_size,
            labels: self.labels,
            extensions: self.extensions,
            selection: self.selection.clone(),
            code_blocks: self.code_blocks,
            focus: self.focus.clone(),
            notify_entity: self.notify_entity,
            link_handler: self.link_handler,
            next_text_id: 0,
            next_code_id: 0,
            next_link_id: 0,
            next_flow_id: 0,
        };
        let children = self
            .document
            .blocks
            .iter()
            .map(|block| context.render_block(block))
            .collect::<Vec<_>>();

        let selection_for_copy = self.selection.clone();
        let selection_for_mouse_up = self.selection.clone();
        let selection_for_mouse_up_out = self.selection.clone();
        let selection_for_mouse_down_out = self.selection;
        let notify_entity = self.notify_entity;
        div()
            .id("markdown-content")
            .track_focus(&self.focus)
            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                let modifiers = &event.keystroke.modifiers;
                if !modifiers.secondary() || modifiers.number_of_modifiers() != 1 {
                    return;
                }
                if event.keystroke.key.eq_ignore_ascii_case("c") {
                    if let Some(text) = selection_for_copy.selected_text() {
                        cx.write_to_clipboard(ClipboardItem::new_string(text));
                        window.prevent_default();
                        cx.stop_propagation();
                    }
                } else if event.keystroke.key.eq_ignore_ascii_case("a")
                    && selection_for_copy.select_all()
                {
                    window.prevent_default();
                    cx.stop_propagation();
                    cx.notify(notify_entity);
                }
            })
            .on_mouse_up(MouseButton::Left, move |_, _, cx| {
                if selection_for_mouse_up.finish_drag() {
                    cx.notify(notify_entity);
                }
            })
            .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
                if selection_for_mouse_up_out.end() {
                    cx.notify(notify_entity);
                }
            })
            .on_mouse_down_out(move |event, _, cx| {
                if event.button == MouseButton::Left && selection_for_mouse_down_out.reset() {
                    cx.notify(notify_entity);
                }
            })
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px((self.text_size * 0.72).round()))
            .children(children)
            .into_any_element()
    }
}

struct RenderContext<'a> {
    base_directory: &'a Path,
    theme: Theme,
    text_font_family: &'a str,
    text_size: f32,
    code_font_family: &'a str,
    code_text_size: f32,
    labels: MarkdownLabels,
    extensions: Option<&'a dyn MarkdownExtensions>,
    selection: MarkdownSelection,
    code_blocks: MarkdownCodeBlocks,
    focus: FocusHandle,
    notify_entity: gpui_kit::EntityId,
    link_handler: Option<LinkHandler>,
    next_text_id: usize,
    next_code_id: usize,
    next_link_id: usize,
    next_flow_id: usize,
}

impl RenderContext<'_> {
    fn render_block(&mut self, block: &Block) -> AnyElement {
        match block {
            Block::Paragraph { runs } => self.text_block(
                runs,
                self.text_size,
                (self.text_size * 1.55).round(),
                FontWeight::NORMAL,
            ),
            Block::Heading { level, runs } => {
                let (scale, weight) = match level {
                    1 => (1.45, FontWeight::BOLD),
                    2 => (1.28, FontWeight::BOLD),
                    3 => (1.14, FontWeight::SEMIBOLD),
                    4 => (1.05, FontWeight::SEMIBOLD),
                    _ => (1.0, FontWeight::SEMIBOLD),
                };
                let size = (self.text_size * scale).round();
                div()
                    .w_full()
                    .min_w_0()
                    .when(*level <= 2, |element| element.pt(px(4.0)))
                    .text_size(px(size))
                    .line_height(px((size * 1.42).round()))
                    .child(self.inline_text(runs, weight))
                    .into_any_element()
            }
            Block::Image {
                url,
                alt,
                alt_selection_range,
            } => self.render_image(url, alt, alt_selection_range),
            Block::Code {
                language,
                code,
                selection_range,
            } => self.render_code_block(language, code, selection_range),
            Block::Quote { children } => {
                let children = children
                    .iter()
                    .map(|block| self.render_block(block))
                    .collect::<Vec<_>>();
                div()
                    .w_full()
                    .min_w_0()
                    .flex()
                    .gap(px(10.0))
                    .child(
                        div()
                            .w(px(2.0))
                            .flex_none()
                            .rounded_full()
                            .bg(self.theme.input),
                    )
                    .child(
                        div()
                            .w_0()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(self.block_gap()))
                            .text_color(self.theme.muted_foreground)
                            .children(children),
                    )
                    .into_any_element()
            }
            Block::List {
                ordered_start,
                items,
            } => self.render_list(*ordered_start, items),
            Block::Table {
                header,
                rows,
                align,
            } => self.render_table(header, rows, align),
            Block::Rule => div()
                .w_full()
                .h(px(1.0))
                .my(px(4.0))
                .bg(self.theme.input)
                .into_any_element(),
        }
    }

    fn text_block(
        &mut self,
        runs: &[InlineRun],
        text_size: f32,
        line_height: f32,
        weight: FontWeight,
    ) -> AnyElement {
        div()
            .w_full()
            .min_w_0()
            .text_size(px(text_size))
            .line_height(px(line_height))
            .child(self.inline_text(runs, weight))
            .into_any_element()
    }

    fn inline_text(&mut self, runs: &[InlineRun], base_weight: FontWeight) -> AnyElement {
        if !runs
            .iter()
            .any(|run| run.style.code || run.style.link.is_some())
        {
            return self.styled_text(runs, base_weight);
        }

        let mut items = Vec::new();
        let mut index = 0;
        while index < runs.len() {
            let run = &runs[index];
            if let Some(url) = run.style.link.as_deref() {
                let mut title = String::new();
                while index < runs.len() && runs[index].style.link.as_deref() == Some(url) {
                    title.push_str(&runs[index].text);
                    index += 1;
                }
                items.push(InlineFlowItem::Element(self.render_link_button(url, title)));
            } else if run.style.code {
                items.push(InlineFlowItem::Element(self.inline_code(run, base_weight)));
                index += 1;
            } else {
                let start = index;
                while index < runs.len()
                    && !runs[index].style.code
                    && runs[index].style.link.is_none()
                {
                    index += 1;
                }
                items.push(InlineFlowItem::Text(
                    self.text_item(&runs[start..index], base_weight),
                ));
            }
        }

        let id = SharedString::from(format!("markdown-inline-flow-{}", self.next_flow_id));
        self.next_flow_id += 1;
        InlineFlow::new(
            id,
            items,
            self.theme.selection,
            selection::TextInteraction {
                selection: self.selection.clone(),
                focus: self.focus.clone(),
                notify_entity: self.notify_entity,
                link_handler: self.link_handler.clone(),
            },
        )
        .into_any_element()
    }

    fn render_link_button(&mut self, url: &str, title: String) -> AnyElement {
        let id = SharedString::from(format!("markdown-link-{}", self.next_link_id));
        self.next_link_id += 1;
        let click_url = url.to_owned();
        let keyboard_url = click_url.clone();
        let click_handler = self.link_handler.clone();
        let keyboard_handler = click_handler.clone();

        div()
            .id(id)
            .role(Role::Button)
            .aria_label(title.clone())
            .tab_index(0)
            .tab_stop(true)
            .w_auto()
            .min_w_0()
            .max_w_full()
            .h(px((self.code_text_size * 1.8).round()))
            .mx(px(2.0))
            .px(px(8.0))
            .flex()
            .items_center()
            .gap(px(5.0))
            .rounded_full()
            .border_1()
            .border_color(self.theme.border)
            .bg(self.theme.secondary)
            .font_family(self.code_font_family)
            .text_size(px(self.code_text_size))
            .line_height(px((self.code_text_size * 1.4).round()))
            .text_color(self.theme.foreground)
            .cursor_pointer()
            .focus_visible(|style| style.border_color(self.theme.ring))
            .hover(|style| style.bg(self.theme.accent).border_color(self.theme.input))
            .active(|style| style.bg(self.theme.accent))
            .child(self.render_link_icon(url))
            .child(div().min_w_0().truncate().child(title))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(move |_, window, cx| {
                activate_link(&click_url, click_handler.as_ref(), window, cx);
                cx.stop_propagation();
            })
            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                if !event.keystroke.modifiers.modified()
                    && matches!(event.keystroke.key.as_str(), "enter" | "space")
                {
                    activate_link(&keyboard_url, keyboard_handler.as_ref(), window, cx);
                    cx.stop_propagation();
                }
            })
            .into_any_element()
    }

    fn render_link_icon(&self, url: &str) -> AnyElement {
        if let Some(icon) = self
            .extensions
            .and_then(|extensions| extensions.render_link_icon(url, &self.theme))
        {
            return icon;
        }
        generic_link_icon(&self.theme)
    }

    fn inline_code(&mut self, run: &InlineRun, base_weight: FontWeight) -> AnyElement {
        let mut run_font = font(self.code_font_family);
        run_font.weight = if run.style.bold && base_weight < FontWeight::SEMIBOLD {
            FontWeight::SEMIBOLD
        } else {
            base_weight
        };
        let text_run = TextRun {
            len: run.text.len(),
            font: run_font,
            color: self.theme.foreground,
            background_color: None,
            underline: run.style.link.is_some().then_some(UnderlineStyle {
                color: Some(self.theme.link),
                thickness: px(1.0),
                wavy: false,
            }),
            strikethrough: run.style.strikethrough.then_some(StrikethroughStyle {
                thickness: px(1.0),
                color: Some(self.theme.muted_foreground),
            }),
        };
        let links = run
            .style
            .link
            .as_ref()
            .map(|url| vec![(0..run.text.len(), url.clone())])
            .unwrap_or_default();
        let text = SharedString::from(run.text.clone());
        let element = self.selectable_text(
            text.clone(),
            vec![text_run],
            run.selection_range.clone(),
            links,
        );
        div()
            .w_auto()
            .min_w_0()
            .max_w_full()
            .px(px(4.0))
            .rounded(px(4.0))
            .border_1()
            .border_color(self.theme.border)
            .bg(self.theme.accent)
            .font_family(self.code_font_family)
            .text_size(px(self.code_text_size))
            .line_height(px((self.code_text_size * 1.8).round()))
            .text_color(self.theme.foreground)
            .whitespace_normal()
            .child(element)
            .into_any_element()
    }

    fn styled_text(&mut self, runs: &[InlineRun], base_weight: FontWeight) -> AnyElement {
        let item = self.text_item(runs, base_weight);
        SelectableText::new(
            item.id,
            item.text.clone(),
            StyledText::new(item.text).with_runs(item.runs),
            item.document_range,
            item.links,
            self.theme.selection,
            selection::TextInteraction {
                selection: self.selection.clone(),
                focus: self.focus.clone(),
                notify_entity: self.notify_entity,
                link_handler: self.link_handler.clone(),
            },
        )
        .into_any_element()
    }

    fn text_item(&mut self, runs: &[InlineRun], base_weight: FontWeight) -> InlineTextItem {
        let mut text = String::new();
        let mut text_runs = Vec::with_capacity(runs.len());
        let mut links: Vec<(std::ops::Range<usize>, String)> = Vec::new();

        for run in runs {
            if run.text.is_empty() {
                continue;
            }
            let start = text.len();
            text.push_str(&run.text);
            let end = text.len();
            let mut run_font = font(self.text_font_family);
            run_font.weight = if run.style.bold && base_weight < FontWeight::SEMIBOLD {
                FontWeight::SEMIBOLD
            } else {
                base_weight
            };
            run_font.style = if run.style.italic {
                FontStyle::Italic
            } else {
                FontStyle::Normal
            };

            if let Some(url) = &run.style.link {
                match links.last_mut() {
                    Some((range, previous)) if range.end == start && previous == url => {
                        range.end = end
                    }
                    _ => links.push((start..end, url.clone())),
                }
            }

            text_runs.push(TextRun {
                len: run.text.len(),
                font: run_font,
                color: if run.style.link.is_some() {
                    self.theme.link
                } else {
                    self.theme.foreground
                },
                background_color: None,
                underline: run.style.link.is_some().then_some(UnderlineStyle {
                    color: Some(self.theme.link),
                    thickness: px(1.0),
                    wavy: false,
                }),
                strikethrough: run.style.strikethrough.then_some(StrikethroughStyle {
                    thickness: px(1.0),
                    color: Some(self.theme.muted_foreground),
                }),
            });
        }

        let document_range = runs
            .first()
            .zip(runs.last())
            .map(|(first, last)| first.selection_range.start..last.selection_range.end)
            .unwrap_or(0..0);
        let id = SharedString::from(format!("markdown-text-{}", self.next_text_id));
        self.next_text_id += 1;
        InlineTextItem {
            id,
            text: SharedString::from(text),
            runs: text_runs,
            document_range,
            links,
        }
    }

    fn selectable_text(
        &mut self,
        text: SharedString,
        runs: Vec<TextRun>,
        document_range: std::ops::Range<usize>,
        links: Vec<(std::ops::Range<usize>, String)>,
    ) -> AnyElement {
        let id = SharedString::from(format!("markdown-text-{}", self.next_text_id));
        self.next_text_id += 1;
        let styled = StyledText::new(text.clone()).with_runs(runs);
        SelectableText::new(
            id,
            text,
            styled,
            document_range,
            links,
            self.theme.selection,
            selection::TextInteraction {
                selection: self.selection.clone(),
                focus: self.focus.clone(),
                notify_entity: self.notify_entity,
                link_handler: self.link_handler.clone(),
            },
        )
        .into_any_element()
    }

    fn render_list(&mut self, ordered_start: Option<u64>, items: &[ListItem]) -> AnyElement {
        let marker_width = if ordered_start.is_some() { 26.0 } else { 18.0 };
        let rows = items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let marker = match (ordered_start, item.task) {
                    (_, Some(checked)) => self.task_marker(checked, marker_width),
                    (Some(start), None) => {
                        self.marker_text(format!("{}.", start + index as u64), marker_width)
                    }
                    (None, None) => self.marker_text("•".to_owned(), marker_width),
                };
                let blocks = item
                    .blocks
                    .iter()
                    .map(|block| self.render_block(block))
                    .collect::<Vec<_>>();
                div()
                    .w_full()
                    .min_w_0()
                    .flex()
                    .items_start()
                    .child(marker)
                    .child(
                        div()
                            .w_0()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(self.block_gap() * 0.6))
                            .children(blocks),
                    )
                    .into_any_element()
            })
            .collect::<Vec<_>>();

        div()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(self.block_gap() * 0.5))
            .children(rows)
            .into_any_element()
    }

    fn marker_text(&self, text: String, width: f32) -> AnyElement {
        div()
            .w(px(width))
            .flex_none()
            .text_size(px(self.text_size))
            .line_height(px((self.text_size * 1.55).round()))
            .text_color(self.theme.muted_foreground)
            .child(SharedString::from(text))
            .into_any_element()
    }

    fn task_marker(&self, checked: bool, width: f32) -> AnyElement {
        let size = (self.text_size * 0.9).round();
        div()
            .w(px(width))
            .flex_none()
            .child(
                div()
                    .size(px(size))
                    .my(px(((self.text_size * 1.55 - size) / 2.0).max(0.0)))
                    .rounded(px(3.0))
                    .border_1()
                    .border_color(if checked {
                        self.theme.primary
                    } else {
                        self.theme.input
                    })
                    .when(checked, |element| element.bg(self.theme.primary))
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(checked, |element| {
                        element.child(
                            div()
                                .text_size(px((size - 3.0).max(9.0)))
                                .line_height(px(size))
                                .text_color(self.theme.primary_foreground)
                                .child("✓"),
                        )
                    }),
            )
            .into_any_element()
    }

    fn render_image(
        &mut self,
        url: &str,
        alt: &str,
        alt_selection_range: &std::ops::Range<usize>,
    ) -> AnyElement {
        let caption =
            (!alt.trim().is_empty()).then(|| self.caption_text(alt, alt_selection_range.clone()));
        if let Some(image) = self
            .extensions
            .and_then(|extensions| extensions.render_image(url, &self.theme))
        {
            return div()
                .w_full()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(image)
                .when_some(caption, |element, caption| element.child(caption))
                .into_any_element();
        }
        let image = if url.contains("://") && !url.starts_with("file://") {
            img(url.to_owned())
        } else {
            img(image_path(self.base_directory, url))
        };
        div()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(
                image
                    .max_w(relative(1.0))
                    .max_h(px(420.0))
                    .rounded(px(10.0))
                    .object_fit(ObjectFit::ScaleDown),
            )
            .when_some(caption, |element, caption| element.child(caption))
            .into_any_element()
    }

    fn caption_text(&mut self, alt: &str, selection_range: std::ops::Range<usize>) -> AnyElement {
        let mut run_font = font(self.text_font_family);
        run_font.weight = FontWeight::NORMAL;
        let run = TextRun {
            len: alt.len(),
            font: run_font,
            color: self.theme.muted_foreground,
            ..Default::default()
        };
        div()
            .w_full()
            .min_w_0()
            .text_size(px((self.text_size - 2.0).max(12.0)))
            .line_height(px((self.text_size * 1.35).round()))
            .text_color(self.theme.muted_foreground)
            .child(self.selectable_text(
                SharedString::from(alt.to_owned()),
                vec![run],
                selection_range,
                Vec::new(),
            ))
            .into_any_element()
    }

    fn render_code_block(
        &mut self,
        language: &Option<String>,
        code: &str,
        selection_range: &std::ops::Range<usize>,
    ) -> AnyElement {
        if let Some(element) = language.as_deref().and_then(|language| {
            self.extensions
                .and_then(|extensions| extensions.render_code_block(language, code, &self.theme))
        }) {
            return element;
        }
        let code_id = self.next_code_id;
        self.next_code_id += 1;
        let expanded = self.code_blocks.is_expanded(code_id);
        let overflowing = self.code_blocks.is_overflowing(code_id);
        let body_height = if language.is_some() { 168.0 } else { 200.0 };
        let maximum_content_height = body_height - 16.0;
        let copy_label = self.labels.copy_code.clone();
        let copied_code = code.to_owned();
        let mut code_font = font(self.code_font_family);
        code_font.weight = FontWeight::NORMAL;
        let code_text = self.selectable_text(
            SharedString::from(code.to_owned()),
            vec![TextRun {
                len: code.len(),
                font: code_font,
                color: self.theme.foreground,
                ..Default::default()
            }],
            selection_range.clone(),
            Vec::new(),
        );
        let copy_button = Button::new(SharedString::from(format!("copy-code-{code_id}")))
            .ghost()
            .xsmall()
            .icon(IconName::Copy)
            .accessibility_label(copy_label)
            .on_click(move |_, _, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(copied_code.clone()));
            });

        let measured_code_blocks = self.code_blocks.clone();
        let notify_entity = self.notify_entity;
        let mut code_body = div()
            .relative()
            .w_full()
            .min_w_0()
            .overflow_hidden()
            .px(px(10.0))
            .py(px(8.0))
            .font_family(self.code_font_family)
            .text_size(px(self.code_text_size))
            .line_height(px((self.code_text_size * 1.5).round()))
            .text_color(self.theme.foreground)
            .whitespace_normal()
            .on_children_prepainted(move |bounds, _, cx| {
                let overflowing = bounds
                    .first()
                    .is_some_and(|bounds| bounds.size.height > px(maximum_content_height + 0.5));
                if measured_code_blocks.set_overflowing(code_id, overflowing) {
                    cx.notify(notify_entity);
                }
            })
            .child(code_text);
        if !expanded {
            code_body = code_body.max_h(px(body_height));
        }
        if overflowing && !expanded {
            code_body = code_body.child(self.render_code_toggle(code_id, false));
        }

        div()
            .w_full()
            .min_w_0()
            .rounded(px(8.0))
            .border_1()
            .border_color(self.theme.border)
            .bg(self.theme.secondary)
            .overflow_hidden()
            .when_some(language.as_ref(), |element, language| {
                element.child(
                    div()
                        .w_full()
                        .h(px(32.0))
                        .pl(px(10.0))
                        .pr(px(4.0))
                        .border_b_1()
                        .border_color(self.theme.border)
                        .flex()
                        .items_center()
                        .text_size(px(12.0))
                        .line_height(px(16.0))
                        .text_color(self.theme.muted_foreground)
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .truncate()
                                .child(language.to_ascii_lowercase()),
                        )
                        .child(copy_button),
                )
            })
            .child(code_body)
            .when(expanded, |element| {
                element.child(self.render_code_toggle(code_id, true))
            })
            .into_any_element()
    }

    fn render_code_toggle(&self, code_id: usize, expanded: bool) -> AnyElement {
        let label = if expanded {
            self.labels.collapse_code.clone()
        } else {
            self.labels.expand_code.clone()
        };
        let click_code_blocks = self.code_blocks.clone();
        let keyboard_code_blocks = self.code_blocks.clone();
        let notify_entity = self.notify_entity;
        let mut control = div()
            .id(SharedString::from(format!(
                "{}-code-{code_id}",
                if expanded { "collapse" } else { "expand" }
            )))
            .role(Role::Button)
            .aria_label(label)
            .tab_index(0)
            .tab_stop(true)
            .w_full()
            .h(px(44.0))
            .flex()
            .justify_center()
            .cursor_pointer()
            .focus_visible(|style| style.border_1().border_color(self.theme.ring));
        control = if expanded {
            control.items_center()
        } else {
            control
                .absolute()
                .bottom_0()
                .left_0()
                .right_0()
                .items_end()
                .pb(px(7.0))
                .bg(linear_gradient(
                    180.0,
                    linear_color_stop(self.theme.secondary.opacity(0.0), 0.0),
                    linear_color_stop(self.theme.secondary, 0.72),
                ))
        };
        control
            .child(
                Icon::new(IconName::ChevronDown)
                    .size(px(16.0))
                    .text_color(self.theme.muted_foreground)
                    .transform(Transformation::rotate(radians(if expanded {
                        std::f32::consts::PI
                    } else {
                        0.0
                    }))),
            )
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(move |_, _, cx| {
                if click_code_blocks.set_expanded(code_id, !expanded) {
                    cx.notify(notify_entity);
                }
                cx.stop_propagation();
            })
            .on_key_down(move |event: &KeyDownEvent, _, cx| {
                if !event.keystroke.modifiers.modified()
                    && matches!(event.keystroke.key.as_str(), "enter" | "space")
                {
                    if keyboard_code_blocks.set_expanded(code_id, !expanded) {
                        cx.notify(notify_entity);
                    }
                    cx.stop_propagation();
                }
            })
            .into_any_element()
    }

    fn render_table(
        &mut self,
        header: &[Vec<InlineRun>],
        rows: &[Vec<Vec<InlineRun>>],
        align: &[TableAlign],
    ) -> AnyElement {
        let columns = header
            .len()
            .max(rows.iter().map(Vec::len).max().unwrap_or(0));
        if columns == 0 {
            return div().into_any_element();
        }
        let widths = column_widths(header, rows, columns);
        let mut table = div()
            .w_full()
            .min_w_0()
            .rounded(px(8.0))
            .border_1()
            .border_color(self.theme.input)
            .overflow_hidden()
            .flex()
            .flex_col();
        if !header.is_empty() {
            table = table.child(
                self.table_row(
                    header,
                    &widths,
                    align,
                    FontWeight::SEMIBOLD,
                    !rows.is_empty(),
                )
                .when(rows.is_empty(), |row| row.rounded(px(7.0)))
                .when(!rows.is_empty(), |row| row.rounded_t(px(7.0)))
                .bg(self.theme.accent),
            );
        }
        for (index, row) in rows.iter().enumerate() {
            table = table.child(self.table_row(
                row,
                &widths,
                align,
                FontWeight::NORMAL,
                index + 1 < rows.len(),
            ));
        }
        table.into_any_element()
    }

    fn table_row(
        &mut self,
        cells: &[Vec<InlineRun>],
        widths: &[f32],
        align: &[TableAlign],
        weight: FontWeight,
        divider: bool,
    ) -> gpui_kit::Div {
        let mut row = div()
            .w_full()
            .min_w_0()
            .flex()
            .items_start()
            .when(divider, |element| {
                element.border_b_1().border_color(self.theme.border)
            });
        for (index, width) in widths.iter().enumerate() {
            let cell = cells.get(index).map(Vec::as_slice).unwrap_or_default();
            let alignment = align.get(index).copied().unwrap_or_default();
            let content = self.inline_text(cell, weight);
            row = row.child(
                div()
                    .w(relative(*width))
                    .min_w_0()
                    .px(px(9.0))
                    .py(px(6.0))
                    .text_size(px((self.text_size - 0.5).max(12.5)))
                    .line_height(px((self.text_size * 1.4).round()))
                    .map(|element| match alignment {
                        TableAlign::Left => element,
                        TableAlign::Center => element.items_center().text_center(),
                        TableAlign::Right => element.items_end().text_right(),
                    })
                    .child(content),
            );
        }
        row
    }

    fn block_gap(&self) -> f32 {
        (self.text_size * 0.72).round()
    }
}

fn image_path(base_directory: &Path, url: &str) -> PathBuf {
    if let Some(path) = url.strip_prefix("file://") {
        return PathBuf::from(path);
    }
    let path = Path::new(url);
    if path.is_absolute() {
        path.to_owned()
    } else {
        base_directory.join(path)
    }
}

fn activate_link(
    url: &str,
    handler: Option<&LinkHandler>,
    window: &mut gpui_kit::Window,
    cx: &mut gpui_kit::App,
) {
    if !handler.is_some_and(|handler| handler(url, window, cx)) {
        cx.open_url(url);
    }
}

pub fn generic_link_icon(theme: &Theme) -> AnyElement {
    Icon::new(IconName::ExternalLink)
        .size(px(12.0))
        .text_color(theme.muted_foreground)
        .into_any_element()
}

fn column_widths(
    header: &[Vec<InlineRun>],
    rows: &[Vec<Vec<InlineRun>>],
    columns: usize,
) -> Vec<f32> {
    let mut content = vec![0.0f32; columns];
    let mut measure = |index: usize, cell: &[InlineRun]| {
        let length = cell
            .iter()
            .map(|run| run.text.chars().count())
            .sum::<usize>() as f32;
        content[index] = content[index].max(length);
    };
    for (index, cell) in header.iter().enumerate() {
        measure(index, cell);
    }
    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            measure(index, cell);
        }
    }

    let even = 1.0 / columns as f32;
    let floor = even * 0.55;
    if content.iter().sum::<f32>() == 0.0 {
        return vec![even; columns];
    }
    let mut widths = vec![even; columns];
    let mut pinned = vec![false; columns];
    loop {
        let free = pinned.iter().filter(|pinned| !**pinned).count();
        if free == 0 {
            break;
        }
        let budget = 1.0 - floor * (columns - free) as f32;
        let free_content = content
            .iter()
            .enumerate()
            .filter(|(index, _)| !pinned[*index])
            .map(|(_, length)| *length)
            .sum::<f32>();
        let mut pinned_any = false;
        for index in 0..columns {
            if pinned[index] {
                continue;
            }
            widths[index] = if free_content > 0.0 {
                budget * content[index] / free_content
            } else {
                budget / free as f32
            };
            if widths[index] < floor {
                widths[index] = floor;
                pinned[index] = true;
                pinned_any = true;
            }
        }
        if !pinned_any {
            break;
        }
    }
    widths
}

#[cfg(test)]
mod tests {
    use super::*;

    struct CodeBlockView {
        document: MarkdownDocument,
        selection: MarkdownSelection,
        code_blocks: MarkdownCodeBlocks,
        focus: FocusHandle,
    }

    impl gpui_kit::Render for CodeBlockView {
        fn render(
            &mut self,
            _: &mut gpui_kit::Window,
            cx: &mut gpui_kit::Context<Self>,
        ) -> impl IntoElement {
            Markdown::new(
                &self.document,
                Path::new("/notes"),
                MarkdownPresentation {
                    theme: Theme::global(cx).clone(),
                    typography: MarkdownTypography {
                        text_font_family: ".SystemUIFont",
                        text_size: 14.0,
                        code_font_family: "Menlo",
                        code_text_size: 14.0,
                    },
                    labels: MarkdownLabels {
                        copy_code: "Copy code".into(),
                        collapse_code: "Collapse code".into(),
                        expand_code: "Expand code".into(),
                    },
                },
                self.selection.clone(),
                self.focus.clone(),
                cx.entity_id(),
            )
            .with_code_blocks(self.code_blocks.clone())
            .render()
        }
    }

    #[gpui_kit::test]
    fn plain_code_blocks_preserve_text_selection_and_overflow(cx: &mut gpui_kit::TestAppContext) {
        cx.update(gpui_kit::component::init);
        let code = "    let 中文 = \"**literal** ~~~~ <tag>\";\n".repeat(30);
        let document = MarkdownDocument::parse(&format!("Before\n\n```rust\n{code}```\n\nAfter"));
        let selection = MarkdownSelection::default();
        let code_blocks = MarkdownCodeBlocks::default();
        document.synchronize_interaction_state(&selection, &code_blocks);
        let (_, cx) = cx.add_window_view(|_, cx| CodeBlockView {
            document,
            selection: selection.clone(),
            code_blocks: code_blocks.clone(),
            focus: cx.focus_handle(),
        });

        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(code_blocks.is_overflowing(0));
        assert!(!code_blocks.is_expanded(0));
        assert!(selection.select_all());
        assert_eq!(
            selection.selected_text().as_deref(),
            Some(format!("Before\n{code}After").as_str())
        );

        code_blocks.set_expanded(0, true);
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(code_blocks.is_expanded(0));
        assert!(code_blocks.is_overflowing(0));
    }

    #[test]
    fn resolves_relative_images_from_the_note_directory() {
        assert_eq!(
            image_path(Path::new("/notes/article"), "diagram.png"),
            PathBuf::from("/notes/article/diagram.png")
        );
    }

    #[test]
    fn table_widths_preserve_a_readable_floor() {
        let short = vec![InlineRun {
            text: "x".to_owned(),
            style: Default::default(),
            selection_range: 0..1,
        }];
        let long = vec![InlineRun {
            text: "a much longer value".to_owned(),
            style: Default::default(),
            selection_range: 0..19,
        }];
        let widths = column_widths(&[short, long], &[], 2);
        assert!((widths.iter().sum::<f32>() - 1.0).abs() < f32::EPSILON);
        assert!(widths.iter().all(|width| *width >= 0.275));
    }

    #[test]
    fn extension_code_blocks_can_be_hidden_from_selection_text() {
        let document = MarkdownDocument::parse_with_hidden_code_blocks(
            "Before\n\n```gallery\n![Sea](sea.jpg)\n```\n\nAfter",
            &["gallery"],
        );

        assert_eq!(document.selection_text.as_ref(), "Before\nAfter");
    }
}
