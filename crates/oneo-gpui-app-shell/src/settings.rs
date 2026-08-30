use super::TITLEBAR_HEIGHT;
use gpui::{
    AnyElement, Div, FocusHandle, FontWeight, Role, SharedString, Stateful, div, prelude::*, px,
    rems,
};
use oneo_gpui_ui::{Icon, Theme, icon};

pub fn settings_sidebar_header(
    left_control_clearance: f32,
    drag_region: AnyElement,
    back_button: AnyElement,
) -> Div {
    div()
        .size_full()
        .flex()
        .items_center()
        .child(
            div()
                .w(px(left_control_clearance))
                .h_full()
                .flex_none()
                .child(drag_region),
        )
        .child(
            div()
                .h_full()
                .flex_1()
                .pr(px(8.0))
                .py(px(7.0))
                .child(back_button),
        )
}

pub fn settings_navigation_group<E>(
    label: impl Into<SharedString>,
    rows: impl IntoIterator<Item = E>,
    theme: Theme,
) -> Div
where
    E: IntoElement,
{
    rows.into_iter().fold(
        div().flex().flex_col().gap(px(3.0)).child(
            div()
                .h(px(22.0))
                .px(px(11.0))
                .flex()
                .items_center()
                .text_size(rems(11.0 / 16.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(theme.text_tertiary)
                .child(label.into()),
        ),
        |group, row| group.child(row),
    )
}

pub fn settings_navigation_row(
    id: &'static str,
    label: &'static str,
    icon_kind: Icon,
    focus: &FocusHandle,
    selected: bool,
    theme: Theme,
) -> Stateful<Div> {
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(label)
        .track_focus(focus)
        .tab_index(0)
        .tab_stop(true)
        .h(px(36.0))
        .px(px(11.0))
        .rounded(px(8.0))
        .flex()
        .items_center()
        .gap(px(10.0))
        .cursor_default()
        .text_size(rems(13.0 / 16.0))
        .text_color(if selected {
            theme.text
        } else {
            theme.text_secondary
        })
        .when(selected, |row| row.bg(theme.control_selected))
        .hover(|style| style.bg(theme.control_selected))
        .active(|style| style.bg(theme.control_selected))
        .focus_visible(|style| style.border_1().border_color(theme.focus))
        .child(icon(
            icon_kind,
            15.0,
            if selected {
                theme.text_secondary
            } else {
                theme.text_tertiary
            },
        ))
        .child(label)
}

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
        .text_size(rems(13.0 / 16.0))
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
            titlebar_height: TITLEBAR_HEIGHT,
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
                    .border_r_1()
                    .border_color(self.theme.sidebar_border)
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
