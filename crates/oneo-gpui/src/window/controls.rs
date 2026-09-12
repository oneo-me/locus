use gpui_kit::component::{
    IconName, Theme,
    button::{Button, ButtonCustomVariant, ButtonVariants as _},
};
use gpui_kit::{App, Div, IntoElement, RenderOnce, Window, WindowControlArea, div, prelude::*, px};

use super::TITLEBAR_HEIGHT;

pub fn content_titlebar(content: impl IntoElement) -> Div {
    div()
        .debug_selector(|| "content-titlebar".to_owned())
        .w_full()
        .h(px(TITLEBAR_HEIGHT))
        .flex_none()
        .flex()
        .child(div().min_w_0().flex_1().h_full().child(content))
        .when(cfg!(target_os = "windows"), |bar| bar.child(WindowControls))
}

#[derive(IntoElement)]
struct WindowControls;

impl RenderOnce for WindowControls {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let supported = window.window_controls();
        div()
            .debug_selector(|| "window-controls".to_owned())
            .h_full()
            .flex_none()
            .flex()
            .when(supported.minimize, |row| {
                row.child(
                    control_button(
                        "window-minimize",
                        "window.minimize",
                        IconName::WindowMinimize,
                        cx,
                    )
                    .window_control_area(WindowControlArea::Min)
                    .on_click(|_, window, _| window.minimize_window()),
                )
            })
            .when(supported.maximize, |row| {
                let maximized = window.is_maximized();
                row.child(
                    control_button(
                        "window-maximize",
                        if maximized {
                            "window.restore"
                        } else {
                            "window.maximize"
                        },
                        if maximized {
                            IconName::WindowRestore
                        } else {
                            IconName::WindowMaximize
                        },
                        cx,
                    )
                    .window_control_area(WindowControlArea::Max)
                    .on_click(|_, window, _| super::platform::toggle_maximized(window)),
                )
            })
            .child(
                control_button("window-close", "window.close", IconName::WindowClose, cx)
                    .window_control_area(WindowControlArea::Close)
                    .custom(
                        ButtonCustomVariant::new(cx)
                            .foreground(Theme::global(cx).foreground)
                            .hover(Theme::global(cx).danger.opacity(0.15))
                            .active(Theme::global(cx).danger.opacity(0.25)),
                    )
                    .on_click(|_, window, cx| {
                        window.dispatch_action(Box::new(crate::CloseWindow), cx)
                    }),
            )
    }
}

fn control_button(id: &'static str, key: &'static str, icon: IconName, cx: &App) -> Button {
    let label = rust_i18n::t!(key, locale = &gpui_kit::component::locale()).into_owned();
    Button::new(id)
        .debug_selector(move || id.to_owned())
        .ghost()
        .rounded_none()
        .w(px(46.0))
        .h_full()
        .p_0()
        .icon(icon)
        .text_color(Theme::global(cx).foreground)
        .accessibility_label(label.clone())
        .tooltip(label)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::test::TestWindowExt;
    use gpui_kit::{Context, Render, TestAppContext};

    struct ControlsHarness {
        closes: usize,
        focus: gpui_kit::FocusHandle,
    }

    impl Render for ControlsHarness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .tab_group()
                .track_focus(&self.focus)
                .on_action(cx.listener(|this, _: &crate::CloseWindow, _, cx| {
                    this.closes += 1;
                    cx.notify();
                }))
                .w(px(500.0))
                .h(px(TITLEBAR_HEIGHT))
                .flex()
                .child(
                    div()
                        .debug_selector(|| "title-content".to_owned())
                        .flex_1()
                        .h_full(),
                )
                .child(WindowControls)
        }
    }

    #[gpui_kit::test]
    fn controls_reserve_space_and_close_dispatches_the_shell_action(cx: &mut TestAppContext) {
        cx.update(gpui_kit::component::init);
        let (view, cx) = cx.add_window_view(|window, cx| {
            let focus = cx.focus_handle();
            window.focus(&focus, cx);
            ControlsHarness { closes: 0, focus }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let content = cx.debug_bounds("title-content").unwrap();
        let controls = cx.debug_bounds("window-controls").unwrap();
        let close = cx.debug_bounds("window-close").unwrap();
        assert_eq!(content.right(), controls.left());
        assert_eq!(controls.right(), px(500.0));
        assert_eq!(close.size.height, px(TITLEBAR_HEIGHT));
        assert_eq!(close.right(), controls.right());
        cx.simulate_click(close.center(), Default::default());
        assert_eq!(view.read_with(cx, |view, _| view.closes), 1);
        cx.update(|window, cx| {
            for _ in 0..4 {
                window.focus_next(cx);
                window.render_frame(cx);
                if window.find("window-close").focused() == Some(true) {
                    return;
                }
            }
            panic!("Close button must be reachable by keyboard");
        });
        let keystroke = gpui_kit::Keystroke::parse("enter").unwrap();
        cx.simulate_event(gpui_kit::KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(gpui_kit::KeyUpEvent { keystroke });
        assert_eq!(view.read_with(cx, |view, _| view.closes), 2);
    }
}
