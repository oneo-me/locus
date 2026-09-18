use rust_i18n::t;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config::{
    DEFAULT_CODE_FONT_FAMILY, DEFAULT_CODE_FONT_SIZE, DEFAULT_LIBRARY_PATH, DEFAULT_MCP_ENABLED,
    DEFAULT_MCP_LOCAL_NETWORK, DEFAULT_MCP_PORT, DEFAULT_UI_FONT_FAMILY,
};
use crate::storage::settings_store;
use oneo_gpui::{AppearancePreferences, FONT_SIZES, Language};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TerminalPreference {
    #[default]
    System,
    Ghostty,
}

impl TerminalPreference {
    pub const ALL: &'static [Self] = if cfg!(target_os = "windows") {
        &[Self::System]
    } else {
        &[Self::System, Self::Ghostty]
    };

    pub fn label(self, language: Language) -> String {
        match self {
            Self::System => t!("settings.terminal.system", locale = language.locale()).into_owned(),
            Self::Ghostty => "Ghostty".to_owned(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentPreference {
    #[default]
    Codex,
    Pi,
    OpenCode,
    ClaudeCode,
}

impl AgentPreference {
    pub const ALL: [Self; 4] = [Self::Codex, Self::Pi, Self::OpenCode, Self::ClaudeCode];

    pub fn label(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::Pi => "pi",
            Self::OpenCode => "OpenCode",
            Self::ClaudeCode => "Claude Code",
        }
    }

    pub fn command(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Pi => "pi",
            Self::OpenCode => "opencode",
            Self::ClaudeCode => "claude",
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
    #[serde(flatten)]
    pub appearance: AppearancePreferences,
    pub ui_font_family: String,
    pub code_font_family: String,
    pub code_font_size: f32,
    pub terminal: TerminalPreference,
    pub agent: AgentPreference,
    pub mcp_enabled: bool,
    pub mcp_local_network: bool,
    pub mcp_port: u16,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            library_path: DEFAULT_LIBRARY_PATH.map(PathBuf::from),
            editor: EditorPreference::System,
            appearance: AppearancePreferences::default(),
            ui_font_family: DEFAULT_UI_FONT_FAMILY.to_owned(),
            code_font_family: DEFAULT_CODE_FONT_FAMILY.to_owned(),
            code_font_size: DEFAULT_CODE_FONT_SIZE,
            terminal: TerminalPreference::default(),
            agent: AgentPreference::default(),
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
        if self.mcp_port == 0 {
            self.mcp_port = defaults.mcp_port;
        }
        normalize_font_family(&mut self.ui_font_family, &defaults.ui_font_family);
        normalize_font_family(&mut self.code_font_family, &defaults.code_font_family);
    }

    pub fn resolved_ui_font_family(&self) -> &str {
        resolved_font_family(&self.ui_font_family, DEFAULT_UI_FONT_FAMILY)
    }

    pub fn resolved_code_font_family(&self) -> &str {
        resolved_font_family(&self.code_font_family, DEFAULT_CODE_FONT_FAMILY)
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
            appearance: AppearancePreferences {
                theme: ThemePreference::Dark,
                language: Language::English,
                ui_font_size: 18.0,
            },
            ui_font_family: "Avenir Next".to_owned(),
            code_font_family: "JetBrains Mono".to_owned(),
            code_font_size: 15.0,
            terminal: TerminalPreference::Ghostty,
            agent: AgentPreference::ClaudeCode,
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
        assert_eq!(restored.terminal, preferences.terminal);
        assert_eq!(restored.agent, preferences.agent);
        assert_eq!(restored.mcp_enabled, preferences.mcp_enabled);
        assert_eq!(restored.mcp_local_network, preferences.mcp_local_network);
        assert_eq!(restored.mcp_port, preferences.mcp_port);
    }

    #[test]
    fn removed_terminal_settings_do_not_reset_other_preferences() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.toml");
        std::fs::write(
            &path,
            r#"
library_path = "/tmp/notes"
theme = "dark"
mcp_enabled = true
terminal_shell = "/bin/bash"
terminal_command = "old-command"
terminal_font_family = "Old Font"
terminal_font_size = 18.0
terminal_color_scheme = "dark"
terminal_cursor_style = "bar"
auto_open_canvas = false
"#,
        )
        .unwrap();
        let store = oneo_gpui::SettingsStore::at_path(path.clone()).unwrap();
        let preferences: Preferences = store.load().unwrap();
        assert_eq!(preferences.library_path, Some(PathBuf::from("/tmp/notes")));
        assert_eq!(preferences.appearance.theme, ThemePreference::Dark);
        assert!(preferences.mcp_enabled);
        assert_eq!(preferences.terminal, TerminalPreference::System);
        assert_eq!(preferences.agent, AgentPreference::Codex);
        store.save(&preferences).unwrap();
        let saved = std::fs::read_to_string(path).unwrap();
        assert!(!saved.contains("terminal_"));
        assert!(!saved.contains("auto_open_canvas"));
        assert!(saved.contains("mcp_enabled = true"));
    }

    #[test]
    fn removed_export_setting_does_not_affect_other_preferences() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.toml");
        std::fs::write(
            &path,
            "export_directory = '/tmp/exports'\nlibrary_path = '/tmp/notes'\nmcp_enabled = true\n",
        )
        .unwrap();
        let store = oneo_gpui::SettingsStore::at_path(path.clone()).unwrap();
        let preferences: Preferences = store.load().unwrap();
        assert_eq!(preferences.library_path, Some(PathBuf::from("/tmp/notes")));
        assert!(preferences.mcp_enabled);
        store.save(&preferences).unwrap();
        let saved = std::fs::read_to_string(path).unwrap();
        assert!(!saved.contains("export_directory"));
        let restored: Preferences = store.load().unwrap();
        assert_eq!(restored.library_path, preferences.library_path);
        assert!(restored.mcp_enabled);
    }

    #[test]
    fn agents_use_their_installed_executable_names() {
        assert_eq!(
            AgentPreference::ALL.map(AgentPreference::command),
            ["codex", "pi", "opencode", "claude"]
        );
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
        assert_eq!(preferences.mcp_port, DEFAULT_MCP_PORT);
    }
}
