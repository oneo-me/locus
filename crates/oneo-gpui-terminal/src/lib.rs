use std::borrow::Cow;
use std::cell::Cell;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

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
    App, Bounds, ClipboardItem, Context, ElementInputHandler, EntityInputHandler, EventEmitter,
    FocusHandle, Focusable, FontStyle, FontWeight, Hsla, IntoElement, KeyDownEvent, Keystroke,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, Render, Role,
    ScrollDelta, ScrollWheelEvent, SharedString, StrikethroughStyle, StyledText, TextRun,
    UTF16Selection, UnderlineStyle, Window, canvas, div, font, point, prelude::*, px, rgb, size,
};

use gpui_component::Theme;

mod color;
mod config;
mod input;
mod platform;
mod process;
mod session;
mod view;

use color::*;
pub use config::{TerminalColorScheme, TerminalConfiguration, TerminalCursorStyle, TerminalLabels};
use input::*;
use platform::*;
use process::*;
#[cfg(test)]
use session::{TerminalDimensions, TerminalEventProxy};
use session::{TerminalSession, TerminalUiEvent};
pub use view::TerminalView;
use view::{
    CursorPaint, TerminalCell, TerminalPalette, TerminalRow, TerminalRun, TerminalRunStyle,
    TerminalSnapshot, cursor_paint, terminal_cell_height, terminal_cell_text, terminal_config,
};
#[cfg(test)]
use view::{TerminalTitleState, marked_text_range, normalized_program_title};

const CELL_WIDTH_FALLBACK: f32 = 7.8;
const CELL_HEIGHT_PADDING: f32 = 5.0;
const PADDING_X: f32 = 10.0;
const PADDING_Y: f32 = 8.0;
const MIN_COLUMNS: usize = 20;
const MIN_ROWS: usize = 8;
const SCROLLBACK_LINES: usize = 10_000;

#[derive(Clone, Copy)]
pub enum TerminalEvent {
    TitleChanged,
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
    fn program_title_takes_precedence_and_is_sanitized() {
        assert_eq!(
            normalized_program_title(Some("\0π - locus\n".to_owned())),
            Some("π - locus".to_owned())
        );
        assert_eq!(normalized_program_title(Some("\n\0".to_owned())), None);
        let mut title = TerminalTitleState::default();
        assert!(title.update_active_process("pi".to_owned()));
        assert_eq!(title.current(), Some("pi"));
        assert!(title.update_program_title("pi".to_owned(), Some("π - locus".to_owned())));
        assert_eq!(title.current(), Some("π - locus"));
        assert!(title.update_active_process("zsh".to_owned()));
        assert_eq!(title.current(), Some("zsh"));
    }

    #[test]
    fn terminal_title_events_reach_the_ui() {
        let (proxy, ui_events) = TerminalEventProxy::for_test();

        proxy.send_event(Event::Title("π - locus".to_owned()));
        assert!(matches!(
            ui_events.try_recv(),
            Ok(TerminalUiEvent::TitleChanged(Some(title))) if title == "π - locus"
        ));

        proxy.send_event(Event::ResetTitle);
        assert!(matches!(
            ui_events.try_recv(),
            Ok(TerminalUiEvent::TitleChanged(None))
        ));
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
            labels: TerminalLabels {
                accessibility_label: "test terminal".into(),
                could_not_start: "test startup failure".into(),
                starting: "test starting".into(),
                process_exited: "test process exit".into(),
            },
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

    #[cfg(unix)]
    #[test]
    fn active_process_name_follows_the_foreground_job() {
        let configuration = TerminalConfiguration {
            shell: "/bin/sh".to_owned(),
            startup_command: String::new(),
            color_scheme: TerminalColorScheme::Auto,
            cursor_style: TerminalCursorStyle::Block,
            font_family: "monospace".into(),
            font_size: 14.0,
            labels: TerminalLabels {
                accessibility_label: "test terminal".into(),
                could_not_start: "test startup failure".into(),
                starting: "test starting".into(),
                process_exited: "test process exit".into(),
            },
        };
        let session = TerminalSession::new(Path::new("/tmp"), &configuration, 80, 24).unwrap();

        assert_process_name_eventually(&session, "sh");
        session.write(b"sleep 1\r".to_vec());
        assert_process_name_eventually(&session, "sleep");
        session.write(b"\x03exit\r".to_vec());
    }

    #[cfg(unix)]
    fn assert_process_name_eventually(session: &TerminalSession, expected: &str) {
        for _ in 0..100 {
            if session.active_process_name() == expected {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!(
            "PTY did not report foreground process {expected:?}; last name: {:?}",
            session.active_process_name()
        );
    }
}
