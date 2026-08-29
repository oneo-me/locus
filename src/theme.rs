use gpui::{App, Hsla, WindowAppearance, hsla, rgb};

pub struct Theme {
    pub is_dark: bool,
    pub surface: Hsla,
    pub sidebar_drag_background: Hsla,
    pub border: Hsla,
    pub border_strong: Hsla,
    pub text: Hsla,
    pub muted_text: Hsla,
    pub control_hover: Hsla,
    pub control_active: Hsla,
    pub focus: Hsla,
    pub resize_handle: Hsla,
}

impl Theme {
    pub fn current(cx: &App) -> Self {
        match cx.window_appearance() {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Self {
                is_dark: true,
                surface: rgb(0x1a1a1a).into(),
                sidebar_drag_background: rgb(0x181818).into(),
                border: hsla(0.0, 0.0, 1.0, 0.07),
                border_strong: hsla(220.0 / 360.0, 0.10, 0.90, 0.14),
                text: rgb(0xe2e2e2).into(),
                muted_text: rgb(0x7d7d7d).into(),
                control_hover: hsla(220.0 / 360.0, 0.10, 0.90, 0.05),
                control_active: hsla(220.0 / 360.0, 0.10, 0.90, 0.09),
                focus: rgb(0x3b82f6).into(),
                resize_handle: rgb(0x3b82f6).into(),
            },
            WindowAppearance::Light | WindowAppearance::VibrantLight => Self {
                is_dark: false,
                surface: rgb(0xf6f5f6).into(),
                sidebar_drag_background: rgb(0xf3f3f3).into(),
                border: hsla(0.0, 0.0, 0.0, 0.08),
                border_strong: hsla(220.0 / 360.0, 0.10, 0.12, 0.15),
                text: rgb(0x242424).into(),
                muted_text: rgb(0x858585).into(),
                control_hover: hsla(0.0, 0.0, 0.0, 0.05),
                control_active: hsla(0.0, 0.0, 0.0, 0.09),
                focus: rgb(0x2563eb).into(),
                resize_handle: rgb(0x2563eb).into(),
            },
        }
    }
}
