rust_i18n::i18n!("locales", fallback = "en");

pub mod config;
pub mod localization;
mod platform;
mod preferences;
pub mod storage;
pub mod theme;

pub use preferences::{
    EditorPreference, FONT_SIZES, Preferences, TerminalColorScheme, TerminalCursorStyle,
    terminal_color_scheme_label, terminal_cursor_style_label,
};

#[cfg(test)]
mod localization_tests;
