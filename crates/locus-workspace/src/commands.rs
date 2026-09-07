use gpui_kit::{App, KeyBinding, MenuItem, actions};
use oneo_gpui::Language;
use oneo_gpui::{AppShellMenuLabels, AppShellMenus, set_app_shell_menus};
use rust_i18n::t;

const COMMAND_PALETTE_KEY: &str = "secondary-e";
const NEW_TERMINAL_KEY: &str = "secondary-`";

actions!(
    locus,
    [
        ArchiveCurrentNote,
        ChooseCurrentNoteIcon,
        CopyCurrentNoteReference,
        EditCurrentNote,
        NewTerminal,
        OpenCommandPalette,
        OpenCurrentNoteDirectory,
        ReloadCurrentNote
    ]
);

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new(COMMAND_PALETTE_KEY, OpenCommandPalette, None),
        KeyBinding::new(NEW_TERMINAL_KEY, NewTerminal, None),
        KeyBinding::new("secondary-r", ReloadCurrentNote, None),
    ]);
}

pub fn set_app_menus(cx: &mut App, language: Language, can_reload_current_note: bool) {
    set_app_shell_menus(
        cx,
        AppShellMenus::new(
            "Locus",
            AppShellMenuLabels {
                about: t!("menu.application.about", locale = language.locale())
                    .into_owned()
                    .into(),
                close_window: t!("menu.window.close", locale = language.locale())
                    .into_owned()
                    .into(),
                developer: t!("menu.developer.title", locale = language.locale())
                    .into_owned()
                    .into(),
                edit: t!("menu.edit.title", locale = language.locale())
                    .into_owned()
                    .into(),
                file: t!("menu.file.title", locale = language.locale())
                    .into_owned()
                    .into(),
                quit: t!("menu.application.quit", locale = language.locale())
                    .into_owned()
                    .into(),
                settings: t!("menu.application.settings", locale = language.locale())
                    .into_owned()
                    .into(),
                toggle_left_panel: t!("menu.view.toggle_left_sidebar", locale = language.locale())
                    .into_owned()
                    .into(),
                toggle_right_panel: t!(
                    "menu.view.toggle_right_sidebar",
                    locale = language.locale()
                )
                .into_owned()
                .into(),
                toggle_fps_counter: t!("menu.developer.show_fps", locale = language.locale())
                    .into_owned()
                    .into(),
                toggle_full_screen: t!("menu.view.enter_full_screen", locale = language.locale())
                    .into_owned()
                    .into(),
                view: t!("menu.view.title", locale = language.locale())
                    .into_owned()
                    .into(),
                window: t!("menu.window.title", locale = language.locale())
                    .into_owned()
                    .into(),
            },
        )
        .view_items_before_panels([
            MenuItem::action(
                t!("menu.view.command_palette", locale = language.locale()).into_owned(),
                OpenCommandPalette,
            ),
            MenuItem::action(
                t!("menu.view.reload_current_note", locale = language.locale()).into_owned(),
                ReloadCurrentNote,
            )
            .disabled(!can_reload_current_note),
        ]),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_terminal_shortcut_is_a_valid_key_binding() {
        KeyBinding::new(NEW_TERMINAL_KEY, NewTerminal, None);
    }

    #[test]
    fn command_palette_shortcut_is_a_valid_key_binding() {
        KeyBinding::new(COMMAND_PALETTE_KEY, OpenCommandPalette, None);
    }
}
