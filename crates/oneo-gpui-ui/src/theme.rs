use gpui::{App, Global, Hsla, hsla, rgb};

#[derive(Clone, Copy)]
pub struct Theme {
    pub is_dark: bool,
    pub window_background: Hsla,
    pub surface: Hsla,
    pub raised: Hsla,
    pub border: Hsla,
    pub border_strong: Hsla,
    pub sidebar_border: Hsla,
    pub text: Hsla,
    pub text_secondary: Hsla,
    pub text_tertiary: Hsla,
    pub control_hover: Hsla,
    pub control_active: Hsla,
    pub control_selected: Hsla,
    pub focus: Hsla,
    pub resize_handle: Hsla,
    pub error: Hsla,
    pub success: Hsla,
    pub warning: Hsla,
}

impl Theme {
    pub fn current(cx: &App) -> Self {
        cx.try_global::<ActiveTheme>()
            .map_or_else(Self::light, |active| active.0)
    }

    pub fn dark() -> Self {
        Self {
            is_dark: true,
            window_background: if cfg!(target_os = "macos") {
                Hsla::from(rgb(0x181818)).opacity(0.92)
            } else {
                rgb(0x181818).into()
            },
            surface: rgb(0x1a1a1a).into(),
            raised: rgb(0x232323).into(),
            border: hsla(220.0 / 360.0, 0.10, 0.90, 0.07),
            border_strong: hsla(220.0 / 360.0, 0.10, 0.90, 0.14),
            sidebar_border: hsla(126.93 / 360.0, 0.000_000_1, 0.16077, 1.0),
            text: rgb(0xe2e2e2).into(),
            text_secondary: rgb(0xa3a3a3).into(),
            text_tertiary: rgb(0x7d7d7d).into(),
            control_hover: hsla(220.0 / 360.0, 0.10, 0.90, 0.05),
            control_active: hsla(220.0 / 360.0, 0.10, 0.90, 0.09),
            control_selected: hsla(0.0, 0.0, 0.941, 0.06),
            focus: rgb(0x3b82f6).into(),
            resize_handle: rgb(0x3b82f6).into(),
            error: rgb(0xe2726a).into(),
            success: rgb(0x58a66a).into(),
            warning: rgb(0xd29b3d).into(),
        }
    }

    pub fn light() -> Self {
        Self {
            is_dark: false,
            window_background: if cfg!(target_os = "macos") {
                Hsla::from(rgb(0xf3f3f3)).opacity(0.92)
            } else {
                rgb(0xf3f3f3).into()
            },
            surface: rgb(0xf6f5f6).into(),
            raised: rgb(0xececec).into(),
            border: hsla(220.0 / 360.0, 0.10, 0.12, 0.08),
            border_strong: hsla(220.0 / 360.0, 0.10, 0.12, 0.15),
            sidebar_border: hsla(0.0, 0.0, 0.078, 0.12),
            text: rgb(0x242424).into(),
            text_secondary: rgb(0x666666).into(),
            text_tertiary: rgb(0x858585).into(),
            control_hover: hsla(220.0 / 360.0, 0.10, 0.12, 0.05),
            control_active: hsla(220.0 / 360.0, 0.10, 0.12, 0.09),
            control_selected: hsla(0.0, 0.0, 0.078, 0.06),
            focus: rgb(0x2563eb).into(),
            resize_handle: rgb(0x2563eb).into(),
            error: rgb(0xc64a42).into(),
            success: rgb(0x27823f).into(),
            warning: rgb(0xa66a10).into(),
        }
    }
}

#[derive(Clone, Copy)]
struct ActiveTheme(Theme);

impl Global for ActiveTheme {}

pub fn set_theme(theme: Theme, cx: &mut App) {
    cx.set_global(ActiveTheme(theme));
}
