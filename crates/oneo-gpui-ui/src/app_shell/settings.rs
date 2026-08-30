use crate::{Icon, Theme, icon};
use gpui::{AnyElement, Div, FocusHandle, Role, Stateful, div, prelude::*, px};

pub fn settings_back_button(
    id: &'static str,
    label: &'static str,
    focus: &FocusHandle,
    theme: Theme,
) -> Stateful<Div> {
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(label)
        .track_focus(focus)
        .tab_index(0)
        .tab_stop(true)
        .size_full()
        .px(px(9.0))
        .rounded(px(8.0))
        .flex()
        .items_center()
        .cursor_default()
        .text_sm()
        .text_color(theme.text_secondary)
        .hover(|style| style.bg(theme.control_hover))
        .active(|style| style.bg(theme.control_active))
        .focus_visible(|style| style.border_1().border_color(theme.focus))
        .gap(px(9.0))
        .child(icon(Icon::ArrowLeft, 15.0, theme.text_tertiary))
        .child(label)
}

pub struct SettingsShell {
    sidebar_header: AnyElement,
    sidebar: AnyElement,
    content_header: AnyElement,
    content: AnyElement,
    sidebar_width: f32,
    titlebar_height: f32,
    theme: Theme,
}

impl SettingsShell {
    pub fn new(
        sidebar_header: AnyElement,
        sidebar: AnyElement,
        content_header: AnyElement,
        content: AnyElement,
        theme: Theme,
    ) -> Self {
        Self {
            sidebar_header,
            sidebar,
            content_header,
            content,
            sidebar_width: 252.0,
            titlebar_height: 48.0,
            theme,
        }
    }

    pub fn sidebar_width(mut self, width: f32) -> Self {
        self.sidebar_width = width;
        self
    }

    pub fn titlebar_height(mut self, height: f32) -> Self {
        self.titlebar_height = height;
        self
    }

    pub fn into_element(self) -> Div {
        div()
            .size_full()
            .flex()
            .bg(self.theme.surface)
            .text_color(self.theme.text)
            .child(
                div()
                    .w(px(self.sidebar_width))
                    .h_full()
                    .flex_none()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .h(px(self.titlebar_height))
                            .flex_none()
                            .child(self.sidebar_header),
                    )
                    .child(div().min_h_0().flex_1().child(self.sidebar)),
            )
            .child(
                div()
                    .min_w_0()
                    .h_full()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .border_l_1()
                    .border_color(self.theme.sidebar_border)
                    .bg(self.theme.surface)
                    .child(
                        div()
                            .h(px(self.titlebar_height))
                            .flex_none()
                            .child(self.content_header),
                    )
                    .child(div().min_h_0().flex_1().child(self.content)),
            )
    }
}
