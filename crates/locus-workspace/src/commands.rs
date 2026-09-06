use gpui::{App, KeyBinding, MenuItem, actions};
use locus_preferences::localization::Language;
use oneo_gpui::{AppShellMenuLabels, AppShellMenus, set_app_shell_menus};

const COMMAND_PALETTE_KEY: &str = "secondary-e";
const NEW_TERMINAL_KEY: &str = "secondary-`";

actions!(
    locus,
    [
        ArchiveCurrentNote,
        ChooseCurrentNoteIcon,
        CloseTerminalTab,
        CopyCurrentNoteReference,
        EditCurrentNote,
        NewTerminal,
        OpenCommandPalette,
        OpenCurrentNoteDirectory,
        OpenKnowledgeGraph,
        ReloadCurrentNote
    ]
);

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new(COMMAND_PALETTE_KEY, OpenCommandPalette, None),
        KeyBinding::new("secondary-w", CloseTerminalTab, Some("Terminal")),
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
                about: language.text("menu.application.about").into(),
                close_window: language.text("menu.window.close").into(),
                developer: language.text("menu.developer.title").into(),
                edit: language.text("menu.edit.title").into(),
                file: language.text("menu.file.title").into(),
                quit: language.text("menu.application.quit").into(),
                settings: language.text("menu.application.settings").into(),
                toggle_left_panel: language.text("menu.view.toggle_left_sidebar").into(),
                toggle_right_panel: language.text("menu.view.toggle_right_sidebar").into(),
                toggle_fps_counter: language.text("menu.developer.show_fps").into(),
                toggle_full_screen: language.text("menu.view.enter_full_screen").into(),
                view: language.text("menu.view.title").into(),
                window: language.text("menu.window.title").into(),
            },
        )
        .view_items_before_panels([
            MenuItem::action(
                language.text("menu.view.command_palette"),
                OpenCommandPalette,
            ),
            MenuItem::action(
                language.text("menu.view.reload_current_note"),
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

    #[test]
    fn terminal_close_shortcut_is_a_valid_contextual_key_binding() {
        KeyBinding::new("secondary-w", CloseTerminalTab, Some("Terminal"));
    }
}
