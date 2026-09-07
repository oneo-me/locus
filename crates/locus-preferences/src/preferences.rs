use rust_i18n::t;
use std::path::PathBuf;

pub use oneo_gpui_terminal::{TerminalColorScheme, TerminalCursorStyle};
use serde::{Deserialize, Serialize};

use crate::config::{
    DEFAULT_CODE_FONT_FAMILY, DEFAULT_CODE_FONT_SIZE, DEFAULT_LIBRARY_PATH, DEFAULT_MCP_ENABLED,
    DEFAULT_MCP_LOCAL_NETWORK, DEFAULT_MCP_PORT, DEFAULT_TERMINAL_COMMAND,
    DEFAULT_TERMINAL_FONT_FAMILY, DEFAULT_TERMINAL_FONT_SIZE, DEFAULT_TERMINAL_SHELL,
    DEFAULT_UI_FONT_FAMILY,
};
use crate::storage::settings_store;
use oneo_gpui::{AppearancePreferences, FONT_SIZES, Language};

pub fn terminal_color_scheme_label(value: TerminalColorScheme, language: Language) -> String {
    match value {
        TerminalColorScheme::Auto => {
            t!("terminal.color_scheme.auto", locale = language.locale()).into_owned()
        }
        TerminalColorScheme::Light => {
            t!("terminal.color_scheme.light", locale = language.locale()).into_owned()
        }
        TerminalColorScheme::Dark => {
            t!("terminal.color_scheme.dark", locale = language.locale()).into_owned()
        }
    }
}

pub fn terminal_cursor_style_label(value: TerminalCursorStyle, language: Language) -> String {
    match value {
        TerminalCursorStyle::Block => {
            t!("terminal.cursor_style.block", locale = language.locale()).into_owned()
        }
        TerminalCursorStyle::Underline => t!(
            "terminal.cursor_style.underline",
            locale = language.locale()
        )
        .into_owned(),
        TerminalCursorStyle::Bar => {
            t!("terminal.cursor_style.bar", locale = language.locale()).into_owned()
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EditorPreference {
    System,
    VisualStudioCode,
}

impl EditorPreference {
    pub const ALL: [Self; 2] = [Self::System, Self::VisualStudioCode];

    pub fn label(self, language: Language) -> String {
        match self {
            Self::System => {
                t!("settings.editor.system_default", locale = language.locale()).into_owned()
            }
            Self::VisualStudioCode => "VS Code".to_owned(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub library_path: Option<PathBuf>,
    pub editor: EditorPreference,
    pub auto_open_canvas: bool,
    #[serde(flatten)]
    pub appearance: AppearancePreferences,
    pub ui_font_family: String,
    pub code_font_family: String,
    pub code_font_size: f32,
    pub terminal_font_family: String,
    pub terminal_font_size: f32,
    pub terminal_shell: String,
    pub terminal_command: String,
    pub terminal_color_scheme: TerminalColorScheme,
    pub terminal_cursor_style: TerminalCursorStyle,
    pub mcp_enabled: bool,
    pub mcp_local_network: bool,
    pub mcp_port: u16,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            library_path: DEFAULT_LIBRARY_PATH.map(PathBuf::from),
            editor: EditorPreference::System,
            auto_open_canvas: false,
            appearance: AppearancePreferences::default(),
            ui_font_family: DEFAULT_UI_FONT_FAMILY.to_owned(),
            code_font_family: DEFAULT_CODE_FONT_FAMILY.to_owned(),
            code_font_size: DEFAULT_CODE_FONT_SIZE,
            terminal_font_family: DEFAULT_TERMINAL_FONT_FAMILY.to_owned(),
            terminal_font_size: DEFAULT_TERMINAL_FONT_SIZE,
            terminal_shell: DEFAULT_TERMINAL_SHELL.to_owned(),
            terminal_command: DEFAULT_TERMINAL_COMMAND.to_owned(),
            terminal_color_scheme: TerminalColorScheme::Auto,
            terminal_cursor_style: TerminalCursorStyle::Block,
            mcp_enabled: DEFAULT_MCP_ENABLED,
            mcp_local_network: DEFAULT_MCP_LOCAL_NETWORK,
            mcp_port: DEFAULT_MCP_PORT,
        }
    }
}

impl Preferences {
    pub fn load() -> (Self, Option<String>) {
        match settings_store()
            .and_then(|store| store.load::<Self>().map_err(|error| format!("{error:#}")))
        {
            Ok(mut preferences) => {
                preferences.normalize();
                (preferences, None)
            }
            Err(error) => (Self::default(), Some(format!("无法读取设置：{error}"))),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        settings_store()?
            .save(self)
            .map_err(|error| format!("{error:#}"))
    }

    fn normalize(&mut self) {
        let defaults = Self::default();
        self.appearance.normalize();
        if !FONT_SIZES.contains(&self.code_font_size) {
            self.code_font_size = defaults.code_font_size;
        }
        if !FONT_SIZES.contains(&self.terminal_font_size) {
            self.terminal_font_size = defaults.terminal_font_size;
        }
        if self.mcp_port == 0 {
            self.mcp_port = defaults.mcp_port;
        }
        normalize_font_family(&mut self.ui_font_family, &defaults.ui_font_family);
        normalize_font_family(&mut self.code_font_family, &defaults.code_font_family);
        normalize_font_family(
            &mut self.terminal_font_family,
            &defaults.terminal_font_family,
        );
    }

    pub fn resolved_ui_font_family(&self) -> &str {
        resolved_font_family(&self.ui_font_family, DEFAULT_UI_FONT_FAMILY)
    }

    pub fn resolved_code_font_family(&self) -> &str {
        resolved_font_family(&self.code_font_family, DEFAULT_CODE_FONT_FAMILY)
    }

    pub fn resolved_terminal_font_family(&self) -> &str {
        resolved_font_family(&self.terminal_font_family, DEFAULT_TERMINAL_FONT_FAMILY)
    }
}

fn normalize_font_family(family: &mut String, default: &str) {
    *family = family.trim().to_owned();
    if family.is_empty() {
        *family = default.to_owned();
    }
}

fn resolved_font_family<'a>(family: &'a str, default: &'static str) -> &'a str {
    let family = family.trim();
    if family.is_empty() { default } else { family }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oneo_gpui::ThemePreference;

    #[test]
    fn preferences_round_trip() {
        let preferences = Preferences {
            library_path: Some(PathBuf::from("/tmp/笔记")),
            editor: EditorPreference::VisualStudioCode,
            auto_open_canvas: true,
            appearance: AppearancePreferences {
                theme: ThemePreference::Dark,
                language: Language::English,
                ui_font_size: 18.0,
            },
            ui_font_family: "Avenir Next".to_owned(),
            code_font_family: "JetBrains Mono".to_owned(),
            code_font_size: 15.0,
            terminal_font_family: "SF Mono".to_owned(),
            terminal_font_size: 16.0,
            terminal_shell: "/bin/zsh".to_owned(),
            terminal_command: "git status".to_owned(),
            terminal_color_scheme: TerminalColorScheme::Dark,
            terminal_cursor_style: TerminalCursorStyle::Bar,
            mcp_enabled: true,
            mcp_local_network: true,
            mcp_port: 43123,
        };
        let directory = tempfile::tempdir().unwrap();
        let store =
            oneo_gpui::SettingsStore::at_path(directory.path().join("settings.toml")).unwrap();
        store.save(&preferences).unwrap();
        let restored: Preferences = store.load().unwrap();

        assert_eq!(restored.library_path, preferences.library_path);
        assert_eq!(restored.editor, preferences.editor);
        assert!(restored.auto_open_canvas);
        assert_eq!(restored.appearance.theme, preferences.appearance.theme);
        assert_eq!(
            restored.appearance.language,
            preferences.appearance.language
        );
        assert_eq!(restored.ui_font_family, preferences.ui_font_family);
        assert_eq!(
            restored.appearance.ui_font_size,
            preferences.appearance.ui_font_size
        );
        assert_eq!(restored.code_font_family, preferences.code_font_family);
        assert_eq!(restored.code_font_size, preferences.code_font_size);
        assert_eq!(
            restored.terminal_font_family,
            preferences.terminal_font_family
        );
        assert_eq!(restored.terminal_font_size, preferences.terminal_font_size);
        assert_eq!(restored.terminal_shell, preferences.terminal_shell);
        assert_eq!(restored.terminal_command, preferences.terminal_command);
        assert_eq!(
            restored.terminal_color_scheme,
            preferences.terminal_color_scheme
        );
        assert_eq!(
            restored.terminal_cursor_style,
            preferences.terminal_cursor_style
        );
        assert_eq!(restored.mcp_enabled, preferences.mcp_enabled);
        assert_eq!(restored.mcp_local_network, preferences.mcp_local_network);
        assert_eq!(restored.mcp_port, preferences.mcp_port);
    }

    #[test]
    fn flat_appearance_fields_use_shared_defaults_and_explicit_choices() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.toml");
        let store = oneo_gpui::SettingsStore::at_path(path.clone()).unwrap();
        std::fs::write(&path, "mcp_enabled = true\n").unwrap();
        let preferences: Preferences = store.load().unwrap();
        assert_eq!(preferences.appearance, AppearancePreferences::default());
        assert!(preferences.mcp_enabled);
        assert!(!preferences.auto_open_canvas);
        std::fs::write(
            &path,
            r#"theme = "dark"
language = "chinese"
ui_font_size = 18.0
"#,
        )
        .unwrap();
        let preferences: Preferences = store.load().unwrap();
        assert_eq!(preferences.appearance.theme, ThemePreference::Dark);
        assert_eq!(preferences.appearance.language, Language::Chinese);
        assert_eq!(preferences.appearance.ui_font_size, 18.0);
        store.save(&preferences).unwrap();
        let saved = std::fs::read_to_string(path).unwrap();
        assert!(!saved.contains("[appearance]"));
    }

    #[test]
    fn unsupported_font_sizes_return_to_defaults() {
        let mut preferences = Preferences {
            ui_font_family: " ".to_owned(),
            appearance: AppearancePreferences {
                ui_font_size: f32::NAN,
                ..AppearancePreferences::default()
            },
            code_font_family: "  ".to_owned(),
            code_font_size: 99.0,
            terminal_font_family: "\t".to_owned(),
            terminal_font_size: -1.0,
            mcp_port: 0,
            ..Preferences::default()
        };

        preferences.normalize();

        assert_eq!(preferences.ui_font_family, DEFAULT_UI_FONT_FAMILY);
        assert_eq!(
            preferences.appearance.ui_font_size,
            AppearancePreferences::default().ui_font_size
        );
        assert_eq!(preferences.code_font_family, DEFAULT_CODE_FONT_FAMILY);
        assert_eq!(preferences.code_font_size, DEFAULT_CODE_FONT_SIZE);
        assert_eq!(
            preferences.terminal_font_family,
            DEFAULT_TERMINAL_FONT_FAMILY
        );
        assert_eq!(preferences.terminal_font_size, DEFAULT_TERMINAL_FONT_SIZE);
        assert_eq!(preferences.mcp_port, DEFAULT_MCP_PORT);
    }
}
