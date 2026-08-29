use std::borrow::Cow;
use std::cell::Cell;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use alacritty_terminal::event::{Event, EventListener, WindowSize};
use alacritty_terminal::event_loop::{EventLoop, EventLoopSender, Msg};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Line, Point as TerminalPoint, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::cell::{Cell as AlacrittyCell, Flags};
use alacritty_terminal::term::{Config, Term, TermMode};
use alacritty_terminal::tty::{self, Shell};
use alacritty_terminal::vte::ansi::{
    Color, CursorShape as AlacrittyCursorShape, CursorStyle as AlacrittyCursorStyle, NamedColor,
    Rgb,
};
use gpui::{
    App, Bounds, ClipboardItem, Context, ElementInputHandler, EntityInputHandler, FocusHandle,
    Focusable, FontStyle, FontWeight, Hsla, IntoElement, KeyDownEvent, Keystroke, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, Render, Role, ScrollDelta,
    ScrollWheelEvent, SharedString, StrikethroughStyle, StyledText, TextRun, UTF16Selection,
    UnderlineStyle, Window, canvas, div, font, point, prelude::*, px, rgb, size,
};

use crate::localization::Language;
use crate::preferences::{Preferences, TerminalColorScheme, TerminalCursorStyle};
use crate::theme::Theme;

const CELL_WIDTH_FALLBACK: f32 = 7.8;
const CELL_HEIGHT_PADDING: f32 = 5.0;
const PADDING_X: f32 = 10.0;
const PADDING_Y: f32 = 8.0;
const MIN_COLUMNS: usize = 20;
const MIN_ROWS: usize = 8;
const SCROLLBACK_LINES: usize = 10_000;

fn terminal_cell_height(font_size: f32) -> f32 {
    font_size + CELL_HEIGHT_PADDING
}

fn marked_text_range(text: &str) -> Option<Range<usize>> {
    (!text.is_empty()).then(|| 0..text.encode_utf16().count())
}

fn terminal_cell_text(cell: &AlacrittyCell) -> String {
    if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
        return String::new();
    }
    if cell
        .flags
        .intersects(Flags::LEADING_WIDE_CHAR_SPACER | Flags::HIDDEN)
    {
        return " ".to_owned();
    }
    let mut text = cell.c.to_string();
    if let Some(zerowidth) = cell.zerowidth() {
        text.extend(zerowidth);
    }
    text
}

#[derive(Clone)]
pub struct TerminalConfiguration {
    shell: String,
    startup_command: String,
    color_scheme: TerminalColorScheme,
    cursor_style: TerminalCursorStyle,
    code_font_family: SharedString,
    code_font_size: f32,
    language: Language,
}

impl TerminalConfiguration {
    pub fn from_preferences(preferences: &Preferences) -> Self {
        Self {
            shell: preferences.terminal_shell.clone(),
            startup_command: preferences.terminal_command.clone(),
            color_scheme: preferences.terminal_color_scheme,
            cursor_style: preferences.terminal_cursor_style,
            code_font_family: preferences.resolved_code_font_family().to_owned().into(),
            code_font_size: preferences.code_font_size,
            language: preferences.language,
        }
    }
}

enum TerminalUiEvent {
    ClipboardStore(String),
    ClipboardLoad(Arc<dyn Fn(&str) -> String + Send + Sync>),
    Exited,
}

#[derive(Clone)]
struct TerminalEventProxy {
    dirty: Arc<AtomicBool>,
    sender: Arc<OnceLock<EventLoopSender>>,
    ui_events: Sender<TerminalUiEvent>,
    window_size: Arc<Mutex<WindowSize>>,
    dark_palette: Arc<AtomicBool>,
}

impl TerminalEventProxy {
    fn write_pty(&self, bytes: impl Into<Cow<'static, [u8]>>) {
        if let Some(sender) = self.sender.get() {
            let _ = sender.send(Msg::Input(bytes.into()));
        }
    }
}

impl EventListener for TerminalEventProxy {
    fn send_event(&self, event: Event) {
        match event {
            Event::Wakeup | Event::MouseCursorDirty | Event::CursorBlinkingChange => {
                self.dirty.store(true, Ordering::Release);
            }
            Event::ClipboardStore(_, text) => {
                let _ = self.ui_events.send(TerminalUiEvent::ClipboardStore(text));
            }
            Event::ClipboardLoad(_, formatter) => {
                let _ = self
                    .ui_events
                    .send(TerminalUiEvent::ClipboardLoad(formatter));
            }
            Event::PtyWrite(text) => self.write_pty(text.into_bytes()),
            Event::ColorRequest(index, formatter) => {
                let dark = self.dark_palette.load(Ordering::Acquire);
                self.write_pty(formatter(terminal_rgb(index, dark)).into_bytes());
            }
            Event::TextAreaSizeRequest(formatter) => {
                let size = *lock_unpoisoned(&self.window_size);
                self.write_pty(formatter(size).into_bytes());
            }
            Event::Exit | Event::ChildExit(_) => {
                let _ = self.ui_events.send(TerminalUiEvent::Exited);
                self.dirty.store(true, Ordering::Release);
            }
            Event::Title(_) | Event::ResetTitle | Event::Bell => {}
        }
    }
}

struct TerminalDimensions {
    columns: usize,
    rows: usize,
}

impl Dimensions for TerminalDimensions {
    fn total_lines(&self) -> usize {
        self.rows
    }

    fn screen_lines(&self) -> usize {
        self.rows
    }

    fn columns(&self) -> usize {
        self.columns
    }
}

struct TerminalSession {
    term: Arc<FairMutex<Term<TerminalEventProxy>>>,
    sender: EventLoopSender,
    dirty: Arc<AtomicBool>,
    ui_events: Receiver<TerminalUiEvent>,
    window_size: Arc<Mutex<WindowSize>>,
    grid_size: (usize, usize),
    dark_palette: Arc<AtomicBool>,
}

impl TerminalSession {
    fn new(
        working_directory: &Path,
        configuration: &TerminalConfiguration,
        columns: usize,
        rows: usize,
    ) -> Result<Self, String> {
        let columns = columns.max(MIN_COLUMNS);
        let rows = rows.max(MIN_ROWS);
        let cell_height = terminal_cell_height(configuration.code_font_size);
        let window_size = WindowSize {
            num_lines: rows.min(u16::MAX as usize) as u16,
            num_cols: columns.min(u16::MAX as usize) as u16,
            cell_width: CELL_WIDTH_FALLBACK.round() as u16,
            cell_height: cell_height.round() as u16,
        };
        let shared_window_size = Arc::new(Mutex::new(window_size));
        let dirty = Arc::new(AtomicBool::new(true));
        let sender_slot = Arc::new(OnceLock::new());
        let dark_palette = Arc::new(AtomicBool::new(true));
        let (ui_event_tx, ui_events) = channel();
        let proxy = TerminalEventProxy {
            dirty: dirty.clone(),
            sender: sender_slot.clone(),
            ui_events: ui_event_tx,
            window_size: shared_window_size.clone(),
            dark_palette: dark_palette.clone(),
        };
        let term = Arc::new(FairMutex::new(Term::new(
            terminal_config(configuration.cursor_style),
            &TerminalDimensions { columns, rows },
            proxy.clone(),
        )));

        let shell = configured_shell(&configuration.shell);
        let mut options = tty::Options {
            shell: Some(Shell::new(
                shell.to_string_lossy().into_owned(),
                shell_arguments(&shell),
            )),
            working_directory: Some(working_directory.to_path_buf()),
            drain_on_exit: false,
            ..Default::default()
        };
        options.env.insert("TERM".into(), "xterm-256color".into());
        options.env.insert("COLORTERM".into(), "truecolor".into());

        let pty = tty::new(&options, window_size, 0).map_err(|error| {
            format!(
                "{}: {error}",
                configuration
                    .language
                    .text("无法启动终端", "Could not start terminal")
            )
        })?;
        let event_loop = EventLoop::new(term.clone(), proxy, pty, false, false)
            .map_err(|error| error.to_string())?;
        let sender = event_loop.channel();
        sender_slot
            .set(sender.clone())
            .map_err(|_| "Could not initialize terminal input".to_owned())?;
        event_loop.spawn();

        if !configuration.startup_command.trim().is_empty() {
            let mut command = configuration.startup_command.as_bytes().to_vec();
            command.push(b'\r');
            let _ = sender.send(Msg::Input(command.into()));
        }

        Ok(Self {
            term,
            sender,
            dirty,
            ui_events,
            window_size: shared_window_size,
            grid_size: (columns, rows),
            dark_palette,
        })
    }

    fn write(&self, bytes: impl Into<Cow<'static, [u8]>>) {
        let bytes = bytes.into();
        if !bytes.is_empty() {
            let _ = self.sender.send(Msg::Input(bytes));
        }
    }

    fn resize(&mut self, columns: usize, rows: usize, cell_width: f32, cell_height: f32) {
        let columns = columns.max(MIN_COLUMNS);
        let rows = rows.max(MIN_ROWS);
        let size = WindowSize {
            num_lines: rows.min(u16::MAX as usize) as u16,
            num_cols: columns.min(u16::MAX as usize) as u16,
            cell_width: cell_width.round() as u16,
            cell_height: cell_height.round() as u16,
        };
        let current_size = *lock_unpoisoned(&self.window_size);
        let size_changed = current_size.num_lines != size.num_lines
            || current_size.num_cols != size.num_cols
            || current_size.cell_width != size.cell_width
            || current_size.cell_height != size.cell_height;
        if self.grid_size == (columns, rows) && !size_changed {
            return;
        }
        if self.grid_size != (columns, rows) {
            self.grid_size = (columns, rows);
            self.term
                .lock()
                .resize(TerminalDimensions { columns, rows });
        }
        *lock_unpoisoned(&self.window_size) = size;
        let _ = self.sender.send(Msg::Resize(size));
        self.dirty.store(true, Ordering::Release);
    }

    fn mode(&self) -> TermMode {
        *self.term.lock().mode()
    }

    fn scroll(&self, lines: i32) {
        if lines != 0 {
            self.term.lock().scroll_display(Scroll::Delta(lines));
            self.dirty.store(true, Ordering::Release);
        }
    }

    fn set_cursor_style(&self, cursor_style: TerminalCursorStyle) {
        self.term.lock().set_options(terminal_config(cursor_style));
        self.dirty.store(true, Ordering::Release);
    }

    fn snapshot(&self, palette: TerminalPalette, focused: bool) -> TerminalSnapshot {
        self.dark_palette.store(palette.dark, Ordering::Release);
        let term = self.term.lock();
        let content = term.renderable_content();
        let columns = self.grid_size.0;
        let rows = self.grid_size.1;
        let cursor_row = content.cursor.point.line.0 + content.display_offset as i32;
        let cursor_column = content.cursor.point.column.0;
        let cursor_paint = cursor_paint(content.cursor.shape, focused);
        let mut cells = vec![TerminalCell::blank(palette); columns * rows];

        for indexed in content.display_iter {
            let row = indexed.point.line.0 + content.display_offset as i32;
            let column = indexed.point.column.0;
            if row < 0 || row as usize >= rows || column >= columns {
                continue;
            }
            let cell = indexed.cell;
            let text = terminal_cell_text(cell);
            let mut foreground = resolve_color(cell.fg, content.colors, palette, true);
            let mut background = resolve_color(cell.bg, content.colors, palette, false);
            if cell.flags.contains(Flags::INVERSE) {
                std::mem::swap(&mut foreground, &mut background);
            }
            if cell.flags.contains(Flags::DIM) {
                if palette.dark {
                    foreground.l *= 0.7;
                } else {
                    foreground.l = 1.0 - (1.0 - foreground.l) * 0.7;
                }
            }
            if content
                .selection
                .is_some_and(|selection| selection.contains(indexed.point))
            {
                background = palette.selection;
                foreground = palette.foreground;
            }
            if cursor_paint == Some(CursorPaint::Block)
                && row == cursor_row
                && column == cursor_column
            {
                background = palette.foreground;
                foreground = palette.background;
            }
            cells[row as usize * columns + column] = TerminalCell {
                text,
                foreground,
                background,
                bold: cell.flags.contains(Flags::BOLD),
                italic: cell.flags.contains(Flags::ITALIC),
                underline: cell.flags.intersects(Flags::ALL_UNDERLINES),
                strikeout: cell.flags.contains(Flags::STRIKEOUT),
            };
        }

        let rows = cells
            .chunks(columns)
            .map(|row| {
                let mut text = String::new();
                let mut runs: Vec<TerminalRun> = Vec::new();
                for cell in row {
                    if cell.text.is_empty() {
                        continue;
                    }
                    let len = cell.text.len();
                    text.push_str(&cell.text);
                    let style = TerminalRunStyle {
                        foreground: cell.foreground,
                        background: cell.background,
                        bold: cell.bold,
                        italic: cell.italic,
                        underline: cell.underline,
                        strikeout: cell.strikeout,
                    };
                    if let Some(run) = runs.last_mut().filter(|run| run.style == style) {
                        run.len += len;
                    } else {
                        runs.push(TerminalRun { len, style });
                    }
                }
                TerminalRow { text, runs }
            })
            .collect();
        let cursor_position = ((0..self.grid_size.1 as i32).contains(&cursor_row)
            && cursor_column < self.grid_size.0)
            .then_some((cursor_row as usize, cursor_column));
        TerminalSnapshot {
            rows,
            cursor: cursor_position.zip(cursor_paint),
            cursor_position,
        }
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = self.sender.send(Msg::Shutdown);
    }
}

#[derive(Clone, Copy)]
struct TerminalPalette {
    dark: bool,
    background: Hsla,
    foreground: Hsla,
    selection: Hsla,
}

impl TerminalPalette {
    fn resolve(color_scheme: TerminalColorScheme, app_theme: Theme) -> Self {
        let dark = match color_scheme {
            TerminalColorScheme::Auto => app_theme.is_dark,
            TerminalColorScheme::Light => false,
            TerminalColorScheme::Dark => true,
        };
        if dark {
            Self {
                dark,
                background: rgb(0x171717).into(),
                foreground: rgb(0xe5e5e5).into(),
                selection: gpui::hsla(214.0 / 360.0, 0.75, 0.55, 0.34),
            }
        } else {
            Self {
                dark,
                background: rgb(0xfafafa).into(),
                foreground: rgb(0x242424).into(),
                selection: gpui::hsla(214.0 / 360.0, 0.75, 0.55, 0.24),
            }
        }
    }
}

#[derive(Clone)]
struct TerminalCell {
    text: String,
    foreground: Hsla,
    background: Hsla,
    bold: bool,
    italic: bool,
    underline: bool,
    strikeout: bool,
}

impl TerminalCell {
    fn blank(palette: TerminalPalette) -> Self {
        Self {
            text: " ".into(),
            foreground: palette.foreground,
            background: palette.background,
            bold: false,
            italic: false,
            underline: false,
            strikeout: false,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
struct TerminalRunStyle {
    foreground: Hsla,
    background: Hsla,
    bold: bool,
    italic: bool,
    underline: bool,
    strikeout: bool,
}

struct TerminalRun {
    len: usize,
    style: TerminalRunStyle,
}

struct TerminalRow {
    text: String,
    runs: Vec<TerminalRun>,
}

struct TerminalSnapshot {
    rows: Vec<TerminalRow>,
    cursor: Option<((usize, usize), CursorPaint)>,
    cursor_position: Option<(usize, usize)>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum CursorPaint {
    Block,
    Underline,
    Bar,
    Outline,
}

pub struct TerminalView {
    session: Option<TerminalSession>,
    error: Option<String>,
    focus_handle: FocusHandle,
    configuration: TerminalConfiguration,
    available_width: f32,
    available_height: f32,
    measured_cell_width: Option<f32>,
    grid_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    selecting: bool,
    scroll_accumulator: f32,
    exited: bool,
    marked_text: String,
}

impl TerminalView {
    pub fn new(
        working_directory: PathBuf,
        configuration: TerminalConfiguration,
        cx: &mut Context<Self>,
    ) -> Self {
        let session_configuration = configuration.clone();
        cx.spawn(async move |this, cx| {
            let started = cx
                .background_executor()
                .spawn(async move {
                    TerminalSession::new(&working_directory, &session_configuration, 52, 36)
                })
                .await;
            if this
                .update(cx, |this, cx| {
                    match started {
                        Ok(session) => this.session = Some(session),
                        Err(error) => this.error = Some(error),
                    }
                    cx.notify();
                })
                .is_err()
            {
                return;
            }
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(24))
                    .await;
                if this
                    .update(cx, |this, cx| {
                        if this.poll(cx) {
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();

        Self {
            session: None,
            error: None,
            focus_handle: cx.focus_handle(),
            configuration,
            available_width: 520.0,
            available_height: 820.0,
            measured_cell_width: None,
            grid_bounds: Rc::new(Cell::new(None)),
            selecting: false,
            scroll_accumulator: 0.0,
            exited: false,
            marked_text: String::new(),
        }
    }

    pub fn set_available_size(&mut self, width: f32, height: f32) {
        self.available_width = width;
        self.available_height = height;
    }

    pub fn set_color_scheme(&mut self, color_scheme: TerminalColorScheme, cx: &mut Context<Self>) {
        self.configuration.color_scheme = color_scheme;
        if let Some(session) = &self.session {
            session.dirty.store(true, Ordering::Release);
        }
        cx.notify();
    }

    pub fn set_cursor_style(&mut self, cursor_style: TerminalCursorStyle, cx: &mut Context<Self>) {
        self.configuration.cursor_style = cursor_style;
        if let Some(session) = &self.session {
            session.set_cursor_style(cursor_style);
        }
        cx.notify();
    }

    pub fn set_code_font(
        &mut self,
        family: impl Into<SharedString>,
        size: f32,
        cx: &mut Context<Self>,
    ) {
        self.configuration.code_font_family = family.into();
        self.configuration.code_font_size = size;
        self.measured_cell_width = None;
        if let Some(session) = &self.session {
            session.dirty.store(true, Ordering::Release);
        }
        cx.notify();
    }

    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.configuration.language = language;
        cx.notify();
    }

    fn write_text(&mut self, text: &str, cx: &mut Context<Self>) {
        self.marked_text.clear();
        let Some(session) = &self.session else {
            cx.notify();
            return;
        };
        if !text.is_empty() {
            session.term.lock().selection = None;
            session.write(text.as_bytes().to_vec());
            session.dirty.store(true, Ordering::Release);
        }
        cx.notify();
    }

    fn ime_cursor_bounds(&self) -> Option<Bounds<Pixels>> {
        let bounds = self.grid_bounds.get()?;
        let session = self.session.as_ref()?;
        let term = session.term.lock();
        let content = term.renderable_content();
        let row = content.cursor.point.line.0 + content.display_offset as i32;
        let column = content.cursor.point.column.0;
        if !(0..session.grid_size.1 as i32).contains(&row) || column >= session.grid_size.0 {
            return None;
        }
        let cell_width = self.measured_cell_width.unwrap_or(CELL_WIDTH_FALLBACK);
        let cell_height = terminal_cell_height(self.configuration.code_font_size);
        Some(Bounds::new(
            point(
                bounds.left() + px(column as f32 * cell_width),
                bounds.top() + px(row as f32 * cell_height),
            ),
            size(px(cell_width), px(cell_height)),
        ))
    }

    fn poll(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(session) = &self.session else {
            return false;
        };
        let mut changed = session.dirty.swap(false, Ordering::AcqRel);
        while let Ok(event) = session.ui_events.try_recv() {
            changed = true;
            match event {
                TerminalUiEvent::ClipboardStore(text) => {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
                TerminalUiEvent::ClipboardLoad(formatter) => {
                    let text = cx
                        .read_from_clipboard()
                        .and_then(|item| item.text())
                        .unwrap_or_default();
                    session.write(formatter(&text).into_bytes());
                }
                TerminalUiEvent::Exited => self.exited = true,
            }
        }
        changed
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let modifiers = &event.keystroke.modifiers;
        let clipboard_modifier = if cfg!(target_os = "macos") {
            modifiers.secondary() && !modifiers.control && !modifiers.alt
        } else {
            modifiers.control && modifiers.shift && !modifiers.alt && !modifiers.platform
        };
        if clipboard_modifier && event.keystroke.key.eq_ignore_ascii_case("c") {
            if let Some(text) = self.selected_text() {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
            }
            window.prevent_default();
            cx.stop_propagation();
            return;
        }
        if clipboard_modifier && event.keystroke.key.eq_ignore_ascii_case("a") {
            self.select_all(cx);
            window.prevent_default();
            cx.stop_propagation();
            return;
        }
        let Some(session) = &self.session else {
            return;
        };
        if clipboard_modifier && event.keystroke.key.eq_ignore_ascii_case("v") {
            if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                session.term.lock().selection = None;
                session.write(bracketed_paste(text, session.mode()));
                session.dirty.store(true, Ordering::Release);
                window.prevent_default();
                cx.stop_propagation();
            }
            return;
        }
        if let Some(bytes) = terminal_key_bytes(&event.keystroke, session.mode()) {
            session.term.lock().selection = None;
            session.write(bytes);
            session.dirty.store(true, Ordering::Release);
            window.prevent_default();
            cx.stop_propagation();
        }
    }

    fn grid_point(&self, position: Point<Pixels>, clamp: bool) -> Option<(TerminalPoint, Side)> {
        let bounds = self.grid_bounds.get()?;
        let session = self.session.as_ref()?;
        if !clamp && !bounds.contains(&position) {
            return None;
        }
        let columns = session.grid_size.0;
        let rows = session.grid_size.1;
        let cell_width = self.measured_cell_width.unwrap_or(CELL_WIDTH_FALLBACK);
        let cell_height = terminal_cell_height(self.configuration.code_font_size);
        let x = f32::from(position.x - bounds.origin.x).clamp(0.0, columns as f32 * cell_width);
        let y = f32::from(position.y - bounds.origin.y).clamp(0.0, rows as f32 * cell_height);
        let column = ((x / cell_width).floor() as usize).min(columns - 1);
        let viewport_row = ((y / cell_height).floor() as usize).min(rows - 1) as i32;
        let display_offset = session.term.lock().grid().display_offset();
        let line = Line(viewport_row - display_offset.min(i32::MAX as usize) as i32);
        let side = if x >= columns as f32 * cell_width || x % cell_width >= cell_width / 2.0 {
            Side::Right
        } else {
            Side::Left
        };
        Some((TerminalPoint::new(line, Column(column)), side))
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle, cx);
        let Some((point, side)) = self.grid_point(event.position, false) else {
            return;
        };
        let Some(session) = &self.session else {
            return;
        };
        let mut term = session.term.lock();
        if event.modifiers.shift
            && let Some(selection) = term.selection.as_mut()
        {
            selection.update(point, side);
        } else {
            let selection_type = match event.click_count {
                2 => SelectionType::Semantic,
                count if count >= 3 => SelectionType::Lines,
                _ => SelectionType::Simple,
            };
            term.selection = Some(Selection::new(selection_type, point, side));
        }
        drop(term);
        self.selecting = true;
        session.dirty.store(true, Ordering::Release);
        cx.notify();
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selecting || event.pressed_button != Some(MouseButton::Left) {
            return;
        }
        let Some((point, side)) = self.grid_point(event.position, true) else {
            return;
        };
        let Some(session) = &self.session else {
            return;
        };
        if let Some(selection) = session.term.lock().selection.as_mut() {
            selection.update(point, side);
            session.dirty.store(true, Ordering::Release);
            cx.notify();
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.selecting = false;
    }

    fn on_scroll_wheel(
        &mut self,
        event: &ScrollWheelEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = &self.session else {
            return;
        };
        let delta = match event.delta {
            ScrollDelta::Pixels(delta) => {
                f32::from(delta.y) / terminal_cell_height(self.configuration.code_font_size)
            }
            ScrollDelta::Lines(delta) => delta.y,
        };
        self.scroll_accumulator += delta;
        let lines = self.scroll_accumulator.trunc() as i32;
        if lines != 0 {
            self.scroll_accumulator -= lines as f32;
            session.scroll(lines);
            cx.stop_propagation();
            cx.notify();
        }
    }

    fn selected_text(&self) -> Option<String> {
        self.session
            .as_ref()?
            .term
            .lock()
            .selection_to_string()
            .filter(|text| !text.is_empty())
    }

    fn select_all(&mut self, cx: &mut Context<Self>) {
        let Some(session) = &self.session else {
            return;
        };
        let mut term = session.term.lock();
        let start = TerminalPoint::new(term.topmost_line(), Column(0));
        let end = TerminalPoint::new(term.bottommost_line(), term.last_column());
        let mut selection = Selection::new(SelectionType::Simple, start, Side::Left);
        selection.update(end, Side::Right);
        term.selection = Some(selection);
        drop(term);
        session.dirty.store(true, Ordering::Release);
        cx.notify();
    }
}

impl EntityInputHandler for TerminalView {
    fn text_for_range(
        &mut self,
        _: Range<usize>,
        _: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        None
    }

    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: 0..0,
            reversed: false,
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        marked_text_range(&self.marked_text)
    }

    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        if !self.marked_text.is_empty() {
            self.marked_text.clear();
            cx.notify();
        }
    }

    fn paste(&mut self, item: ClipboardItem, _: &mut Window, cx: &mut Context<Self>) {
        let Some(text) = item.text() else {
            return;
        };
        self.marked_text.clear();
        let Some(session) = &self.session else {
            cx.notify();
            return;
        };
        session.term.lock().selection = None;
        session.write(bracketed_paste(text, session.mode()));
        session.dirty.store(true, Ordering::Release);
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        _: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.write_text(text, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        _: Option<Range<usize>>,
        new_text: &str,
        _: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.marked_text != new_text {
            self.marked_text.clear();
            self.marked_text.push_str(new_text);
            cx.notify();
        }
    }

    fn bounds_for_range(
        &mut self,
        range: Range<usize>,
        _: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let mut bounds = self.ime_cursor_bounds()?;
        let cell_width = self.measured_cell_width.unwrap_or(CELL_WIDTH_FALLBACK);
        bounds.origin.x += px(cell_width * range.start as f32);
        Some(bounds)
    }

    fn character_index_for_point(
        &mut self,
        _: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        None
    }
}

impl Focusable for TerminalView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TerminalView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = TerminalPalette::resolve(self.configuration.color_scheme, Theme::current(cx));
        let terminal_font = font(self.configuration.code_font_family.clone());
        let font_size = self.configuration.code_font_size;
        let cell_height = terminal_cell_height(font_size);
        let cell_width = *self.measured_cell_width.get_or_insert_with(|| {
            let text_system = cx.text_system();
            let font_id = text_system.resolve_font(&terminal_font);
            text_system
                .advance(font_id, px(font_size), 'm')
                .map_or(CELL_WIDTH_FALLBACK, |advance| f32::from(advance.width))
        });
        let columns = ((self.available_width - PADDING_X * 2.0) / cell_width)
            .floor()
            .max(MIN_COLUMNS as f32) as usize;
        let rows = ((self.available_height - PADDING_Y * 2.0) / cell_height)
            .floor()
            .max(MIN_ROWS as f32) as usize;
        if let Some(session) = self.session.as_mut() {
            session.resize(columns, rows, cell_width, cell_height);
        }
        let focused = window.is_window_active() && self.focus_handle.is_focused(window);
        let snapshot = self
            .session
            .as_ref()
            .map(|session| session.snapshot(palette, focused));

        let mut screen = div()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .overflow_hidden()
            .flex()
            .flex_col()
            .relative()
            .cursor_text()
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move));

        if let Some(snapshot) = snapshot {
            let cursor_position = snapshot.cursor_position;
            for row in snapshot.rows {
                let runs = row
                    .runs
                    .into_iter()
                    .map(|run| {
                        let mut run_font = terminal_font.clone();
                        if run.style.bold {
                            run_font.weight = FontWeight::BOLD;
                        }
                        if run.style.italic {
                            run_font.style = FontStyle::Italic;
                        }
                        TextRun {
                            len: run.len,
                            font: run_font,
                            color: run.style.foreground,
                            background_color: Some(run.style.background),
                            underline: run.style.underline.then_some(UnderlineStyle {
                                thickness: px(1.0),
                                color: Some(run.style.foreground),
                                wavy: false,
                            }),
                            strikethrough: run.style.strikeout.then_some(StrikethroughStyle {
                                thickness: px(1.0),
                                color: Some(run.style.foreground),
                            }),
                        }
                    })
                    .collect::<Vec<_>>();
                screen = screen.child(
                    div()
                        .h(px(cell_height))
                        .flex_none()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_size(px(font_size))
                        .line_height(px(cell_height))
                        .child(StyledText::new(row.text).with_runs(runs)),
                );
            }
            if let Some(((row, column), cursor)) = snapshot.cursor {
                let left = column as f32 * cell_width;
                let top = row as f32 * cell_height;
                match cursor {
                    CursorPaint::Block => {}
                    CursorPaint::Outline => {
                        screen = screen.child(
                            div()
                                .absolute()
                                .left(px(left))
                                .top(px(top))
                                .w(px(cell_width))
                                .h(px(cell_height))
                                .border_1()
                                .border_color(palette.foreground),
                        );
                    }
                    CursorPaint::Underline => {
                        screen = screen.child(
                            div()
                                .absolute()
                                .left(px(left))
                                .top(px(top + cell_height - 2.0))
                                .w(px(cell_width))
                                .h(px(2.0))
                                .bg(palette.foreground),
                        );
                    }
                    CursorPaint::Bar => {
                        screen = screen.child(
                            div()
                                .absolute()
                                .left(px(left))
                                .top(px(top))
                                .w(px(2.0))
                                .h(px(cell_height))
                                .bg(palette.foreground),
                        );
                    }
                }
            }
            if !self.marked_text.is_empty()
                && let Some((row, column)) = cursor_position
            {
                screen = screen.child(
                    div()
                        .absolute()
                        .left(px(column as f32 * cell_width))
                        .top(px(row as f32 * cell_height))
                        .h(px(cell_height))
                        .bg(palette.background)
                        .font_family(self.configuration.code_font_family.clone())
                        .text_size(px(font_size))
                        .line_height(px(cell_height))
                        .text_color(palette.foreground)
                        .whitespace_nowrap()
                        .underline()
                        .child(SharedString::from(self.marked_text.clone())),
                );
            }
        } else {
            let message = self.error.clone().unwrap_or_else(|| {
                self.configuration
                    .language
                    .text("正在启动终端…", "Starting terminal…")
                    .to_owned()
            });
            screen = screen.child(
                div()
                    .p(px(12.0))
                    .font_family(self.configuration.code_font_family.clone())
                    .text_size(px(font_size))
                    .line_height(px(cell_height))
                    .text_color(if self.error.is_some() {
                        Theme::current(cx).error
                    } else {
                        palette.foreground
                    })
                    .child(SharedString::from(message)),
            );
        }
        if self.exited {
            screen = screen.child(
                div()
                    .absolute()
                    .bottom(px(8.0))
                    .right(px(10.0))
                    .text_xs()
                    .text_color(palette.foreground.opacity(0.55))
                    .child(
                        self.configuration
                            .language
                            .text("进程已退出", "Process exited"),
                    ),
            );
        }
        let grid_bounds = self.grid_bounds.clone();
        let input = cx.entity();
        let focus = self.focus_handle.clone();
        screen = screen.child(
            canvas(
                move |bounds, _, _| grid_bounds.set(Some(bounds)),
                move |bounds, _, window, cx| {
                    window.handle_input(&focus, ElementInputHandler::new(bounds, input), cx);
                },
            )
            .absolute()
            .inset_0(),
        );

        div()
            .id("terminal-view")
            .key_context("Terminal")
            .role(Role::Terminal)
            .aria_label(self.configuration.language.text("终端", "Terminal"))
            .track_focus(&self.focus_handle)
            .size_full()
            .min_h_0()
            .min_w_0()
            .flex()
            .flex_col()
            .bg(palette.background)
            .px(px(PADDING_X))
            .py(px(PADDING_Y))
            .child(screen)
            .on_key_down(cx.listener(Self::on_key_down))
            .on_scroll_wheel(cx.listener(Self::on_scroll_wheel))
    }
}

fn terminal_config(cursor_style: TerminalCursorStyle) -> Config {
    Config {
        scrolling_history: SCROLLBACK_LINES,
        default_cursor_style: AlacrittyCursorStyle {
            shape: match cursor_style {
                TerminalCursorStyle::Block => AlacrittyCursorShape::Block,
                TerminalCursorStyle::Underline => AlacrittyCursorShape::Underline,
                TerminalCursorStyle::Bar => AlacrittyCursorShape::Beam,
            },
            blinking: false,
        },
        ..Default::default()
    }
}

fn cursor_paint(shape: AlacrittyCursorShape, focused: bool) -> Option<CursorPaint> {
    if shape == AlacrittyCursorShape::Hidden {
        return None;
    }
    if !focused || shape == AlacrittyCursorShape::HollowBlock {
        return Some(CursorPaint::Outline);
    }
    Some(match shape {
        AlacrittyCursorShape::Block => CursorPaint::Block,
        AlacrittyCursorShape::Underline => CursorPaint::Underline,
        AlacrittyCursorShape::Beam => CursorPaint::Bar,
        AlacrittyCursorShape::HollowBlock | AlacrittyCursorShape::Hidden => unreachable!(),
    })
}

fn configured_shell(configured: &str) -> PathBuf {
    let configured = configured.trim();
    if !configured.is_empty() {
        let path = PathBuf::from(configured);
        if path.components().count() != 1 {
            return path;
        }
        return executable_in_path(configured)
            .or_else(|| executable_from_login_shell(configured))
            .unwrap_or(path);
    }
    std::env::var_os("SHELL")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            if cfg!(target_os = "macos") {
                PathBuf::from("/bin/zsh")
            } else {
                PathBuf::from("/bin/sh")
            }
        })
}

fn executable_in_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|directory| directory.join(name))
        .find(|path| is_executable(path))
}

#[cfg(unix)]
fn executable_from_login_shell(name: &str) -> Option<PathBuf> {
    let login_shell = std::env::var_os("SHELL")
        .map(PathBuf::from)
        .filter(|path| is_executable(path))
        .unwrap_or_else(|| {
            if cfg!(target_os = "macos") {
                PathBuf::from("/bin/zsh")
            } else {
                PathBuf::from("/bin/sh")
            }
        });
    let output = Command::new(login_shell)
        .args(["-l", "-i", "-c", "command -v -- \"$LOCUS_TERMINAL_SHELL\""])
        .env("LOCUS_TERMINAL_SHELL", name)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()?
        .lines()
        .rev()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .find(|path| path.is_absolute() && is_executable(path))
}

#[cfg(not(unix))]
fn executable_from_login_shell(_: &str) -> Option<PathBuf> {
    None
}

fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn shell_arguments(_: &Path) -> Vec<String> {
    if cfg!(windows) {
        Vec::new()
    } else {
        vec!["-l".to_owned()]
    }
}

fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}

fn bracketed_paste(text: String, mode: TermMode) -> Vec<u8> {
    if mode.contains(TermMode::BRACKETED_PASTE) {
        format!("\x1b[200~{text}\x1b[201~").into_bytes()
    } else {
        text.into_bytes()
    }
}

fn terminal_key_bytes(keystroke: &Keystroke, mode: TermMode) -> Option<Vec<u8>> {
    let modifiers = keystroke.modifiers;
    let key = keystroke.key.as_str();
    if modifiers.platform {
        #[cfg(target_os = "macos")]
        return match key {
            "left" => Some(vec![0x01]),
            "right" => Some(vec![0x05]),
            "backspace" => Some(vec![0x15]),
            _ => None,
        };
        #[cfg(not(target_os = "macos"))]
        return None;
    }
    let modifier = 1
        + u8::from(modifiers.shift)
        + u8::from(modifiers.alt) * 2
        + u8::from(modifiers.control) * 4;
    let app_cursor = mode.contains(TermMode::APP_CURSOR);
    let special = match key {
        "enter" | "return"
            if modifiers.shift && !modifiers.alt && !modifiers.control && !modifiers.platform =>
        {
            Some("\n".to_owned())
        }
        "enter" | "return"
            if modifiers.alt && !modifiers.shift && !modifiers.control && !modifiers.platform =>
        {
            Some("\x1b\r".to_owned())
        }
        "enter" | "return" => Some("\r".to_owned()),
        "tab" if modifiers.shift => Some("\x1b[Z".to_owned()),
        "tab" => Some("\t".to_owned()),
        "backspace" if modifiers.control && !modifiers.alt && !modifiers.platform => {
            Some("\x08".to_owned())
        }
        "backspace" if modifiers.alt && !modifiers.control && !modifiers.platform => {
            Some("\x1b\x7f".to_owned())
        }
        "backspace" => Some("\x7f".to_owned()),
        "space" if modifiers.control && !modifiers.alt && !modifiers.platform => {
            Some("\x00".to_owned())
        }
        "escape" => Some("\x1b".to_owned()),
        "up" => Some(cursor_sequence('A', modifier, app_cursor)),
        "down" => Some(cursor_sequence('B', modifier, app_cursor)),
        "right" => Some(cursor_sequence('C', modifier, app_cursor)),
        "left" => Some(cursor_sequence('D', modifier, app_cursor)),
        "home" => Some(csi_sequence('H', modifier)),
        "end" => Some(csi_sequence('F', modifier)),
        "insert" => Some(tilde_sequence(2, modifier)),
        "delete" | "forwarddelete" => Some(tilde_sequence(3, modifier)),
        "pageup" => Some(tilde_sequence(5, modifier)),
        "pagedown" => Some(tilde_sequence(6, modifier)),
        "f1" => Some(function_sequence('P', modifier)),
        "f2" => Some(function_sequence('Q', modifier)),
        "f3" => Some(function_sequence('R', modifier)),
        "f4" => Some(function_sequence('S', modifier)),
        "f5" => Some(tilde_sequence(15, modifier)),
        "f6" => Some(tilde_sequence(17, modifier)),
        "f7" => Some(tilde_sequence(18, modifier)),
        "f8" => Some(tilde_sequence(19, modifier)),
        "f9" => Some(tilde_sequence(20, modifier)),
        "f10" => Some(tilde_sequence(21, modifier)),
        "f11" => Some(tilde_sequence(23, modifier)),
        "f12" => Some(tilde_sequence(24, modifier)),
        _ => None,
    };
    if let Some(special) = special {
        return Some(special.into_bytes());
    }
    let text = if modifiers.control {
        (key.chars().count() == 1).then_some(key)
    } else {
        keystroke
            .key_char
            .as_deref()
            .or_else(|| (key.chars().count() == 1).then_some(key))
    }?;
    let mut bytes = if modifiers.control {
        control_bytes(text)?
    } else {
        text.as_bytes().to_vec()
    };
    if modifiers.alt {
        bytes.insert(0, 0x1b);
    }
    Some(bytes)
}

fn control_bytes(text: &str) -> Option<Vec<u8>> {
    let character = text.chars().next()?.to_ascii_lowercase();
    let byte = match character {
        ' ' | '@' => 0,
        'a'..='z' => character as u8 - b'a' + 1,
        '[' => 27,
        '\\' => 28,
        ']' => 29,
        '^' => 30,
        '_' => 31,
        '?' => 127,
        _ => return None,
    };
    Some(vec![byte])
}

fn cursor_sequence(final_byte: char, modifier: u8, app_cursor: bool) -> String {
    if modifier == 1 {
        format!("\x1b{}{}", if app_cursor { 'O' } else { '[' }, final_byte)
    } else {
        format!("\x1b[1;{modifier}{final_byte}")
    }
}

fn csi_sequence(final_byte: char, modifier: u8) -> String {
    if modifier == 1 {
        format!("\x1b[{final_byte}")
    } else {
        format!("\x1b[1;{modifier}{final_byte}")
    }
}

fn function_sequence(final_byte: char, modifier: u8) -> String {
    if modifier == 1 {
        format!("\x1bO{final_byte}")
    } else {
        format!("\x1b[1;{modifier}{final_byte}")
    }
}

fn tilde_sequence(number: u8, modifier: u8) -> String {
    if modifier == 1 {
        format!("\x1b[{number}~")
    } else {
        format!("\x1b[{number};{modifier}~")
    }
}

fn resolve_color(
    color: Color,
    colors: &alacritty_terminal::term::color::Colors,
    palette: TerminalPalette,
    foreground: bool,
) -> Hsla {
    let rgb = match color {
        Color::Spec(color) => Some(color),
        Color::Indexed(index) => {
            colors[index as usize].or_else(|| Some(terminal_rgb(index as usize, palette.dark)))
        }
        Color::Named(NamedColor::Foreground | NamedColor::BrightForeground) => None,
        Color::Named(NamedColor::Background) => return palette.background,
        Color::Named(NamedColor::Cursor) => return palette.foreground,
        Color::Named(named) => {
            colors[named as usize].or_else(|| Some(terminal_rgb(named as usize, palette.dark)))
        }
    };
    rgb.map(rgb_to_hsla).unwrap_or(if foreground {
        palette.foreground
    } else {
        palette.background
    })
}

fn rgb_to_hsla(color: Rgb) -> Hsla {
    rgb((u32::from(color.r) << 16) | (u32::from(color.g) << 8) | u32::from(color.b)).into()
}

fn terminal_rgb(index: usize, dark: bool) -> Rgb {
    const DARK: [u32; 16] = [
        0x1d1f21, 0xcc6666, 0xb5bd68, 0xf0c674, 0x81a2be, 0xb294bb, 0x8abeb7, 0xc5c8c6, 0x666666,
        0xd54e53, 0xb9ca4a, 0xe7c547, 0x7aa6da, 0xc397d8, 0x70c0b1, 0xeaeaea,
    ];
    const LIGHT: [u32; 16] = [
        0x000000, 0xc82829, 0x718c00, 0xeab700, 0x4271ae, 0x8959a8, 0x3e999f, 0xc7c7c7, 0x8e908c,
        0xc82829, 0x718c00, 0xeab700, 0x4271ae, 0x8959a8, 0x3e999f, 0xffffff,
    ];
    let ansi = if dark { &DARK } else { &LIGHT };
    let value = match index {
        0..=15 => ansi[index],
        16..=231 => {
            let index = index - 16;
            let channel = |value: usize| {
                if value == 0 {
                    0
                } else {
                    55 + value as u32 * 40
                }
            };
            (channel(index / 36) << 16) | (channel((index / 6) % 6) << 8) | channel(index % 6)
        }
        232..=255 => {
            let value = 8 + (index as u32 - 232) * 10;
            (value << 16) | (value << 8) | value
        }
        _ => {
            if dark {
                0xe5e5e5
            } else {
                0x242424
            }
        }
    };
    Rgb {
        r: (value >> 16) as u8,
        g: (value >> 8) as u8,
        b: value as u8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alacritty_terminal::event::VoidListener;
    use alacritty_terminal::vte::ansi::Processor;
    use gpui::Modifiers;

    fn key(key: &str, key_char: Option<&str>, modifiers: Modifiers) -> Keystroke {
        Keystroke {
            key: key.to_owned(),
            key_char: key_char.map(str::to_owned),
            modifiers,
        }
    }

    #[test]
    fn empty_shell_uses_a_platform_default() {
        assert!(!configured_shell("").as_os_str().is_empty());
    }

    #[test]
    fn configured_shell_is_used_verbatim() {
        assert_eq!(configured_shell(" /bin/bash "), PathBuf::from("/bin/bash"));
    }

    #[test]
    fn configured_shell_name_resolves_to_an_executable() {
        let shell = configured_shell("sh");

        assert!(shell.is_absolute());
        assert!(is_executable(&shell));
        assert_eq!(shell.file_name().and_then(|name| name.to_str()), Some("sh"));
    }

    #[cfg(unix)]
    #[test]
    fn login_shell_environment_resolves_executable_names() {
        let shell = executable_from_login_shell("sh").expect("login shell should resolve sh");

        assert!(shell.is_absolute());
        assert!(is_executable(&shell));
    }

    #[test]
    fn control_shortcuts_use_the_physical_key() {
        let modifiers = Modifiers {
            control: true,
            ..Default::default()
        };

        assert_eq!(
            terminal_key_bytes(&key("c", Some("\u{3}"), modifiers), TermMode::empty()),
            Some(vec![3])
        );
        assert_eq!(
            terminal_key_bytes(&key("left", None, modifiers), TermMode::empty()),
            Some(b"\x1b[1;5D".to_vec())
        );
    }

    #[test]
    fn modified_enter_and_editing_keys_use_terminal_sequences() {
        let shift = Modifiers {
            shift: true,
            ..Default::default()
        };
        let alt = Modifiers {
            alt: true,
            ..Default::default()
        };
        let control = Modifiers {
            control: true,
            ..Default::default()
        };

        assert_eq!(
            terminal_key_bytes(&key("enter", None, shift), TermMode::empty()),
            Some(b"\n".to_vec())
        );
        assert_eq!(
            terminal_key_bytes(&key("enter", None, alt), TermMode::empty()),
            Some(b"\x1b\r".to_vec())
        );
        assert_eq!(
            terminal_key_bytes(&key("backspace", None, control), TermMode::empty()),
            Some(vec![0x08])
        );
        assert_eq!(
            terminal_key_bytes(&key("backspace", None, alt), TermMode::empty()),
            Some(vec![0x1b, 0x7f])
        );
        assert_eq!(
            terminal_key_bytes(&key("space", Some(" "), control), TermMode::empty()),
            Some(vec![0])
        );
    }

    #[test]
    fn cursor_paint_uses_terminal_shape_and_focus_state() {
        assert_eq!(
            cursor_paint(AlacrittyCursorShape::Block, true),
            Some(CursorPaint::Block)
        );
        assert_eq!(
            cursor_paint(AlacrittyCursorShape::Underline, true),
            Some(CursorPaint::Underline)
        );
        assert_eq!(
            cursor_paint(AlacrittyCursorShape::Beam, true),
            Some(CursorPaint::Bar)
        );
        assert_eq!(
            cursor_paint(AlacrittyCursorShape::Block, false),
            Some(CursorPaint::Outline)
        );
        assert_eq!(cursor_paint(AlacrittyCursorShape::Hidden, true), None);
    }

    #[test]
    fn ime_marked_range_counts_utf16_units() {
        assert_eq!(marked_text_range("中文"), Some(0..2));
        assert_eq!(marked_text_range("😀"), Some(0..2));
        assert_eq!(marked_text_range(""), None);
    }

    #[test]
    fn wide_character_spacer_does_not_add_a_third_cell() {
        let dimensions = TerminalDimensions {
            columns: 4,
            rows: 1,
        };
        let mut term = Term::new(Config::default(), &dimensions, VoidListener);
        let mut processor: Processor = Processor::new();
        processor.advance(&mut term, "中".as_bytes());

        let wide = &term.grid()[Line(0)][Column(0)];
        let spacer = &term.grid()[Line(0)][Column(1)];
        assert!(wide.flags.contains(Flags::WIDE_CHAR));
        assert!(spacer.flags.contains(Flags::WIDE_CHAR_SPACER));
        assert_eq!(terminal_cell_text(wide), "中");
        assert_eq!(terminal_cell_text(spacer), "");
    }

    #[cfg(unix)]
    #[test]
    fn pty_starts_in_the_requested_working_directory() {
        let configuration = TerminalConfiguration {
            shell: "/bin/sh".to_owned(),
            startup_command: "pwd; exit".to_owned(),
            color_scheme: TerminalColorScheme::Auto,
            cursor_style: TerminalCursorStyle::Block,
            code_font_family: "monospace".into(),
            code_font_size: 14.0,
            language: Language::English,
        };
        let working_directory = std::fs::canonicalize("/tmp").unwrap();
        let expected = working_directory.to_string_lossy().into_owned();
        let session = TerminalSession::new(&working_directory, &configuration, 80, 24).unwrap();
        let mut last_text = String::new();

        for _ in 0..50 {
            {
                let mut term = session.term.lock();
                let start = TerminalPoint::new(term.topmost_line(), Column(0));
                let end = TerminalPoint::new(term.bottommost_line(), term.last_column());
                let mut selection = Selection::new(SelectionType::Simple, start, Side::Left);
                selection.update(end, Side::Right);
                term.selection = Some(selection);
                last_text = term.selection_to_string().unwrap_or_default();
                if last_text.lines().any(|line| line.trim() == expected) {
                    return;
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }

        panic!("PTY did not report the requested working directory: {last_text:?}");
    }
}
