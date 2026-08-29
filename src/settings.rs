use gpui::{
    App, AppContext, Bounds, Context, FocusHandle, Render, TitlebarOptions, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowOptions, div, prelude::*, px,
    size,
};

use crate::{CloseWindow, theme::Theme};

pub struct Settings {
    focus: FocusHandle,
}

pub fn open_settings_window(cx: &mut App) -> WindowHandle<Settings> {
    let bounds = Bounds::centered(None, size(px(720.0), px(520.0)), cx);

    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_min_size: Some(size(px(560.0), px(400.0))),
            titlebar: Some(TitlebarOptions {
                title: Some("设置".into()),
                appears_transparent: true,
                traffic_light_position: None,
            }),
            window_background: WindowBackgroundAppearance::Opaque,
            ..Default::default()
        },
        |window, cx| {
            cx.new(|cx| {
                let focus = cx.focus_handle();
                window.focus(&focus, cx);
                Settings { focus }
            })
        },
    )
    .expect("failed to open settings window")
}

impl Render for Settings {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus)
            .on_action(|_: &CloseWindow, window, _| window.remove_window())
            .size_full()
            .bg(Theme::current(cx).surface)
    }
}
