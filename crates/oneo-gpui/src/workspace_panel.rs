use crate::{TITLEBAR_HEIGHT, titlebar_drag_region};
use gpui_kit::{AnyElement, Div, div, prelude::*, px};

pub const DEFAULT_LEFT_SIDEBAR_WIDTH: f32 = 252.0;
pub const LEFT_SIDEBAR_MIN_WIDTH: f32 = DEFAULT_LEFT_SIDEBAR_WIDTH;

pub struct WorkspacePanel {
    content: AnyElement,
    header: Option<AnyElement>,
    footer: Option<AnyElement>,
}

impl WorkspacePanel {
    pub fn new(content: impl IntoElement) -> Self {
        Self {
            content: content.into_any_element(),
            header: None,
            footer: None,
        }
    }

    pub fn header(mut self, header: impl IntoElement) -> Self {
        self.header = Some(header.into_any_element());
        self
    }

    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }

    pub fn into_element(self) -> Div {
        div()
            .relative()
            .size_full()
            .min_w_0()
            .text_sm()
            .flex()
            .flex_col()
            .when_some(self.header, |panel, header| {
                panel.child(
                    div()
                        .h(px(TITLEBAR_HEIGHT))
                        .flex_none()
                        .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                        .child(header),
                )
            })
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(self.content),
            )
            .when_some(self.footer, |panel, footer| {
                panel.child(div().flex_none().child(footer))
            })
    }
}

pub struct WorkspaceSidebar {
    content: AnyElement,
    settings: AnyElement,
    header_end: Option<AnyElement>,
    footer_start: Option<AnyElement>,
    control_clearance: f32,
}

impl WorkspaceSidebar {
    pub fn new(
        content: impl IntoElement,
        theme: &gpui_kit::component::Theme,
        control_clearance: f32,
    ) -> Self {
        Self {
            content: content.into_any_element(),
            settings: crate::settings_button("open-settings", theme)
                .debug_selector(|| "open-settings".to_owned())
                .into_any_element(),
            header_end: None,
            footer_start: None,
            control_clearance,
        }
    }

    pub fn header_end(mut self, content: impl IntoElement) -> Self {
        self.header_end = Some(content.into_any_element());
        self
    }

    pub fn footer_start(mut self, content: impl IntoElement) -> Self {
        self.footer_start = Some(content.into_any_element());
        self
    }

    pub fn into_element(self) -> Div {
        let header = div()
            .debug_selector(|| "workspace-sidebar-header".to_owned())
            .size_full()
            .flex()
            .items_center()
            .child(
                titlebar_drag_region("sidebar-titlebar-drag")
                    .flex_1()
                    .min_w(px(self.control_clearance)),
            )
            .when_some(self.header_end, |header, content| {
                header.child(div().min_w_0().pr_3().child(content))
            });
        let footer = div()
            .debug_selector(|| "workspace-sidebar-footer".to_owned())
            .p_2()
            .flex()
            .items_center()
            .gap_2()
            .child(div().flex_1().min_w_0().children(self.footer_start))
            .child(div().flex_none().child(self.settings));
        WorkspacePanel::new(self.content)
            .header(header)
            .footer(footer)
            .into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{Context, Render, TestAppContext, Window};

    struct EmptyPanel;

    impl Render for EmptyPanel {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().w(px(280.0)).h(px(600.0)).child(
                WorkspacePanel::new(
                    div()
                        .min_h_0()
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .debug_selector(|| "empty-message".to_owned())
                                .h(px(20.0))
                                .child("No content"),
                        ),
                )
                .into_element()
                .debug_selector(|| "empty-panel".to_owned()),
            )
        }
    }

    #[gpui_kit::test]
    fn panel_content_fills_available_height(cx: &mut TestAppContext) {
        let (_, cx) = cx.add_window_view(|_, _| EmptyPanel);
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let panel = cx.debug_bounds("empty-panel").unwrap();
        let message = cx.debug_bounds("empty-message").unwrap();
        assert_eq!(message.center(), panel.center());
    }
}
