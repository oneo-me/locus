mod group;
mod item;
use super::TITLEBAR_HEIGHT;
use gpui_kit::component::{Icon, IconName, Sizable as _, Theme};
use gpui_kit::component::{
    Selectable as _,
    button::{Button, ButtonCustomVariant, ButtonVariants as _},
};
use gpui_kit::{AnyElement, App, Div, FontWeight, Hsla, SharedString, div, prelude::*, px, rems};
pub use group::SettingsGroup;
pub use item::SettingsItem;

pub fn settings_button(id: &'static str, theme: &Theme) -> Button {
    crate::toolbar_icon_button(
        id,
        rust_i18n::t!("shell.settings", locale = &gpui_kit::component::locale()).into_owned(),
        IconName::Settings,
        theme,
    )
    .size_7()
    .on_click(|_, window, cx| window.dispatch_action(Box::new(crate::OpenSettings), cx))
}

fn ui_rem(value: f32) -> gpui_kit::Rems {
    rems(value / 16.0)
}

pub fn sidebar_selection_background(theme: &Theme) -> Hsla {
    theme.foreground.opacity(0.05)
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

pub fn settings_back_button(id: &'static str, label: impl Into<SharedString>, cx: &App) -> Button {
    settings_navigation_row(id, label, IconName::ArrowLeft, false, cx)
        .on_click(|_, window, cx| window.dispatch_action(Box::new(crate::CloseSettings), cx))
}

pub struct SettingsShell {
    sidebar: AnyElement,
    content: AnyElement,
    theme: Theme,
    back: Button,
}

impl SettingsShell {
    pub fn new(sidebar: AnyElement, content: AnyElement, cx: &App) -> Self {
        Self {
            sidebar,
            content,
            theme: Theme::global(cx).clone(),
            back: settings_back_button(
                "settings-back",
                rust_i18n::t!("shell.back", locale = &gpui_kit::component::locale()).into_owned(),
                cx,
            )
            .debug_selector(|| "settings-back".to_owned()),
        }
    }

    pub fn appearance(
        controls: &crate::AppearanceControls,
        language: crate::Language,
        additional: impl IntoElement,
        cx: &App,
    ) -> Self {
        let title = crate::appearance::label("shell.appearance", language);
        let sidebar = div().p_3().child(settings_navigation_row(
            "settings-appearance",
            title.clone(),
            IconName::Palette,
            true,
            cx,
        ));
        let content = div()
            .id("settings-scroll")
            .size_full()
            .overflow_y_scroll()
            .child(
                div()
                    .p_5()
                    .w_full()
                    .max_w(rems(45.0))
                    .mx_auto()
                    .flex()
                    .flex_col()
                    .gap_6()
                    .child(SettingsGroup::new(title).child(controls.render(language, cx)))
                    .child(additional),
            );
        Self::new(sidebar.into_any_element(), content.into_any_element(), cx)
    }

    pub fn into_element(self) -> Div {
        div()
            .size_full()
            .flex()
            .bg(self.theme.background)
            .text_color(self.theme.foreground)
            .child(
                div()
                    .debug_selector(|| "settings-sidebar".to_owned())
                    .w(px(crate::DEFAULT_LEFT_SIDEBAR_WIDTH))
                    .h_full()
                    .flex_none()
                    .flex()
                    .flex_col()
                    .bg(self.theme.sidebar)
                    .text_color(self.theme.sidebar_foreground)
                    .border_r_1()
                    .border_color(self.theme.sidebar_border)
                    .when(cfg!(target_os = "macos"), |sidebar| {
                        sidebar.child(
                            div().h(px(TITLEBAR_HEIGHT)).flex_none().child(
                                crate::titlebar_drag_region("settings-sidebar-drag").w_full(),
                            ),
                        )
                    })
                    .child(
                        div()
                            .flex_none()
                            .px_3()
                            .when(!cfg!(target_os = "macos"), |back| back.pt_3())
                            .child(self.back),
                    )
                    .child(div().min_h_0().flex_1().child(self.sidebar)),
            )
            .child(
                div()
                    .debug_selector(|| "settings-content".to_owned())
                    .min_w_0()
                    .h_full()
                    .flex_1()
                    .bg(self.theme.background)
                    .flex()
                    .flex_col()
                    .child(crate::content_titlebar(
                        crate::titlebar_drag_region("settings-content-drag").w_full(),
                    ))
                    .child(div().flex_1().min_h_0().child(self.content)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{Context, Render, TestAppContext, Window};

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

    #[gpui_kit::test]
    fn settings_navigation_rows_render_in_all_selection_states(cx: &mut TestAppContext) {
        cx.update(gpui_kit::component::init);
        let (_, cx) = cx.add_window_view(|_, _| SettingsNavigationHarness);
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }

    struct SettingsReturnHarness {
        returned: bool,
        workspace_focus: gpui_kit::FocusHandle,
    }

    impl Render for SettingsReturnHarness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            if self.returned {
                return div().track_focus(&self.workspace_focus).into_any_element();
            }
            SettingsShell::new(
                div().into_any_element(),
                div().child("Settings content").into_any_element(),
                cx,
            )
            .into_element()
            .track_focus(&self.workspace_focus)
            .on_action(cx.listener(|this, _: &crate::CloseSettings, window, cx| {
                this.returned = true;
                window.focus(&this.workspace_focus, cx);
                cx.notify();
            }))
            .into_any_element()
        }
    }

    #[gpui_kit::test]
    fn settings_top_return_activates_and_restores_workspace_focus(cx: &mut TestAppContext) {
        cx.update(gpui_kit::component::init);
        let (view, cx) = cx.add_window_view(|window, cx| {
            let workspace_focus = cx.focus_handle();
            window.focus(&workspace_focus, cx);
            SettingsReturnHarness {
                returned: false,
                workspace_focus,
            }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let back = cx.debug_bounds("settings-back").unwrap();
        let sidebar = cx.debug_bounds("settings-sidebar").unwrap();
        assert_eq!(sidebar.size.width, px(crate::DEFAULT_LEFT_SIDEBAR_WIDTH));
        let top_inset = if cfg!(target_os = "macos") {
            px(TITLEBAR_HEIGHT)
        } else {
            back.left() - sidebar.left()
        };
        assert_eq!(back.top(), sidebar.top() + top_inset);
        cx.simulate_click(back.center(), Default::default());
        cx.update(|window, cx| {
            assert!(view.read(cx).returned);
            assert!(view.read(cx).workspace_focus.is_focused(window));
        });
    }
}
