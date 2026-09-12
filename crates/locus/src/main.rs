#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod assets;
mod platform;

use gpui_kit::{AppContext, px, size};
use locus_preferences::Preferences;
use locus_preferences::config::{WINDOW_HEIGHT, WINDOW_MIN_HEIGHT, WINDOW_MIN_WIDTH, WINDOW_WIDTH};
use locus_workspace::{InterfaceState, Locus, set_app_menus};
use oneo_gpui::{About, AppShell, ShellWindow};

fn main() {
    AppShell::new().with_assets(assets::Assets).run(|cx| {
        locus_workspace::bind_keys(cx);
        cx.on_action(|_: &About, _| platform::show_about_panel());
        let (preferences, load_error) = Preferences::load();
        let (interface_state, state_load_error) = InterfaceState::load();
        if let Some(error) = state_load_error {
            eprintln!("{error}");
        }
        let language = preferences.appearance.language;
        language.activate();
        let mut can_reload_current_note = false;
        let window = ShellWindow::new("Locus")
            .with_size(size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT)))
            .with_min_size(size(px(WINDOW_MIN_WIDTH), px(WINDOW_MIN_HEIGHT)))
            .with_saved_bounds(interface_state.window_bounds)
            .open(cx, |window, cx| {
                let view =
                    cx.new(|cx| Locus::new(window, preferences, load_error, interface_state, cx));
                can_reload_current_note = view.read(cx).can_reload_current_note();
                view
            })?;
        set_app_menus(cx, language, can_reload_current_note);
        Ok(window)
    });
}
