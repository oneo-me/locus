use gpui::{App, Hsla, Window, WindowAppearance, rgb};
pub use gpui_component::Theme;
use gpui_component::ThemeMode;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemePreference {
    System,
    Light,
    Dark,
}

pub fn workspace_background(theme: &Theme) -> Hsla {
    if cfg!(target_os = "macos") {
        window_material_tint(theme.is_dark())
    } else {
        theme.sidebar
    }
}

fn window_material_tint(dark: bool) -> Hsla {
    Hsla::from(if dark { rgb(0x181818) } else { rgb(0xf3f3f3) }).alpha(0.92)
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
    let mode = if resolves_to_dark(preference, cx.window_appearance()) {
        ThemeMode::Dark
    } else {
        ThemeMode::Light
    };
    Theme::change(mode, Some(window), cx);
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

    #[test]
    fn window_tints_match_the_existing_material_colors() {
        assert_eq!(
            window_material_tint(false),
            Hsla::from(rgb(0xf3f3f3)).alpha(0.92)
        );
        assert_eq!(
            window_material_tint(true),
            Hsla::from(rgb(0x181818)).alpha(0.92)
        );
    }
}
