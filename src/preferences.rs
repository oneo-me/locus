use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config::{
    DEFAULT_CODE_FONT_SIZE, DEFAULT_LANGUAGE, DEFAULT_LIBRARY_PATH, DEFAULT_THEME,
    DEFAULT_UI_FONT_SIZE, SETTINGS_FILE,
};
use crate::localization::Language;
use crate::storage::data_directory;
use crate::theme::ThemePreference;

pub const FONT_SIZES: [f32; 6] = [14.0, 15.0, 16.0, 17.0, 18.0, 20.0];

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
    pub code_font_size: f32,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            library_path: DEFAULT_LIBRARY_PATH.map(PathBuf::from),
            editor: EditorPreference::System,
            theme: DEFAULT_THEME,
            language: DEFAULT_LANGUAGE,
            ui_font_size: DEFAULT_UI_FONT_SIZE,
            code_font_size: DEFAULT_CODE_FONT_SIZE,
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
            code_font_size: 15.0,
        };
        let json = serde_json::to_string(&preferences).unwrap();
        let restored: Preferences = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.library_path, preferences.library_path);
        assert_eq!(restored.editor, preferences.editor);
        assert_eq!(restored.theme, preferences.theme);
        assert_eq!(restored.language, preferences.language);
        assert_eq!(restored.ui_font_size, preferences.ui_font_size);
        assert_eq!(restored.code_font_size, preferences.code_font_size);
    }

    #[test]
    fn unsupported_font_sizes_return_to_defaults() {
        let mut preferences = Preferences {
            ui_font_size: f32::NAN,
            code_font_size: 99.0,
            ..Preferences::default()
        };

        preferences.normalize();

        assert_eq!(preferences.ui_font_size, DEFAULT_UI_FONT_SIZE);
        assert_eq!(preferences.code_font_size, DEFAULT_CODE_FONT_SIZE);
    }
}
