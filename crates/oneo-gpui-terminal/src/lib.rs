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

use oneo_gpui_ui::Theme;

mod color;
mod config;
mod input;
mod platform;
mod session;
mod view;

use color::*;
pub use config::{TerminalColorScheme, TerminalConfiguration, TerminalCursorStyle, TerminalLabels};
use input::*;
use platform::*;
#[cfg(test)]
use session::TerminalDimensions;
use session::{TerminalSession, TerminalUiEvent};
pub use view::TerminalView;
#[cfg(test)]
use view::marked_text_range;
use view::{
    CursorPaint, TerminalCell, TerminalPalette, TerminalRow, TerminalRun, TerminalRunStyle,
    TerminalSnapshot, cursor_paint, terminal_cell_height, terminal_cell_text, terminal_config,
};

const CELL_WIDTH_FALLBACK: f32 = 7.8;
const CELL_HEIGHT_PADDING: f32 = 5.0;
const PADDING_X: f32 = 10.0;
const PADDING_Y: f32 = 8.0;
const MIN_COLUMNS: usize = 20;
const MIN_ROWS: usize = 8;
const SCROLLBACK_LINES: usize = 10_000;

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
            font_family: "monospace".into(),
            font_size: 14.0,
            labels: TerminalLabels::default(),
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
