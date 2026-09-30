use super::ThemePreference;
use gpui_kit::component::Theme;
use gpui_kit::component::ThemeMode;
use gpui_kit::{App, Hsla, Window, WindowAppearance, px, rgb};

pub fn workspace_background(theme: &Theme) -> Hsla {
    if cfg!(target_os = "windows") {
        // Mica supplies its own theme-aware tint through the native backdrop.
        theme.transparent
    } else if cfg!(target_os = "macos") {
        window_material_tint(theme.is_dark())
    } else {
        theme.sidebar
    }
}

fn window_material_tint(dark: bool) -> Hsla {
    Hsla::from(if dark { rgb(0x181818) } else { rgb(0xf3f3f3) }).alpha(0.85)
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

pub fn apply_preference(
    preference: ThemePreference,
    ui_font_size: f32,
    window: &mut Window,
    cx: &mut App,
) {
    super::platform::set_window_appearance(
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
    #[cfg(target_os = "windows")]
    crate::window::set_windows_material_dark(window, mode == ThemeMode::Dark);
    apply_ui_font_size(ui_font_size, window, cx);
}

pub fn apply_ui_font_size(size: f32, window: &mut Window, cx: &mut App) {
    Theme::global_mut(cx).font_size = px(size);
    Theme::sync_base(cx);
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

    #[test]
    fn window_tints_use_shared_material_colors_and_opacity() {
        assert_eq!(
            window_material_tint(false),
            Hsla::from(rgb(0xf3f3f3)).alpha(0.85)
        );
        assert_eq!(
            window_material_tint(true),
            Hsla::from(rgb(0x181818)).alpha(0.85)
        );
    }
}
