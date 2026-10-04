rust_i18n::i18n!("locales", fallback = "en");

pub mod config;
mod preferences;
pub mod storage;

pub use preferences::{
    AgentPreference, EditorPreference, Preferences, ShellPreference, SkillsLocation,
    TerminalPreference,
};

#[cfg(test)]
mod localization_tests;
