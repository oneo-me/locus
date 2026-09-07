#[cfg(feature = "shell")]
mod controls;
#[cfg(feature = "shell")]
mod platform;
#[cfg(feature = "shell")]
pub mod theme;

#[cfg(feature = "shell")]
pub use controls::AppearanceControls;
#[cfg(feature = "shell")]
use gpui_kit::{App, Context, Subscription, Window};
#[cfg(feature = "shell")]
use rust_i18n::t;
use serde::{Deserialize, Serialize};
#[cfg(feature = "shell")]
use std::sync::OnceLock;

pub const FONT_SIZES: [f32; 7] = [12.0, 14.0, 15.0, 16.0, 17.0, 18.0, 20.0];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Language {
    #[default]
    System,
    Chinese,
    English,
}

impl Language {
    pub const ALL: [Self; 3] = [Self::System, Self::Chinese, Self::English];

    #[cfg(feature = "shell")]
    pub fn label(self, language: Self) -> String {
        let key = match self {
            Self::System => "language.system",
            Self::Chinese => "language.chinese",
            Self::English => "language.english",
        };
        t!(key, locale = language.locale()).into_owned()
    }

    #[cfg(feature = "shell")]
    pub fn locale(self) -> &'static str {
        static SYSTEM: OnceLock<&'static str> = OnceLock::new();
        match self {
            Self::System => {
                SYSTEM.get_or_init(|| system_locale(sys_locale::get_locale().as_deref()))
            }
            Self::Chinese => "zh-CN",
            Self::English => "en",
        }
    }

    #[cfg(feature = "shell")]
    pub fn activate(self) {
        gpui_kit::component::set_locale(self.locale());
    }
}

#[cfg(feature = "shell")]
fn system_locale(locale: Option<&str>) -> &'static str {
    if locale
        .and_then(|locale| locale.split(['-', '_']).next())
        .is_some_and(|language| language.eq_ignore_ascii_case("zh"))
    {
        "zh-CN"
    } else {
        "en"
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearancePreferences {
    pub theme: ThemePreference,
    pub language: Language,
    pub ui_font_size: f32,
}

impl Default for AppearancePreferences {
    fn default() -> Self {
        Self {
            theme: ThemePreference::System,
            language: Language::System,
            ui_font_size: 16.0,
        }
    }
}

impl AppearancePreferences {
    pub fn normalize(&mut self) {
        if !FONT_SIZES.contains(&self.ui_font_size) {
            self.ui_font_size = Self::default().ui_font_size;
        }
    }

    #[cfg(feature = "shell")]
    pub fn apply(self, window: &mut Window, cx: &mut App) {
        self.language.activate();
        theme::apply_preference(self.theme, self.ui_font_size, window, cx);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AppearanceChange {
    Theme(ThemePreference),
    Language(Language),
    FontSize(f32),
}

impl AppearanceChange {
    pub fn update(self, preferences: &mut AppearancePreferences) -> bool {
        let previous = *preferences;
        match self {
            Self::Theme(value) => preferences.theme = value,
            Self::Language(value) => preferences.language = value,
            Self::FontSize(value) => preferences.ui_font_size = value,
        }
        preferences.normalize();
        *preferences != previous
    }

    #[cfg(feature = "shell")]
    pub fn apply(
        self,
        preferences: &mut AppearancePreferences,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        if !self.update(preferences) {
            return false;
        }
        preferences.apply(window, cx);
        true
    }
}

#[cfg(feature = "shell")]
pub fn observe_appearance<T: 'static>(
    window: &mut Window,
    cx: &mut Context<T>,
    preferences: fn(&T, &App) -> AppearancePreferences,
) -> Subscription {
    cx.observe_window_appearance(window, move |this, window, cx| {
        let appearance = preferences(this, cx);
        if appearance.theme == ThemePreference::System {
            appearance.apply(window, cx);
            cx.notify();
        }
    })
}

#[cfg(feature = "shell")]
pub fn label(key: &str, language: Language) -> String {
    t!(key, locale = language.locale()).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "shell")]
    #[test]
    fn system_languages_resolve_to_supported_catalogs() {
        for locale in ["zh", "zh-CN", "zh-Hant-TW", "ZH_cn"] {
            assert_eq!(system_locale(Some(locale)), "zh-CN");
        }
        for locale in [Some("en-SG"), Some("fr-FR"), Some(""), None] {
            assert_eq!(system_locale(locale), "en");
        }
    }

    #[test]
    fn appearance_defaults_and_persistence() {
        let defaults = AppearancePreferences::default();
        assert_eq!(defaults.language, Language::System);
        assert_eq!(defaults.theme, ThemePreference::System);
        let restored: AppearancePreferences = toml::from_str("").unwrap();
        assert_eq!(restored, defaults);
        for language in Language::ALL {
            let preferences = AppearancePreferences {
                language,
                theme: ThemePreference::Dark,
                ui_font_size: 20.0,
            };
            let restored: AppearancePreferences =
                toml::from_str(&toml::to_string(&preferences).unwrap()).unwrap();
            assert_eq!(preferences, restored);
        }
        let mut preferences = AppearancePreferences {
            ui_font_size: f32::NAN,
            ..defaults
        };
        preferences.normalize();
        assert_eq!(preferences.ui_font_size, 16.0);
    }
}
