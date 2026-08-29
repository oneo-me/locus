mod app;
mod config;
mod document_library;
mod localization;
mod motion;
mod platform;
mod preferences;
mod state;
mod storage;
mod theme;
mod ui;

use std::cell::RefCell;
use std::rc::Rc;

use app::Locus;
use config::{WINDOW_HEIGHT, WINDOW_MIN_HEIGHT, WINDOW_MIN_WIDTH, WINDOW_WIDTH};
use gpui::{
    App, AppContext, Bounds, KeyBinding, Menu, MenuItem, TitlebarOptions,
    WindowBackgroundAppearance, WindowBounds, WindowOptions, actions, point, px, size,
};
use gpui_platform::application;
use localization::Language;
use preferences::Preferences;
use state::InterfaceState;

actions!(
    locus,
    [
        Quit,
        About,
        CloseWindow,
        OpenSettings,
        ReloadCurrentNote,
        ToggleLeftSidebar,
        ToggleRightSidebar,
        ToggleFpsCounter
    ]
);

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
        platform::init_reduce_motion(cx);
        cx.bind_keys([
            KeyBinding::new("secondary-q", Quit, None),
            KeyBinding::new("secondary-w", CloseWindow, None),
            KeyBinding::new("secondary-,", OpenSettings, None),
            KeyBinding::new("secondary-r", ReloadCurrentNote, None),
            KeyBinding::new("secondary-b", ToggleLeftSidebar, None),
            KeyBinding::new("secondary-shift-b", ToggleRightSidebar, None),
            KeyBinding::new("secondary-alt-shift-f", ToggleFpsCounter, None),
        ]);
        cx.on_action(|_: &About, _| platform::show_about_panel());
        cx.on_action(|_: &Quit, cx| cx.quit());
        let (preferences, load_error) = Preferences::load();
        let (interface_state, state_load_error) = InterfaceState::load();
        if let Some(error) = state_load_error {
            eprintln!("{error}");
        }
        let language = preferences.language;

        let bounds = Bounds::centered(None, size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT)), cx);

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
                    platform::configure_main_window_close_behavior(window, cx);
                    platform::configure_sidebar_material(window)
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

pub(crate) fn set_app_menus(cx: &mut App, language: Language, can_reload_current_note: bool) {
    let mut application_items = Vec::new();
    #[cfg(target_os = "macos")]
    application_items.extend([
        MenuItem::action(language.text("关于 Locus", "About Locus"), About),
        MenuItem::separator(),
    ]);
    application_items.extend([
        MenuItem::action(language.text("设置…", "Settings…"), OpenSettings),
        MenuItem::separator(),
        MenuItem::action(language.text("退出 Locus", "Quit Locus"), Quit),
    ]);

    cx.set_menus(vec![
        Menu::new("Locus").items(application_items),
        Menu::new(language.text("编辑", "Edit")).disabled(true),
        Menu::new(language.text("显示", "View")).items([
            MenuItem::action(
                language.text("重新载入当前笔记", "Reload Current Note"),
                ReloadCurrentNote,
            )
            .disabled(!can_reload_current_note),
            MenuItem::separator(),
            MenuItem::action(
                language.text("切换左侧栏", "Toggle Left Sidebar"),
                ToggleLeftSidebar,
            ),
            MenuItem::action(
                language.text("切换右侧栏", "Toggle Right Sidebar"),
                ToggleRightSidebar,
            ),
            MenuItem::separator(),
            MenuItem::action(language.text("显示帧率", "Show FPS"), ToggleFpsCounter),
        ]),
        Menu::new(language.text("窗口", "Window")).items([MenuItem::action(
            language.text("关闭窗口", "Close Window"),
            CloseWindow,
        )]),
    ]);
}
