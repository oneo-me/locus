use super::*;

pub(super) fn terminal_cell_height(font_size: f32) -> f32 {
    font_size + CELL_HEIGHT_PADDING
}

pub(super) fn marked_text_range(text: &str) -> Option<Range<usize>> {
    (!text.is_empty()).then(|| 0..text.encode_utf16().count())
}

pub(super) fn terminal_cell_text(cell: &AlacrittyCell) -> String {
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

#[derive(Clone, Copy)]
pub(super) struct TerminalPalette {
    pub(super) dark: bool,
    pub(super) background: Hsla,
    pub(super) foreground: Hsla,
    pub(super) selection: Hsla,
}

impl TerminalPalette {
    fn resolve(color_scheme: TerminalColorScheme, app_theme: Theme) -> Self {
        let dark = match color_scheme {
            TerminalColorScheme::Auto => app_theme.is_dark(),
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
pub(super) struct TerminalCell {
    pub(super) text: String,
    pub(super) foreground: Hsla,
    pub(super) background: Hsla,
    pub(super) bold: bool,
    pub(super) italic: bool,
    pub(super) underline: bool,
    pub(super) strikeout: bool,
}

impl TerminalCell {
    pub(super) fn blank(palette: TerminalPalette) -> Self {
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
pub(super) struct TerminalRunStyle {
    pub(super) foreground: Hsla,
    pub(super) background: Hsla,
    pub(super) bold: bool,
    pub(super) italic: bool,
    pub(super) underline: bool,
    pub(super) strikeout: bool,
}

pub(super) struct TerminalRun {
    pub(super) len: usize,
    pub(super) style: TerminalRunStyle,
}

pub(super) struct TerminalRow {
    pub(super) text: String,
    pub(super) runs: Vec<TerminalRun>,
}

pub(super) struct TerminalSnapshot {
    pub(super) rows: Vec<TerminalRow>,
    pub(super) cursor: Option<((usize, usize), CursorPaint)>,
    pub(super) cursor_position: Option<(usize, usize)>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum CursorPaint {
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

    pub fn set_font(&mut self, family: impl Into<SharedString>, size: f32, cx: &mut Context<Self>) {
        self.configuration.font_family = family.into();
        self.configuration.font_size = size;
        self.measured_cell_width = None;
        if let Some(session) = &self.session {
            session.dirty.store(true, Ordering::Release);
        }
        cx.notify();
    }

    pub fn set_labels(&mut self, labels: TerminalLabels, cx: &mut Context<Self>) {
        self.configuration.labels = labels;
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
        let cell_height = terminal_cell_height(self.configuration.font_size);
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
        let cell_height = terminal_cell_height(self.configuration.font_size);
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
                f32::from(delta.y) / terminal_cell_height(self.configuration.font_size)
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
        let palette =
            TerminalPalette::resolve(self.configuration.color_scheme, Theme::global(cx).clone());
        let terminal_font = font(self.configuration.font_family.clone());
        let font_size = self.configuration.font_size;
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
                        .font_family(self.configuration.font_family.clone())
                        .text_size(px(font_size))
                        .line_height(px(cell_height))
                        .text_color(palette.foreground)
                        .whitespace_nowrap()
                        .underline()
                        .child(SharedString::from(self.marked_text.clone())),
                );
            }
        } else {
            let message = self
                .error
                .clone()
                .unwrap_or_else(|| self.configuration.labels.starting.to_string());
            screen = screen.child(
                div()
                    .p(px(12.0))
                    .font_family(self.configuration.font_family.clone())
                    .text_size(px(font_size))
                    .line_height(px(cell_height))
                    .text_color(if self.error.is_some() {
                        Theme::global(cx).danger
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
                    .child(self.configuration.labels.process_exited.clone()),
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
            .aria_label(self.configuration.labels.accessibility_label.clone())
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

pub(super) fn terminal_config(cursor_style: TerminalCursorStyle) -> Config {
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

pub(super) fn cursor_paint(shape: AlacrittyCursorShape, focused: bool) -> Option<CursorPaint> {
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
