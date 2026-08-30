use gpui::{App, Window, WindowAppearance};
pub use gpui_ui::Theme;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemePreference {
    System,
    Light,
    Dark,
}

pub fn resolves_to_dark(preference: ThemePreference, appearance: WindowAppearance) -> bool {
    match preference {
        ThemePreference::System => matches!(
            appearance,
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        ),
        ThemePreference::Light => false,
        ThemePreference::Dark => true,
    }
}

pub fn apply_preference(preference: ThemePreference, window: &mut Window, cx: &mut App) {
    crate::platform::set_window_appearance(
        window,
        match preference {
            ThemePreference::System => None,
            ThemePreference::Light => Some(false),
            ThemePreference::Dark => Some(true),
        },
    );
    let theme = if resolves_to_dark(preference, cx.window_appearance()) {
        Theme::dark()
    } else {
        Theme::light()
    };
    gpui_ui::set_theme(theme, cx);
    window.refresh();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_theme_ignores_system_appearance() {
        assert!(!resolves_to_dark(
            ThemePreference::Light,
            WindowAppearance::Dark
        ));
        assert!(resolves_to_dark(
            ThemePreference::Dark,
            WindowAppearance::Light
        ));
    }
}
