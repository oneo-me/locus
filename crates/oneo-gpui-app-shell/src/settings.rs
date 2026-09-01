use super::TITLEBAR_HEIGHT;
use gpui::{AnyElement, App, Div, FontWeight, Hsla, SharedString, div, prelude::*, px, rems};
use gpui_component::{Icon, IconName, Sizable as _, Theme};
use gpui_component::{
    Selectable as _,
    button::{Button, ButtonCustomVariant, ButtonVariants as _},
};

fn ui_rem(value: f32) -> gpui::Rems {
    rems(value / 16.0)
}

pub fn sidebar_selection_background(theme: &Theme) -> Hsla {
    theme.foreground.opacity(0.05)
}

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
                .pr(ui_rem(8.0))
                .py(ui_rem(7.0))
                .child(back_button),
        )
}

pub fn settings_navigation_group<E>(
    label: impl Into<SharedString>,
    rows: impl IntoIterator<Item = E>,
    theme: &Theme,
) -> Div
where
    E: IntoElement,
{
    rows.into_iter().fold(
        div().flex().flex_col().gap(ui_rem(3.0)).child(
            div()
                .h(ui_rem(22.0))
                .px(ui_rem(11.0))
                .flex()
                .items_center()
                .text_size(rems(11.0 / 16.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(theme.muted_foreground)
                .child(label.into()),
        ),
        |group, row| group.child(row),
    )
}

pub fn settings_navigation_row(
    id: &'static str,
    label: impl Into<SharedString>,
    icon_kind: IconName,
    selected: bool,
    cx: &App,
) -> Button {
    let label = label.into();
    let theme = Theme::global(cx);
    let selection_background = sidebar_selection_background(theme);
    Button::new(id)
        .custom(
            ButtonCustomVariant::new(cx)
                .foreground(if selected {
                    theme.foreground
                } else {
                    theme.muted_foreground
                })
                .hover(selection_background)
                .active(selection_background),
        )
        .small()
        .w_full()
        .h(ui_rem(36.0))
        .px(ui_rem(11.0))
        .text_size(rems(13.0 / 16.0))
        .text_color(if selected {
            theme.foreground
        } else {
            theme.muted_foreground
        })
        .accessibility_label(label.clone())
        .selected(selected)
        .child(
            div()
                .w_full()
                .min_w_0()
                .flex()
                .items_center()
                .gap(ui_rem(10.0))
                .child(Icon::new(icon_kind).small().text_color(if selected {
                    theme.foreground
                } else {
                    theme.muted_foreground
                }))
                .child(div().min_w_0().truncate().child(label)),
        )
}

pub fn settings_back_button(
    id: &'static str,
    label: impl Into<SharedString>,
    theme: &Theme,
) -> Button {
    let label = label.into();
    Button::new(id)
        .ghost()
        .small()
        .w_full()
        .h_full()
        .px(ui_rem(9.0))
        .text_size(rems(13.0 / 16.0))
        .text_color(theme.muted_foreground)
        .accessibility_label(label.clone())
        .child(
            div()
                .w_full()
                .min_w_0()
                .flex()
                .items_center()
                .gap(ui_rem(9.0))
                .child(
                    Icon::new(IconName::ArrowLeft)
                        .small()
                        .text_color(theme.muted_foreground),
                )
                .child(div().min_w_0().truncate().child(label)),
        )
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
            .bg(self.theme.background)
            .text_color(self.theme.foreground)
            .child(
                div()
                    .w(ui_rem(self.sidebar_width))
                    .h_full()
                    .flex_none()
                    .flex()
                    .flex_col()
                    .bg(self.theme.sidebar)
                    .text_color(self.theme.sidebar_foreground)
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
                    .bg(self.theme.background)
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

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Context, Render, TestAppContext, Window};

    struct SettingsNavigationHarness;

    impl Render for SettingsNavigationHarness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size(px(300.0))
                .child(settings_navigation_row(
                    "unselected-settings-row",
                    "General",
                    IconName::Settings,
                    false,
                    cx,
                ))
                .child(settings_navigation_row(
                    "selected-settings-row",
                    "Appearance",
                    IconName::Eye,
                    true,
                    cx,
                ))
        }
    }

    #[gpui::test]
    fn settings_navigation_rows_render_in_all_selection_states(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (_, cx) = cx.add_window_view(|_, _| SettingsNavigationHarness);
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }
}
