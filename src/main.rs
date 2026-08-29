mod app;
mod motion;
mod platform;
mod settings;
mod theme;
mod ui;

use app::Locus;
use gpui::{
    App, AppContext, Bounds, KeyBinding, TitlebarOptions, WindowAppearance,
    WindowBackgroundAppearance, WindowBounds, WindowOptions, actions, point, px, size,
};
use gpui_platform::application;

actions!(locus, [OpenSettings, ToggleLeftSidebar, ToggleRightSidebar]);

const WINDOW_WIDTH: f32 = 1180.0;
const WINDOW_HEIGHT: f32 = 760.0;
const WINDOW_MIN_WIDTH: f32 = 880.0;
const WINDOW_MIN_HEIGHT: f32 = 560.0;

fn main() {
    application().run(|cx: &mut App| {
        platform::init_reduce_motion(cx);
        cx.bind_keys([
            KeyBinding::new("secondary-,", OpenSettings, None),
            KeyBinding::new("secondary-b", ToggleLeftSidebar, None),
            KeyBinding::new("secondary-shift-b", ToggleRightSidebar, None),
        ]);

        let bounds = Bounds::centered(None, size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT)), cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(WINDOW_MIN_WIDTH), px(WINDOW_MIN_HEIGHT))),
                titlebar: Some(TitlebarOptions {
                    title: Some("Locus".into()),
                    appears_transparent: true,
                    traffic_light_position: Some(point(px(16.0), px(17.0))),
                }),
                app_owns_titlebar_drag: cfg!(target_os = "macos"),
                window_background: if cfg!(target_os = "macos") {
                    WindowBackgroundAppearance::Blurred
                } else {
                    WindowBackgroundAppearance::Opaque
                },
                ..Default::default()
            },
            |window, cx| {
                let dark = matches!(
                    cx.window_appearance(),
                    WindowAppearance::Dark | WindowAppearance::VibrantDark
                );
                platform::configure_sidebar_material(window, dark)
                    .expect("failed to install macOS sidebar material");
                cx.new(Locus::new)
            },
        )
        .expect("failed to open Locus window");

        cx.activate(true);
    });
}
