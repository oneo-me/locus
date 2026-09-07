rust_i18n::i18n!("locales", fallback = "en");

pub mod config;
mod preferences;
pub mod storage;

pub use preferences::{
    EditorPreference, Preferences, TerminalColorScheme, TerminalCursorStyle,
    terminal_color_scheme_label, terminal_cursor_style_label,
};

#[cfg(test)]
mod localization_tests;
