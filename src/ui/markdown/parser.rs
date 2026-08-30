use std::ops::Range;
use std::sync::LazyLock;

use pulldown_cmark::{Alignment, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag};
use regex::Regex;

static BARE_WEB_URL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)\bhttps?://[^\s<>"`\\]+"#).expect("bare web URL regex should compile")
});

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InlineStyle {
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
    pub strikethrough: bool,
    pub link: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InlineRun {
    pub text: String,
    pub style: InlineStyle,
    pub selection_range: Range<usize>,
}

impl InlineRun {
    fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: InlineStyle::default(),
            selection_range: 0..0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TableAlign {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ListItem {
    pub task: Option<bool>,
    pub blocks: Vec<Block>,
}

#[derive(Clone, Debug, PartialEq)]
enum InlinePiece {
    Run(InlineRun),
    Image { url: String, alt: String },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    Paragraph {
        runs: Vec<InlineRun>,
    },
    Image {
        url: String,
        alt: String,
        alt_selection_range: Range<usize>,
    },
    Heading {
        level: u8,
        runs: Vec<InlineRun>,
    },
    Code {
        language: Option<String>,
        code: String,
        selection_range: Range<usize>,
    },
    Quote {
        children: Vec<Block>,
    },
    List {
        ordered_start: Option<u64>,
        items: Vec<ListItem>,
    },
    Table {
        header: Vec<Vec<InlineRun>>,
        rows: Vec<Vec<Vec<InlineRun>>>,
        align: Vec<TableAlign>,
    },
    Rule,
}

pub fn parse(source: &str) -> Vec<Block> {
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let events = Parser::new_ext(source, options).collect::<Vec<_>>();
    let mut cursor = Cursor {
        events: &events,
        index: 0,
    };
    let mut blocks = Vec::new();
    while let Some(event) = cursor.peek() {
        match event {
            Event::Rule => {
                cursor.bump();
                blocks.push(Block::Rule);
            }
            Event::Start(_) => blocks.extend(parse_started_block(&mut cursor)),
            _ => cursor.bump(),
        }
    }
    blocks
}

struct Cursor<'a, 'event> {
    events: &'a [Event<'event>],
    index: usize,
}

impl<'event> Cursor<'_, 'event> {
    fn peek(&self) -> Option<&Event<'event>> {
        self.events.get(self.index)
    }

    fn bump(&mut self) {
        self.index += 1;
    }

    fn next(&mut self) -> Option<Event<'event>> {
        let event = self.events.get(self.index).cloned();
        if event.is_some() {
            self.index += 1;
        }
        event
    }
}

fn parse_started_block(cursor: &mut Cursor) -> Vec<Block> {
    let Some(Event::Start(tag)) = cursor.next() else {
        return Vec::new();
    };
    match tag {
        Tag::Paragraph => pieces_into_blocks(parse_inline_container(cursor)),
        Tag::Heading { level, .. } => vec![Block::Heading {
            level: heading_level(level),
            runs: pieces_into_runs(parse_inline_container(cursor)),
        }],
        Tag::CodeBlock(kind) => {
            let language = match kind {
                CodeBlockKind::Fenced(info) => info
                    .split_whitespace()
                    .next()
                    .filter(|language| !language.is_empty())
                    .map(str::to_owned),
                CodeBlockKind::Indented => None,
            };
            let mut code = String::new();
            loop {
                match cursor.next() {
                    Some(Event::Text(text)) => code.push_str(&text),
                    Some(Event::End(_)) | None => break,
                    Some(_) => {}
                }
            }
            if code.ends_with('\n') {
                code.pop();
            }
            vec![Block::Code {
                language,
                code,
                selection_range: 0..0,
            }]
        }
        Tag::BlockQuote(_) => vec![Block::Quote {
            children: parse_block_sequence(cursor),
        }],
        Tag::List(ordered_start) => {
            let mut items = Vec::new();
            loop {
                match cursor.peek() {
                    Some(Event::Start(Tag::Item)) => {
                        cursor.bump();
                        items.push(parse_list_item(cursor));
                    }
                    Some(Event::End(_)) | None => {
                        cursor.bump();
                        break;
                    }
                    Some(_) => cursor.bump(),
                }
            }
            vec![Block::List {
                ordered_start,
                items,
            }]
        }
        Tag::Table(alignments) => {
            let align = alignments
                .iter()
                .map(|alignment| match alignment {
                    Alignment::Center => TableAlign::Center,
                    Alignment::Right => TableAlign::Right,
                    Alignment::None | Alignment::Left => TableAlign::Left,
                })
                .collect();
            vec![parse_table(cursor, align)]
        }
        Tag::HtmlBlock => {
            let mut text = String::new();
            loop {
                match cursor.next() {
                    Some(Event::Html(chunk) | Event::Text(chunk)) => text.push_str(&chunk),
                    Some(Event::End(_)) | None => break,
                    Some(_) => {}
                }
            }
            let text = text.trim_end_matches('\n').to_owned();
            (!text.is_empty())
                .then(|| Block::Paragraph {
                    runs: vec![InlineRun::plain(text)],
                })
                .into_iter()
                .collect()
        }
        _ => parse_block_sequence(cursor),
    }
}

fn parse_list_item(cursor: &mut Cursor) -> ListItem {
    let task = match cursor.peek() {
        Some(Event::TaskListMarker(checked)) => {
            let checked = *checked;
            cursor.bump();
            Some(checked)
        }
        _ => None,
    };
    ListItem {
        task,
        blocks: parse_block_sequence(cursor),
    }
}

fn parse_block_sequence(cursor: &mut Cursor) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut inline = Vec::new();
    while let Some(event) = cursor.peek() {
        match event {
            Event::End(_) => {
                cursor.bump();
                break;
            }
            Event::Start(tag) if is_block_tag(tag) => {
                flush_paragraph(&mut blocks, &mut inline);
                blocks.extend(parse_started_block(cursor));
            }
            Event::Rule => {
                flush_paragraph(&mut blocks, &mut inline);
                cursor.bump();
                blocks.push(Block::Rule);
            }
            _ => parse_inline_event(cursor, &mut inline, &InlineStyle::default()),
        }
    }
    flush_paragraph(&mut blocks, &mut inline);
    blocks
}

fn is_block_tag(tag: &Tag) -> bool {
    matches!(
        tag,
        Tag::Paragraph
            | Tag::Heading { .. }
            | Tag::CodeBlock(_)
            | Tag::BlockQuote(_)
            | Tag::List(_)
            | Tag::Item
            | Tag::Table(_)
            | Tag::HtmlBlock
            | Tag::FootnoteDefinition(_)
    )
}

fn flush_paragraph(blocks: &mut Vec<Block>, inline: &mut Vec<InlinePiece>) {
    if !inline.is_empty() {
        blocks.extend(pieces_into_blocks(merge_pieces(std::mem::take(inline))));
    }
}

fn parse_table(cursor: &mut Cursor, align: Vec<TableAlign>) -> Block {
    let mut header = Vec::new();
    let mut rows = Vec::new();
    loop {
        match cursor.peek() {
            Some(Event::Start(Tag::TableHead)) => {
                cursor.bump();
                header = parse_table_row(cursor);
            }
            Some(Event::Start(Tag::TableRow)) => {
                cursor.bump();
                rows.push(parse_table_row(cursor));
            }
            Some(Event::End(_)) | None => {
                cursor.bump();
                break;
            }
            Some(_) => cursor.bump(),
        }
    }
    Block::Table {
        header,
        rows,
        align,
    }
}

fn parse_table_row(cursor: &mut Cursor) -> Vec<Vec<InlineRun>> {
    let mut cells = Vec::new();
    loop {
        match cursor.peek() {
            Some(Event::Start(Tag::TableCell)) => {
                cursor.bump();
                cells.push(pieces_into_runs(parse_inline_container(cursor)));
            }
            Some(Event::End(_)) | None => {
                cursor.bump();
                break;
            }
            Some(_) => cursor.bump(),
        }
    }
    cells
}

fn parse_inline_container(cursor: &mut Cursor) -> Vec<InlinePiece> {
    let mut pieces = Vec::new();
    while let Some(event) = cursor.peek() {
        if matches!(event, Event::End(_)) {
            cursor.bump();
            break;
        }
        parse_inline_event(cursor, &mut pieces, &InlineStyle::default());
    }
    merge_pieces(pieces)
}

fn parse_inline_event(cursor: &mut Cursor, pieces: &mut Vec<InlinePiece>, style: &InlineStyle) {
    let Some(event) = cursor.next() else {
        return;
    };
    match event {
        Event::Text(text) => pieces.push(InlinePiece::Run(InlineRun {
            text: text.to_string(),
            style: style.clone(),
            selection_range: 0..0,
        })),
        Event::Code(text) => {
            let mut code_style = style.clone();
            code_style.code = true;
            pieces.push(InlinePiece::Run(InlineRun {
                text: text.to_string(),
                style: code_style,
                selection_range: 0..0,
            }));
        }
        Event::SoftBreak | Event::HardBreak => pieces.push(InlinePiece::Run(InlineRun {
            text: "\n".to_owned(),
            style: style.clone(),
            selection_range: 0..0,
        })),
        Event::Start(Tag::Image {
            dest_url, title, ..
        }) => {
            let alt = pieces_into_runs(parse_inline_container(cursor))
                .into_iter()
                .map(|run| run.text)
                .collect::<String>();
            pieces.push(InlinePiece::Image {
                url: dest_url.to_string(),
                alt: if alt.trim().is_empty() {
                    title.to_string()
                } else {
                    alt
                },
            });
        }
        Event::Start(tag) => {
            let mut nested = style.clone();
            match &tag {
                Tag::Emphasis => nested.italic = true,
                Tag::Strong => nested.bold = true,
                Tag::Strikethrough => nested.strikethrough = true,
                Tag::Link { dest_url, .. } => nested.link = Some(dest_url.to_string()),
                _ => {}
            }
            while let Some(event) = cursor.peek() {
                if matches!(event, Event::End(_)) {
                    cursor.bump();
                    break;
                }
                parse_inline_event(cursor, pieces, &nested);
            }
        }
        Event::Html(text) | Event::InlineHtml(text) => pieces.push(InlinePiece::Run(InlineRun {
            text: text.to_string(),
            style: style.clone(),
            selection_range: 0..0,
        })),
        Event::FootnoteReference(label) => pieces.push(InlinePiece::Run(InlineRun {
            text: format!("[{label}]"),
            style: style.clone(),
            selection_range: 0..0,
        })),
        Event::TaskListMarker(checked) => pieces.push(InlinePiece::Run(InlineRun {
            text: if checked { "[x] " } else { "[ ] " }.to_owned(),
            style: style.clone(),
            selection_range: 0..0,
        })),
        Event::End(_) | Event::Rule | Event::InlineMath(_) | Event::DisplayMath(_) => {}
    }
}

fn pieces_into_blocks(pieces: Vec<InlinePiece>) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut runs = Vec::new();
    for piece in pieces {
        match piece {
            InlinePiece::Run(run) => runs.push(run),
            InlinePiece::Image { url, alt } => {
                if !runs.is_empty() {
                    blocks.push(Block::Paragraph {
                        runs: std::mem::take(&mut runs),
                    });
                }
                blocks.push(Block::Image {
                    url,
                    alt,
                    alt_selection_range: 0..0,
                });
            }
        }
    }
    if !runs.is_empty() {
        blocks.push(Block::Paragraph { runs });
    }
    blocks
}

fn pieces_into_runs(pieces: Vec<InlinePiece>) -> Vec<InlineRun> {
    merge_runs(
        pieces
            .into_iter()
            .map(|piece| match piece {
                InlinePiece::Run(run) => run,
                InlinePiece::Image { alt, .. } => InlineRun::plain(alt),
            })
            .collect(),
    )
}

fn merge_pieces(pieces: Vec<InlinePiece>) -> Vec<InlinePiece> {
    let mut merged = Vec::with_capacity(pieces.len());
    for piece in pieces {
        match piece {
            InlinePiece::Run(run) if run.text.is_empty() => {}
            InlinePiece::Run(run) => match merged.last_mut() {
                Some(InlinePiece::Run(last)) if last.style == run.style => {
                    last.text.push_str(&run.text)
                }
                _ => merged.push(InlinePiece::Run(run)),
            },
            image => merged.push(image),
        }
    }
    linkify_bare_urls(merged)
}

fn linkify_bare_urls(pieces: Vec<InlinePiece>) -> Vec<InlinePiece> {
    let mut linked = Vec::with_capacity(pieces.len());
    for piece in pieces {
        match piece {
            InlinePiece::Run(run) if !run.style.code && run.style.link.is_none() => {
                push_linkified_run(run, &mut linked)
            }
            piece => linked.push(piece),
        }
    }
    linked
}

fn push_linkified_run(run: InlineRun, pieces: &mut Vec<InlinePiece>) {
    let mut cursor = 0;
    for candidate in BARE_WEB_URL.find_iter(&run.text) {
        let end = trimmed_bare_url_end(&run.text, candidate.start(), candidate.end());
        if cursor < candidate.start() {
            pieces.push(InlinePiece::Run(InlineRun {
                text: run.text[cursor..candidate.start()].to_owned(),
                style: run.style.clone(),
                selection_range: 0..0,
            }));
        }
        let url = &run.text[candidate.start()..end];
        let mut link_style = run.style.clone();
        link_style.link = Some(url.to_owned());
        pieces.push(InlinePiece::Run(InlineRun {
            text: url.to_owned(),
            style: link_style,
            selection_range: 0..0,
        }));
        cursor = end;
    }
    if cursor < run.text.len() {
        pieces.push(InlinePiece::Run(InlineRun {
            text: run.text[cursor..].to_owned(),
            style: run.style,
            selection_range: 0..0,
        }));
    }
}

fn trimmed_bare_url_end(text: &str, start: usize, candidate_end: usize) -> usize {
    let mut end = candidate_end;
    let mut parens = delimiter_balance(&text[start..end], '(', ')');
    let mut brackets = delimiter_balance(&text[start..end], '[', ']');
    let mut braces = delimiter_balance(&text[start..end], '{', '}');
    loop {
        let Some((offset, last)) = text[start..end].char_indices().next_back() else {
            return start;
        };
        let should_trim = match last {
            '.' | ',' | ':' | ';' | '?' | '!' | '\'' => true,
            ')' if parens < 0 => {
                parens += 1;
                true
            }
            ']' if brackets < 0 => {
                brackets += 1;
                true
            }
            '}' if braces < 0 => {
                braces += 1;
                true
            }
            _ => false,
        };
        if !should_trim {
            return end;
        }
        end = start + offset;
    }
}

fn delimiter_balance(text: &str, open: char, close: char) -> i32 {
    text.chars().fold(0, |balance, character| {
        balance + i32::from(character == open) - i32::from(character == close)
    })
}

fn merge_runs(runs: Vec<InlineRun>) -> Vec<InlineRun> {
    let mut merged: Vec<InlineRun> = Vec::with_capacity(runs.len());
    for run in runs {
        if run.text.is_empty() {
            continue;
        }
        match merged.last_mut() {
            Some(last) if last.style == run.style => last.text.push_str(&run.text),
            _ => merged.push(run),
        }
    }
    merged
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

pub fn index_selection_text(blocks: &mut [Block]) -> String {
    let mut indexer = SelectionTextIndexer::default();
    indexer.index_blocks(blocks);
    indexer.text
}

#[derive(Default)]
struct SelectionTextIndexer {
    text: String,
}

impl SelectionTextIndexer {
    fn index_blocks(&mut self, blocks: &mut [Block]) {
        let mut has_previous = false;
        for block in blocks {
            if !block_has_selection_text(block) {
                continue;
            }
            if has_previous {
                self.line_break();
            }
            self.index_block(block);
            has_previous = true;
        }
    }

    fn index_block(&mut self, block: &mut Block) {
        match block {
            Block::Paragraph { runs } | Block::Heading { runs, .. } => self.index_runs(runs),
            Block::Image {
                alt,
                alt_selection_range,
                ..
            } => *alt_selection_range = self.push_text(alt),
            Block::Code {
                language,
                code,
                selection_range,
            } => {
                if matches!(language.as_deref(), Some("gallery" | "attachments")) {
                    *selection_range = self.text.len()..self.text.len();
                } else {
                    *selection_range = self.push_text(code);
                }
            }
            Block::Quote { children } => self.index_blocks(children),
            Block::List {
                ordered_start,
                items,
            } => self.index_list(*ordered_start, items),
            Block::Table { header, rows, .. } => self.index_table(header, rows),
            Block::Rule => {}
        }
    }

    fn index_runs(&mut self, runs: &mut [InlineRun]) {
        for run in runs {
            run.selection_range = self.push_text(&run.text);
        }
    }

    fn index_list(&mut self, ordered_start: Option<u64>, items: &mut [ListItem]) {
        let mut visible_index = 0_u64;
        let mut has_previous = false;
        for item in items {
            if !item.blocks.iter().any(block_has_selection_text) {
                continue;
            }
            if has_previous {
                self.line_break();
            }
            let marker = match (ordered_start, item.task) {
                (_, Some(true)) => "[x] ".to_owned(),
                (_, Some(false)) => "[ ] ".to_owned(),
                (Some(start), None) => format!("{}. ", start + visible_index),
                (None, None) => "• ".to_owned(),
            };
            self.text.push_str(&marker);
            self.index_blocks(&mut item.blocks);
            visible_index += 1;
            has_previous = true;
        }
    }

    fn index_table(&mut self, header: &mut [Vec<InlineRun>], rows: &mut [Vec<Vec<InlineRun>>]) {
        let mut has_previous = false;
        if !header.is_empty() {
            self.index_table_row(header);
            has_previous = true;
        }
        for row in rows {
            if has_previous {
                self.line_break();
            }
            self.index_table_row(row);
            has_previous = true;
        }
    }

    fn index_table_row(&mut self, cells: &mut [Vec<InlineRun>]) {
        for (index, cell) in cells.iter_mut().enumerate() {
            if index > 0 {
                self.text.push('\t');
            }
            self.index_runs(cell);
        }
    }

    fn push_text(&mut self, text: &str) -> Range<usize> {
        let start = self.text.len();
        self.text.push_str(text);
        start..self.text.len()
    }

    fn line_break(&mut self) {
        if !self.text.is_empty() && !self.text.ends_with('\n') {
            self.text.push('\n');
        }
    }
}

fn block_has_selection_text(block: &Block) -> bool {
    match block {
        Block::Paragraph { runs } | Block::Heading { runs, .. } => {
            runs.iter().any(|run| !run.text.is_empty())
        }
        Block::Image { alt, .. } => !alt.is_empty(),
        Block::Code { language, code, .. } => {
            !code.is_empty() && !matches!(language.as_deref(), Some("gallery" | "attachments"))
        }
        Block::Quote { children } => children.iter().any(block_has_selection_text),
        Block::List { items, .. } => items
            .iter()
            .any(|item| item.blocks.iter().any(block_has_selection_text)),
        Block::Table { header, rows, .. } => {
            header.iter().flatten().any(|run| !run.text.is_empty())
                || rows
                    .iter()
                    .flatten()
                    .flatten()
                    .any(|run| !run.text.is_empty())
        }
        Block::Rule => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_document_blocks_and_inline_styles() {
        let blocks = parse("# Heading\n\nA **bold** [link](https://example.com).\n\n- [x] done");
        assert!(matches!(blocks[0], Block::Heading { level: 1, .. }));
        let Block::Paragraph { runs } = &blocks[1] else {
            panic!("expected paragraph");
        };
        assert!(runs.iter().any(|run| run.style.bold));
        assert!(runs.iter().any(|run| run.style.link.is_some()));
        let Block::List { items, .. } = &blocks[2] else {
            panic!("expected list");
        };
        assert_eq!(items[0].task, Some(true));
    }

    #[test]
    fn preserves_locus_link_destinations() {
        let blocks = parse("[Architecture](locus://projects/locus/architecture)");
        let Block::Paragraph { runs } = &blocks[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(
            runs[0].style.link.as_deref(),
            Some("locus://projects/locus/architecture")
        );
    }

    #[test]
    fn separates_images_from_surrounding_paragraph_text() {
        let blocks = parse("before ![diagram](diagram.png) after");
        assert!(matches!(blocks[0], Block::Paragraph { .. }));
        assert!(matches!(blocks[1], Block::Image { .. }));
        assert!(matches!(blocks[2], Block::Paragraph { .. }));
    }

    #[test]
    fn trims_sentence_punctuation_from_bare_links() {
        let blocks = parse("Visit https://example.com/docs.");
        let Block::Paragraph { runs } = &blocks[0] else {
            panic!("expected paragraph");
        };
        let linked = runs.iter().find(|run| run.style.link.is_some()).unwrap();
        assert_eq!(linked.text, "https://example.com/docs");
    }

    #[test]
    fn indexes_rendered_text_across_blocks_lists_and_tables() {
        let mut blocks = parse(
            "# Heading\n\nParagraph\n\n- first\n- [x] second\n\n| A | B |\n| - | - |\n| C | D |",
        );
        let text = index_selection_text(&mut blocks);
        assert_eq!(text, "Heading\nParagraph\n• first\n[x] second\nA\tB\nC\tD");

        let Block::Heading { runs, .. } = &blocks[0] else {
            panic!("expected heading");
        };
        assert_eq!(runs[0].selection_range, 0..7);
        let Block::Paragraph { runs } = &blocks[1] else {
            panic!("expected paragraph");
        };
        assert_eq!(&text[runs[0].selection_range.clone()], "Paragraph");
    }
}
