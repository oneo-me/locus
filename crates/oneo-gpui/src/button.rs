use gpui_kit::component::{
    IconName, Sizable as _, Theme,
    button::{Button, ButtonVariants as _},
};
use gpui_kit::{ElementId, InteractiveElement as _, SharedString, Styled as _, px};

use crate::TITLEBAR_CONTROL_SIZE;

pub fn toolbar_icon_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    icon: impl Into<gpui_kit::component::Icon>,
    theme: &Theme,
) -> Button {
    icon_button(id, label, icon, theme).occlude()
}

fn icon_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    icon: impl Into<gpui_kit::component::Icon>,
    theme: &Theme,
) -> Button {
    let label = label.into();
    Button::new(id)
        .ghost()
        .small()
        .size(px(TITLEBAR_CONTROL_SIZE))
        .icon(icon.into())
        .text_color(theme.muted_foreground)
        .accessibility_label(label.clone())
        .tooltip(label)
}

pub fn toolbar_dropdown_menu(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    icon: IconName,
    theme: &Theme,
    configure: impl FnOnce(Button) -> Button,
    anchor: gpui_kit::Anchor,
    menu: impl Fn(
        gpui_kit::component::menu::PopupMenu,
        &mut gpui_kit::Window,
        &mut gpui_kit::Context<gpui_kit::component::menu::PopupMenu>,
    ) -> gpui_kit::component::menu::PopupMenu
    + 'static,
) -> gpui_kit::Div {
    use gpui_kit::component::menu::DropdownMenu as _;
    use gpui_kit::{ParentElement as _, div};
    let button = configure(icon_button(id, label, icon, theme));
    div()
        .occlude()
        .child(button.dropdown_menu_with_anchor(anchor, menu))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::component::Disableable as _;
    use gpui_kit::{
        Context, MouseButton, MouseDownEvent, MouseUpEvent, Render, TestAppContext, Window, div,
        point, prelude::*,
    };

    #[derive(Default)]
    struct ToolbarHarness {
        presses: usize,
        clicks: usize,
        activations: usize,
        disabled: bool,
    }

    impl Render for ToolbarHarness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .id("titlebar")
                .w(px(300.0))
                .h(px(48.0))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, _| this.presses += 1),
                )
                .on_click(cx.listener(|this, _, _, _| this.clicks += 1))
                .child(
                    toolbar_icon_button("tool", "Tool", IconName::Settings, Theme::global(cx))
                        .debug_selector(|| "tool".to_owned())
                        .disabled(self.disabled)
                        .on_click(cx.listener(|this, _, _, _| this.activations += 1)),
                )
        }
    }

    #[gpui_kit::test]
    fn toolbar_gestures_do_not_reach_the_titlebar(cx: &mut TestAppContext) {
        cx.update(gpui_kit::component::init);
        let (view, cx) = cx.add_window_view(|_, _| ToolbarHarness::default());
        for disabled in [false, true] {
            view.update(cx, |view, cx| {
                view.disabled = disabled;
                cx.notify();
            });
            cx.update(|window, cx| window.draw(cx).clear(cx));
            let position = cx.debug_bounds("tool").unwrap().center();
            cx.simulate_click(position, Default::default());
            cx.simulate_event(MouseDownEvent {
                position,
                button: MouseButton::Left,
                click_count: 2,
                ..Default::default()
            });
            cx.simulate_event(MouseUpEvent {
                position,
                button: MouseButton::Left,
                click_count: 2,
                ..Default::default()
            });
            cx.simulate_mouse_down(position, MouseButton::Left, Default::default());
            let outside = point(px(150.0), px(20.0));
            cx.simulate_mouse_move(outside, MouseButton::Left, Default::default());
            cx.simulate_mouse_up(outside, MouseButton::Left, Default::default());
            view.read_with(cx, |view, _| {
                assert_eq!(view.presses, 0);
                assert_eq!(view.clicks, 0);
                assert_eq!(view.activations, 2);
            });
        }
        cx.simulate_click(point(px(150.0), px(20.0)), Default::default());
        view.read_with(cx, |view, _| {
            assert_eq!(view.presses, 1);
            assert_eq!(view.clicks, 1);
        });
    }
    #[gpui_kit::test]
    fn toolbar_dropdown_still_opens(cx: &mut TestAppContext) {
        use std::{cell::Cell, rc::Rc};
        struct MenuHarness(Rc<Cell<bool>>);
        impl Render for MenuHarness {
            fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
                let opened = self.0.clone();
                toolbar_dropdown_menu(
                    "menu",
                    "Menu",
                    IconName::Ellipsis,
                    Theme::global(cx),
                    |button| button.debug_selector(|| "menu-trigger".to_owned()),
                    gpui_kit::Anchor::TopRight,
                    move |menu, _, _| {
                        opened.set(true);
                        menu
                    },
                )
            }
        }
        cx.update(gpui_kit::component::init);
        let opened = Rc::new(Cell::new(false));
        let (_, cx) = cx.add_window_view(|_, _| MenuHarness(opened.clone()));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let bounds = cx.debug_bounds("menu-trigger").unwrap();
        cx.simulate_click(bounds.center(), Default::default());
        assert!(opened.get());
    }
}
