mod menu;

pub use menu::{
    About, AppShellMenuLabels, AppShellMenus, CloseSettings, CloseWindow, OpenSettings, Quit,
    ToggleFullScreen, ToggleLeftPanel, set_app_shell_menus,
};

use std::cell::Cell;
use std::rc::Rc;

use anyhow::Result;
use gpui_kit::component::Root;
use gpui_kit::{App, Application, AssetSource, WindowHandle};

use crate::window::{close_main_window, init_reduce_motion};
use menu::bind_app_shell_keys;

/// Owns framework initialization and reactivation of the application's main window.
pub struct AppShell {
    application: Application,
}

impl Default for AppShell {
    fn default() -> Self {
        Self::new()
    }
}

impl AppShell {
    pub fn new() -> Self {
        Self {
            application: gpui_kit::application().with_assets(gpui_kit::assets::Assets),
        }
    }

    pub fn with_assets(mut self, assets: impl AssetSource) -> Self {
        self.application = self.application.with_assets(assets);
        self
    }

    /// Initializes components and shell actions before calling the product initializer once.
    /// The returned main window is retained for OS reopen events; errors are logged before quitting.
    pub fn run(self, initialize: impl FnOnce(&mut App) -> Result<WindowHandle<Root>> + 'static) {
        let main_window = Rc::new(Cell::new(None::<WindowHandle<Root>>));
        let reopen_window = main_window.clone();
        self.application.on_reopen(move |cx| {
            if let Some(window) = reopen_window.get() {
                let _ = window.update(cx, |_, window, _| window.activate_window());
            }
            cx.activate(true);
        });
        self.application.run(move |cx| {
            gpui_kit::init(cx);
            #[cfg(debug_assertions)]
            gpui_kit::profiler::set_trace_enabled(true);
            init_reduce_motion(cx);
            bind_app_shell_keys(cx);
            bind_window_actions(cx);
            match initialize(cx) {
                Ok(window) => {
                    main_window.set(Some(window));
                    cx.activate(true);
                }
                Err(error) => {
                    eprintln!("Application initialization failed: {error:#}");
                    cx.quit();
                }
            }
        });
    }
}

fn bind_window_actions(cx: &mut App) {
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.on_action(|_: &CloseWindow, cx| {
        if let Some(window) = cx.active_window() {
            cx.defer(move |cx| {
                let _ = window.update(cx, |_, window, cx| close_main_window(window, cx));
            });
        }
    });
    cx.on_action(|_: &ToggleFullScreen, cx| {
        if let Some(window) = cx.active_window() {
            cx.defer(move |cx| {
                let _ = window.update(cx, |_, window, _| window.toggle_fullscreen());
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{Context, IntoElement, Render, TestAppContext, Window, div};

    struct Content;

    impl Render for Content {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    #[gpui_kit::test]
    fn kit_window_hosts_interactive_dialogs_without_manual_layers(cx: &mut TestAppContext) {
        use gpui_kit::component::WindowExt as _;
        use gpui_kit::test::{TestSupportExt as _, TestWindowExt as _};
        use gpui_kit::{
            AppContext as _, InteractiveElement as _, ParentElement as _,
            StatefulInteractiveElement as _, WindowOptions,
        };

        cx.update(gpui_kit::init);
        let (handle, content) = cx.update(|cx| {
            gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| cx.new(|_| Content))
                .unwrap()
        });
        let clicks = Rc::new(Cell::new(0));
        let observed = clicks.clone();
        handle
            .update(cx, |root, window, cx| {
                assert_eq!(
                    root.downcast::<Root>().unwrap().read(cx).view().entity_id(),
                    content.entity_id(),
                );
                window.open_dialog(cx, move |dialog, _, _| {
                    let clicks = observed.clone();
                    dialog.child(
                        div()
                            .id("dialog-action")
                            .child("Confirm")
                            .on_click(move |_, _, _| clicks.set(clicks.get() + 1))
                            .test_support(),
                    )
                });
                window.render_frame(cx);
                assert!(window.try_find("dialog-action").is_some());
                window.click("dialog-action", cx);
            })
            .unwrap();
        assert_eq!(clicks.get(), 1);
    }

    #[gpui_kit::test]
    fn fullscreen_shortcut_works_without_product_action_handlers(cx: &mut TestAppContext) {
        cx.update(|cx| {
            bind_app_shell_keys(cx);
            bind_window_actions(cx);
        });
        let window = cx.add_window(|_, _| Content);
        window
            .update(cx, |_, window, _| window.activate_window())
            .unwrap();
        #[cfg(target_os = "macos")]
        let shortcut = "ctrl-cmd-f";
        #[cfg(not(target_os = "macos"))]
        let shortcut = "f11";
        cx.simulate_keystrokes(window.into(), shortcut);
        assert!(
            window
                .update(cx, |_, window, _| window.is_fullscreen())
                .unwrap()
        );
        cx.simulate_keystrokes(window.into(), shortcut);
        assert!(
            !window
                .update(cx, |_, window, _| window.is_fullscreen())
                .unwrap()
        );
    }
}
