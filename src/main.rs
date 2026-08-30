mod app;
mod config;
mod document_library;
mod localization;
mod mcp;
mod platform;
mod preferences;
mod process_environment;
mod sidebar_order;
mod state;
mod storage;
mod theme;
mod ui;

use std::cell::RefCell;
use std::rc::Rc;

use app::Locus;
use config::{
    WINDOW_HEIGHT, WINDOW_MIN_HEIGHT, WINDOW_MIN_WIDTH, WINDOW_TITLEBAR_HEIGHT, WINDOW_WIDTH,
};
use gpui::{
    App, AppContext, Bounds, KeyBinding, MenuItem, Pixels, TitlebarOptions,
    WindowBackgroundAppearance, WindowBounds, WindowOptions, actions, point, px, size,
};
use gpui_platform::application;
use localization::Language;
use oneo_gpui_ui::app_shell::{
    About, AppShellMenuLabels, AppShellMenus, CloseWindow, OpenSettings, Quit, ToggleFpsCounter,
    ToggleFullScreen, ToggleLeftPanel, ToggleRightPanel, bind_app_shell_keys, set_app_shell_menus,
};
use oneo_gpui_ui::{
    configure_main_window_close_behavior, configure_sidebar_material, init_reduce_motion,
};
use preferences::Preferences;
use state::InterfaceState;

const MIN_VISIBLE_TITLEBAR_WIDTH: f32 = 120.0;
const MIN_VISIBLE_TITLEBAR_HEIGHT: f32 = 24.0;

actions!(locus, [ReloadCurrentNote]);

fn main() {
    let main_window: Rc<RefCell<Option<gpui::WindowHandle<Locus>>>> = Rc::new(RefCell::new(None));
    let reopen_window = main_window.clone();
    let platform_application = application();
    platform_application.on_reopen(move |cx| {
        if let Some(window) = *reopen_window.borrow() {
            window
                .update(cx, |_, window, _| window.activate_window())
                .ok();
        }
        cx.activate(true);
    });

    platform_application.run(move |cx: &mut App| {
        init_reduce_motion(cx);
        app::bind_keys(cx);
        bind_app_shell_keys(cx);
        cx.bind_keys([KeyBinding::new("secondary-r", ReloadCurrentNote, None)]);
        cx.on_action(|_: &About, _| platform::show_about_panel());
        cx.on_action(|_: &Quit, cx| cx.quit());
        let (preferences, load_error) = Preferences::load();
        let (interface_state, state_load_error) = InterfaceState::load();
        if let Some(error) = state_load_error {
            eprintln!("{error}");
        }
        let language = preferences.language;

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
                    cx.new(|cx| Locus::new(window, preferences, load_error, interface_state, cx))
                },
            )
            .expect("failed to open Locus window");
        *main_window.borrow_mut() = Some(window);

        let can_reload_current_note = window
            .update(cx, |locus, _, _| locus.can_reload_current_note())
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

pub(crate) fn set_app_menus(cx: &mut App, language: Language, can_reload_current_note: bool) {
    set_app_shell_menus(
        cx,
        AppShellMenus::new("Locus")
            .labels(AppShellMenuLabels {
                about: language.text("关于 Locus", "About Locus").into(),
                close_window: language.text("关闭窗口", "Close Window").into(),
                developer: language.text("开发", "Developer").into(),
                edit: language.text("编辑", "Edit").into(),
                file: language.text("文件", "File").into(),
                quit: language.text("退出 Locus", "Quit Locus").into(),
                settings: language.text("设置…", "Settings…").into(),
                toggle_left_panel: language.text("切换左侧栏", "Toggle Left Sidebar").into(),
                toggle_right_panel: language.text("切换右侧栏", "Toggle Right Sidebar").into(),
                toggle_fps_counter: language.text("显示帧率", "Show FPS").into(),
                toggle_full_screen: language.text("进入全屏幕", "Enter Full Screen").into(),
                view: language.text("显示", "View").into(),
                window: language.text("窗口", "Window").into(),
            })
            .view_items_before_panels([MenuItem::action(
                language.text("重新载入当前笔记", "Reload Current Note"),
                ReloadCurrentNote,
            )
            .disabled(!can_reload_current_note)]),
    );
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
