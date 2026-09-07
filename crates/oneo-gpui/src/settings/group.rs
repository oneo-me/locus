use gpui_kit::component::ActiveTheme;
use gpui_kit::{AnyElement, App, FontWeight, SharedString, Window, div, prelude::*};

#[derive(IntoElement)]
pub struct SettingsGroup {
    title: SharedString,
    children: Vec<AnyElement>,
}

impl SettingsGroup {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            children: Vec::new(),
        }
    }
}

impl ParentElement for SettingsGroup {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for SettingsGroup {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .pl_3()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(self.title),
            )
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .rounded(gpui_kit::rems(13.0 / 16.0))
                    .bg(cx.theme().muted)
                    .flex()
                    .flex_col()
                    .items_stretch()
                    .children(self.children),
            )
    }
}
