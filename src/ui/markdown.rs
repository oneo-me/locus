mod attachment_views;
mod extension_syntax;
mod parser;
mod thumbnail;

use std::path::{Path, PathBuf};

use gpui::{
    AnyElement, FontStyle, FontWeight, InteractiveText, ObjectFit, SharedString,
    StrikethroughStyle, StyledText, TextRun, UnderlineStyle, div, font, img, prelude::*, px,
    relative,
};

use self::parser::{Block, InlineRun, ListItem, TableAlign};
use crate::document_library::Attachment;
use crate::localization::Language;
use crate::theme::Theme;

const SANS_FAMILY: &str = ".SystemUIFont";
const MONO_FAMILY: &str = "monospace";

pub struct Markdown<'a> {
    source: &'a str,
    base_directory: &'a Path,
    theme: Theme,
    text_size: f32,
    code_text_size: f32,
    attachments: &'a [Attachment],
    language: Language,
}

impl<'a> Markdown<'a> {
    pub fn new(
        source: &'a str,
        base_directory: &'a Path,
        theme: Theme,
        text_size: f32,
        code_text_size: f32,
    ) -> Self {
        Self {
            source,
            base_directory,
            theme,
            text_size,
            code_text_size,
            attachments: &[],
            language: Language::Chinese,
        }
    }

    pub fn with_attachment_views(
        mut self,
        attachments: &'a [Attachment],
        language: Language,
    ) -> Self {
        self.attachments = attachments;
        self.language = language;
        self
    }

    pub fn render(self) -> AnyElement {
        let blocks = parser::parse(self.source);
        let mut context = RenderContext {
            base_directory: self.base_directory,
            theme: self.theme,
            text_size: self.text_size,
            code_text_size: self.code_text_size,
            attachments: self.attachments,
            language: self.language,
            next_text_id: 0,
        };
        let children = blocks
            .iter()
            .map(|block| context.render_block(block))
            .collect::<Vec<_>>();

        div()
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
    text_size: f32,
    code_text_size: f32,
    attachments: &'a [Attachment],
    language: Language,
    next_text_id: usize,
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
            Block::Image { url, alt } => self.render_image(url, alt),
            Block::Code { language, code } => self.render_code_block(language, code),
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
                            .bg(self.theme.border_strong),
                    )
                    .child(
                        div()
                            .flex_auto()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(self.block_gap()))
                            .text_color(self.theme.text_secondary)
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
                .bg(self.theme.border_strong)
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
            let mut run_font = font(if run.style.code {
                MONO_FAMILY
            } else {
                SANS_FAMILY
            });
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
                    self.theme.focus
                } else {
                    self.theme.text
                },
                background_color: run.style.code.then_some(self.theme.control_selected),
                underline: run.style.link.is_some().then_some(UnderlineStyle {
                    color: Some(self.theme.focus),
                    thickness: px(1.0),
                    wavy: false,
                }),
                strikethrough: run.style.strikethrough.then_some(StrikethroughStyle {
                    thickness: px(1.0),
                    color: Some(self.theme.text_tertiary),
                }),
            });
        }

        let styled = StyledText::new(SharedString::from(text)).with_runs(text_runs);
        if links.is_empty() {
            return styled.into_any_element();
        }
        let id = SharedString::from(format!("markdown-text-{}", self.next_text_id));
        self.next_text_id += 1;
        let (ranges, urls): (Vec<_>, Vec<_>) = links.into_iter().unzip();
        InteractiveText::new(id, styled)
            .on_click(ranges, move |clicked, _, cx| {
                if let Some(url) = urls.get(clicked) {
                    cx.open_url(url);
                }
            })
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
                            .flex_auto()
                            .min_w_0()
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
            .text_color(self.theme.text_tertiary)
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
                        self.theme.focus
                    } else {
                        self.theme.border_strong
                    })
                    .when(checked, |element| element.bg(self.theme.focus))
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(checked, |element| {
                        element.child(
                            div()
                                .text_size(px((size - 3.0).max(9.0)))
                                .line_height(px(size))
                                .text_color(self.theme.surface)
                                .child("✓"),
                        )
                    }),
            )
            .into_any_element()
    }

    fn render_image(&self, url: &str, alt: &str) -> AnyElement {
        if let Some(attachment) = extension_syntax::find_image_attachment(self.attachments, url) {
            return div()
                .w_full()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(attachment_views::render_thumbnail_image(
                    attachment, self.theme,
                ))
                .when(!alt.trim().is_empty(), |element| {
                    element.child(
                        div()
                            .text_size(px((self.text_size - 2.0).max(12.0)))
                            .line_height(px((self.text_size * 1.35).round()))
                            .text_color(self.theme.text_tertiary)
                            .child(SharedString::from(alt.to_owned())),
                    )
                })
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
            .when(!alt.trim().is_empty(), |element| {
                element.child(
                    div()
                        .text_size(px((self.text_size - 2.0).max(12.0)))
                        .line_height(px((self.text_size * 1.35).round()))
                        .text_color(self.theme.text_tertiary)
                        .child(SharedString::from(alt.to_owned())),
                )
            })
            .into_any_element()
    }

    fn render_code_block(&self, language: &Option<String>, code: &str) -> AnyElement {
        match language.as_deref() {
            Some("gallery") => {
                return attachment_views::render_gallery(
                    code,
                    self.attachments,
                    self.theme,
                    self.language,
                );
            }
            Some("attachments") => {
                return attachment_views::render_attachments(
                    code,
                    self.attachments,
                    self.theme,
                    self.language,
                );
            }
            _ => {}
        }
        div()
            .w_full()
            .min_w_0()
            .rounded(px(8.0))
            .border_1()
            .border_color(self.theme.border)
            .bg(self.theme.raised)
            .overflow_hidden()
            .when_some(language.as_ref(), |element, language| {
                element.child(
                    div()
                        .w_full()
                        .px(px(10.0))
                        .py(px(6.0))
                        .border_b_1()
                        .border_color(self.theme.border)
                        .text_size(px(12.0))
                        .line_height(px(16.0))
                        .text_color(self.theme.text_tertiary)
                        .child(language.to_ascii_lowercase()),
                )
            })
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .px(px(10.0))
                    .py(px(8.0))
                    .font_family(MONO_FAMILY)
                    .text_size(px(self.code_text_size))
                    .line_height(px((self.code_text_size * 1.5).round()))
                    .text_color(self.theme.text_secondary)
                    .whitespace_normal()
                    .child(SharedString::from(code.to_owned())),
            )
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
            .border_color(self.theme.border_strong)
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
                .bg(self.theme.control_selected),
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
    ) -> gpui::Div {
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
        }];
        let long = vec![InlineRun {
            text: "a much longer value".to_owned(),
            style: Default::default(),
        }];
        let widths = column_widths(&[short, long], &[], 2);
        assert!((widths.iter().sum::<f32>() - 1.0).abs() < f32::EPSILON);
        assert!(widths.iter().all(|width| *width >= 0.275));
    }
}
