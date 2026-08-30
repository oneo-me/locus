use super::*;

fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}

pub(super) enum TerminalUiEvent {
    ClipboardStore(String),
    ClipboardLoad(Arc<dyn Fn(&str) -> String + Send + Sync>),
    Exited,
}

#[derive(Clone)]
pub(super) struct TerminalEventProxy {
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

pub(super) struct TerminalDimensions {
    pub(super) columns: usize,
    pub(super) rows: usize,
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

pub(super) struct TerminalSession {
    pub(super) term: Arc<FairMutex<Term<TerminalEventProxy>>>,
    sender: EventLoopSender,
    pub(super) dirty: Arc<AtomicBool>,
    pub(super) ui_events: Receiver<TerminalUiEvent>,
    window_size: Arc<Mutex<WindowSize>>,
    pub(super) grid_size: (usize, usize),
    dark_palette: Arc<AtomicBool>,
}

impl TerminalSession {
    pub(super) fn new(
        working_directory: &Path,
        configuration: &TerminalConfiguration,
        columns: usize,
        rows: usize,
    ) -> Result<Self, String> {
        let columns = columns.max(MIN_COLUMNS);
        let rows = rows.max(MIN_ROWS);
        let cell_height = terminal_cell_height(configuration.font_size);
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

        let pty = tty::new(&options, window_size, 0)
            .map_err(|error| format!("{}: {error}", configuration.labels.could_not_start))?;
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

    pub(super) fn write(&self, bytes: impl Into<Cow<'static, [u8]>>) {
        let bytes = bytes.into();
        if !bytes.is_empty() {
            let _ = self.sender.send(Msg::Input(bytes));
        }
    }

    pub(super) fn resize(
        &mut self,
        columns: usize,
        rows: usize,
        cell_width: f32,
        cell_height: f32,
    ) {
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

    pub(super) fn mode(&self) -> TermMode {
        *self.term.lock().mode()
    }

    pub(super) fn scroll(&self, lines: i32) {
        if lines != 0 {
            self.term.lock().scroll_display(Scroll::Delta(lines));
            self.dirty.store(true, Ordering::Release);
        }
    }

    pub(super) fn set_cursor_style(&self, cursor_style: TerminalCursorStyle) {
        self.term.lock().set_options(terminal_config(cursor_style));
        self.dirty.store(true, Ordering::Release);
    }

    pub(super) fn snapshot(&self, palette: TerminalPalette, focused: bool) -> TerminalSnapshot {
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
