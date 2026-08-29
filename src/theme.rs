use gpui::{App, Global, Hsla, Window, WindowAppearance, hsla, rgb, transparent_black};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Clone, Copy)]
pub struct Theme {
    pub is_dark: bool,
    pub sidebar: Hsla,
    pub surface: Hsla,
    pub raised: Hsla,
    pub sidebar_drag_background: Hsla,
    pub border: Hsla,
    pub border_strong: Hsla,
    pub text: Hsla,
    pub muted_text: Hsla,
    pub control_hover: Hsla,
    pub control_active: Hsla,
    pub control_selected: Hsla,
    pub focus: Hsla,
    pub resize_handle: Hsla,
    pub error: Hsla,
}

impl Theme {
    pub fn current(cx: &App) -> Self {
        if cx.has_global::<ActiveTheme>() {
            cx.global::<ActiveTheme>().0
        } else if resolves_to_dark(ThemePreference::System, cx.window_appearance()) {
            Self::dark()
        } else {
            Self::light()
        }
    }

    fn dark() -> Self {
        Self {
            is_dark: true,
            sidebar: if cfg!(target_os = "macos") {
                transparent_black()
            } else {
                rgb(0x181818).into()
            },
            surface: rgb(0x1a1a1a).into(),
            raised: rgb(0x232323).into(),
            sidebar_drag_background: rgb(0x181818).into(),
            border: hsla(0.0, 0.0, 1.0, 0.07),
            border_strong: hsla(220.0 / 360.0, 0.10, 0.90, 0.14),
            text: rgb(0xe2e2e2).into(),
            muted_text: rgb(0x8b8b8b).into(),
            control_hover: hsla(220.0 / 360.0, 0.10, 0.90, 0.08),
            control_active: hsla(220.0 / 360.0, 0.10, 0.90, 0.12),
            control_selected: hsla(220.0 / 360.0, 0.10, 0.90, 0.09),
            focus: rgb(0x3b82f6).into(),
            resize_handle: rgb(0x3b82f6).into(),
            error: rgb(0xe2726a).into(),
        }
    }

    fn light() -> Self {
        Self {
            is_dark: false,
            sidebar: if cfg!(target_os = "macos") {
                transparent_black()
            } else {
                rgb(0xf3f3f3).into()
            },
            surface: rgb(0xf6f5f6).into(),
            raised: rgb(0xececec).into(),
            sidebar_drag_background: rgb(0xf3f3f3).into(),
            border: hsla(0.0, 0.0, 0.0, 0.08),
            border_strong: hsla(220.0 / 360.0, 0.10, 0.12, 0.15),
            text: rgb(0x242424).into(),
            muted_text: rgb(0x737373).into(),
            control_hover: hsla(0.0, 0.0, 0.0, 0.07),
            control_active: hsla(0.0, 0.0, 0.0, 0.11),
            control_selected: hsla(0.0, 0.0, 0.0, 0.07),
            focus: rgb(0x2563eb).into(),
            resize_handle: rgb(0x2563eb).into(),
            error: rgb(0xc64a42).into(),
        }
    }
}

#[derive(Clone, Copy)]
struct ActiveTheme(Theme);

impl Global for ActiveTheme {}

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
    cx.set_global(ActiveTheme(theme));
    crate::platform::set_sidebar_material_color(theme.is_dark)
        .expect("failed to update macOS sidebar material color");
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
