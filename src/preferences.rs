use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config::{
    DEFAULT_CODE_FONT_FAMILY, DEFAULT_CODE_FONT_SIZE, DEFAULT_LANGUAGE, DEFAULT_LIBRARY_PATH,
    DEFAULT_MCP_ENABLED, DEFAULT_MCP_LOCAL_NETWORK, DEFAULT_MCP_PORT, DEFAULT_TERMINAL_COMMAND,
    DEFAULT_TERMINAL_SHELL, DEFAULT_THEME, DEFAULT_UI_FONT_SIZE, SETTINGS_FILE,
};
use crate::localization::Language;
use crate::storage::data_directory;
use crate::theme::ThemePreference;

pub const FONT_SIZES: [f32; 6] = [14.0, 15.0, 16.0, 17.0, 18.0, 20.0];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TerminalColorScheme {
    #[default]
    Auto,
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TerminalCursorStyle {
    #[default]
    Block,
    Underline,
    Bar,
}

impl TerminalCursorStyle {
    pub const ALL: [Self; 3] = [Self::Block, Self::Underline, Self::Bar];

    pub fn label(self, language: Language) -> &'static str {
        match self {
            Self::Block => language.text("块状", "Block"),
            Self::Underline => language.text("下划线", "Underline"),
            Self::Bar => language.text("竖线", "Bar"),
        }
    }
}

impl TerminalColorScheme {
    pub const ALL: [Self; 3] = [Self::Auto, Self::Light, Self::Dark];

    pub fn label(self, language: Language) -> &'static str {
        match self {
            Self::Auto => language.text("自动", "Auto"),
            Self::Light => language.text("亮色", "Light"),
            Self::Dark => language.text("暗色", "Dark"),
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

    pub fn label(self, language: Language) -> &'static str {
        match self {
            Self::System => language.text("系统默认", "System default"),
            Self::VisualStudioCode => "VS Code",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub library_path: Option<PathBuf>,
    pub editor: EditorPreference,
    pub theme: ThemePreference,
    pub language: Language,
    pub ui_font_size: f32,
    pub code_font_family: String,
    pub code_font_size: f32,
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
            theme: DEFAULT_THEME,
            language: DEFAULT_LANGUAGE,
            ui_font_size: DEFAULT_UI_FONT_SIZE,
            code_font_family: DEFAULT_CODE_FONT_FAMILY.to_owned(),
            code_font_size: DEFAULT_CODE_FONT_SIZE,
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
        let path = match settings_path() {
            Ok(path) => path,
            Err(error) => return (Self::default(), Some(error)),
        };
        if !path.exists() {
            return (Self::default(), None);
        }

        match fs::read(&path)
            .map_err(|error| error.to_string())
            .and_then(|bytes| {
                serde_json::from_slice::<Self>(&bytes).map_err(|error| error.to_string())
            }) {
            Ok(mut preferences) => {
                preferences.normalize();
                (preferences, None)
            }
            Err(error) => (Self::default(), Some(format!("无法读取设置：{error}"))),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path = settings_path()?;
        let directory = path.parent().ok_or_else(|| "设置路径无效".to_string())?;
        fs::create_dir_all(directory).map_err(|error| error.to_string())?;
        let data = serde_json::to_vec_pretty(self).map_err(|error| error.to_string())?;
        fs::write(path, data).map_err(|error| error.to_string())
    }

    fn normalize(&mut self) {
        let defaults = Self::default();
        if !FONT_SIZES.contains(&self.ui_font_size) {
            self.ui_font_size = defaults.ui_font_size;
        }
        if !FONT_SIZES.contains(&self.code_font_size) {
            self.code_font_size = defaults.code_font_size;
        }
        if self.mcp_port == 0 {
            self.mcp_port = defaults.mcp_port;
        }
        self.code_font_family = self.code_font_family.trim().to_owned();
        if self.code_font_family.is_empty() {
            self.code_font_family = defaults.code_font_family;
        }
    }

    pub fn resolved_code_font_family(&self) -> &str {
        let family = self.code_font_family.trim();
        if family.is_empty() {
            DEFAULT_CODE_FONT_FAMILY
        } else {
            family
        }
    }
}

fn settings_path() -> Result<PathBuf, String> {
    data_directory().map(|directory| directory.join(SETTINGS_FILE))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferences_round_trip() {
        let preferences = Preferences {
            library_path: Some(PathBuf::from("/tmp/笔记")),
            editor: EditorPreference::VisualStudioCode,
            theme: ThemePreference::Dark,
            language: Language::English,
            ui_font_size: 18.0,
            code_font_family: "JetBrains Mono".to_owned(),
            code_font_size: 15.0,
            terminal_shell: "/bin/zsh".to_owned(),
            terminal_command: "git status".to_owned(),
            terminal_color_scheme: TerminalColorScheme::Dark,
            terminal_cursor_style: TerminalCursorStyle::Bar,
            mcp_enabled: true,
            mcp_local_network: true,
            mcp_port: 43123,
        };
        let json = serde_json::to_string(&preferences).unwrap();
        let restored: Preferences = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.library_path, preferences.library_path);
        assert_eq!(restored.editor, preferences.editor);
        assert_eq!(restored.theme, preferences.theme);
        assert_eq!(restored.language, preferences.language);
        assert_eq!(restored.ui_font_size, preferences.ui_font_size);
        assert_eq!(restored.code_font_family, preferences.code_font_family);
        assert_eq!(restored.code_font_size, preferences.code_font_size);
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
    fn unsupported_font_sizes_return_to_defaults() {
        let mut preferences = Preferences {
            ui_font_size: f32::NAN,
            code_font_family: "  ".to_owned(),
            code_font_size: 99.0,
            mcp_port: 0,
            ..Preferences::default()
        };

        preferences.normalize();

        assert_eq!(preferences.ui_font_size, DEFAULT_UI_FONT_SIZE);
        assert_eq!(preferences.code_font_family, DEFAULT_CODE_FONT_FAMILY);
        assert_eq!(preferences.code_font_size, DEFAULT_CODE_FONT_SIZE);
        assert_eq!(preferences.mcp_port, DEFAULT_MCP_PORT);
    }
}
