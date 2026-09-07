use gpui_kit::component::ActiveTheme;
use gpui_kit::{AnyElement, App, FontWeight, SharedString, Window, div, prelude::*, rems};

#[derive(IntoElement)]
pub struct SettingsItem {
    title: SharedString,
    description: SharedString,
    control: AnyElement,
}

impl SettingsItem {
    pub fn new(
        title: impl Into<SharedString>,
        description: impl Into<SharedString>,
        control: impl IntoElement,
    ) -> Self {
        Self {
            title: title.into(),
            description: description.into(),
            control: control.into_any_element(),
        }
    }
}

impl RenderOnce for SettingsItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .w_full()
            .min_w_0()
            .min_h(rems(4.25))
            .px_5()
            .py(rems(13.0 / 16.0))
            .flex()
            .items_center()
            .gap_6()
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .child(
                        div()
                            .text_sm()
                            .line_height(rems(1.125))
                            .font_weight(FontWeight::MEDIUM)
                            .child(self.title),
                    )
                    .when(!self.description.is_empty(), |text| {
                        text.child(
                            div()
                                .mt_0p5()
                                .text_xs()
                                .line_height(rems(1.0))
                                .text_color(cx.theme().muted_foreground)
                                .child(self.description),
                        )
                    }),
            )
            .child(div().flex_none().min_w_0().child(self.control))
    }
}
