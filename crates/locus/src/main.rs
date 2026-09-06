mod platform;

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{
    App, AppContext, Bounds, Pixels, Styled as _, TitlebarOptions, WindowBackgroundAppearance,
    WindowBounds, WindowOptions, point, px, size,
};
use gpui_component::{Root, Theme};
use gpui_platform::application;
use locus_preferences::Preferences;
use locus_preferences::config::{
    WINDOW_HEIGHT, WINDOW_MIN_HEIGHT, WINDOW_MIN_WIDTH, WINDOW_TITLEBAR_HEIGHT, WINDOW_WIDTH,
};
use locus_workspace::{InterfaceState, Locus, set_app_menus};
use oneo_gpui::{About, Quit, bind_app_shell_keys};
use oneo_gpui::{
    configure_main_window_close_behavior, configure_sidebar_material, init_reduce_motion,
};

const MIN_VISIBLE_TITLEBAR_WIDTH: f32 = 120.0;
const MIN_VISIBLE_TITLEBAR_HEIGHT: f32 = 24.0;

fn main() {
    let main_window: Rc<RefCell<Option<gpui::WindowHandle<Root>>>> = Rc::new(RefCell::new(None));
    let locus_view = Rc::new(RefCell::new(None));
    let reopen_window = main_window.clone();
    let platform_application = application().with_assets(gpui_component_assets::Assets);
    platform_application.on_reopen(move |cx| {
        if let Some(window) = *reopen_window.borrow() {
            window
                .update(cx, |_, window, _| window.activate_window())
                .ok();
        }
        cx.activate(true);
    });

    platform_application.run(move |cx: &mut App| {
        locus_preferences::localization::init();
        gpui_component::init(cx);
        init_reduce_motion(cx);
        bind_app_shell_keys(cx, true);
        locus_workspace::bind_keys(cx);
        cx.on_action(|_: &About, _| platform::show_about_panel());
        cx.on_action(|_: &Quit, cx| cx.quit());
        let (preferences, load_error) = Preferences::load();
        let (interface_state, state_load_error) = InterfaceState::load();
        if let Some(error) = state_load_error {
            eprintln!("{error}");
        }
        let language = preferences.language;
        language.activate();

        let bounds = restored_window_bounds(interface_state.window_bounds, cx);

        let window = cx
            .open_window(
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
                    configure_main_window_close_behavior(window, cx);
                    configure_sidebar_material(window)
                        .expect("failed to install macOS sidebar material");
                    let view = cx
                        .new(|cx| Locus::new(window, preferences, load_error, interface_state, cx));
                    *locus_view.borrow_mut() = Some(view.clone());
                    cx.new(|cx| {
                        Root::new(view, window, cx)
                            .bordered(false)
                            .bg(Theme::global(cx).transparent)
                    })
                },
            )
            .expect("failed to open Locus window");
        *main_window.borrow_mut() = Some(window);

        let can_reload_current_note = locus_view
            .borrow()
            .as_ref()
            .map(|view| view.read(cx).can_reload_current_note())
            .unwrap_or(false);
        set_app_menus(cx, language, can_reload_current_note);
        cx.activate(true);
    });
}

fn restored_window_bounds(saved: Option<Bounds<Pixels>>, cx: &App) -> Bounds<Pixels> {
    saved
        .filter(|bounds| {
            cx.displays()
                .iter()
                .any(|display| window_titlebar_is_reachable(*bounds, display.visible_bounds()))
        })
        .unwrap_or_else(|| Bounds::centered(None, size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT)), cx))
}

fn window_titlebar_is_reachable(
    window_bounds: Bounds<Pixels>,
    visible_bounds: Bounds<Pixels>,
) -> bool {
    let titlebar = Bounds::new(
        window_bounds.origin,
        size(window_bounds.size.width, px(WINDOW_TITLEBAR_HEIGHT)),
    );
    let visible_titlebar = titlebar.intersect(&visible_bounds);
    f32::from(visible_titlebar.size.width) >= MIN_VISIBLE_TITLEBAR_WIDTH
        && f32::from(visible_titlebar.size.height) >= MIN_VISIBLE_TITLEBAR_HEIGHT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_window_requires_a_reachable_titlebar() {
        let display = Bounds::new(point(px(0.0), px(0.0)), size(px(1440.0), px(900.0)));
        let reachable = Bounds::new(point(px(1320.0), px(-24.0)), size(px(1080.0), px(920.0)));
        let unreachable = Bounds::new(point(px(1321.0), px(-25.0)), size(px(1080.0), px(920.0)));

        assert!(window_titlebar_is_reachable(reachable, display));
        assert!(!window_titlebar_is_reachable(unreachable, display));
    }
}
