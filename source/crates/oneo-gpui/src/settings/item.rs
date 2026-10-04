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
                                .w_full()
                                .whitespace_normal()
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

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::test::{TestSupportExt, TestWindowExt};
    use gpui_kit::{Context, Render, TestAppContext};

    struct ItemHarness;

    impl Render for ItemHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .w(rems(28.0))
                .whitespace_nowrap()
                .child(div().id("short-row").test_support().child(SettingsItem::new(
                    "Version", "", div().w(rems(7.0)).child("0.2026.1"),
                )))
                .child(div().id("long-row").test_support().child(SettingsItem::new(
                    "Updates",
                    "An error occurred in retrieving update information. Please try again later. ".repeat(4),
                    div().w(rems(7.0)).child("Check for updates"),
                )))
        }
    }

    #[gpui_kit::test]
    fn long_descriptions_wrap_even_inside_a_nowrap_parent(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let (_, cx) = cx.add_window_view(|_, _| ItemHarness);
        cx.update(|window, cx| {
            window.render_frame(cx);
            assert!(
                window.find("long-row").bounds().size.height
                    > window.find("short-row").bounds().size.height
            );
        });
    }
}
