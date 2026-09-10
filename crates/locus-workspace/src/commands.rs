use gpui_kit::{App, KeyBinding, MenuItem, actions};
use oneo_gpui::Language;
use oneo_gpui::{AppShellMenuLabels, AppShellMenus, set_app_shell_menus};
use rust_i18n::t;

const COMMAND_PALETTE_KEY: &str = "secondary-e";
const NEW_TERMINAL_KEY: &str = "secondary-`";

pub(crate) fn capture_terminal_shortcut(
    event: &gpui_kit::KeyDownEvent,
    window: &mut gpui_kit::Window,
    cx: &mut App,
) {
    if KeyBinding::new(NEW_TERMINAL_KEY, NewTerminal, None)
        .match_keystrokes(std::slice::from_ref(&event.keystroke))
        == Some(false)
    {
        cx.stop_propagation();
        if !event.is_held {
            window.dispatch_action(Box::new(NewTerminal), cx);
        }
    }
}

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
        KeyBinding::new("secondary-r", ReloadCurrentNote, None),
        KeyBinding::new("f5", ReloadCurrentNote, None),
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

    #[gpui_kit::test]
    fn terminal_shortcut_dispatches_once_per_press(cx: &mut gpui_kit::TestAppContext) {
        use gpui_kit::{Context, FocusHandle, Render, Window, div, prelude::*};
        struct ShortcutProbe {
            focus: FocusHandle,
            launches: usize,
        }
        impl Render for ShortcutProbe {
            fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
                div()
                    .size_full()
                    .track_focus(&self.focus)
                    .capture_key_down(capture_terminal_shortcut)
                    .on_action(cx.listener(|this, _: &NewTerminal, _, _| this.launches += 1))
            }
        }
        let (view, cx) = cx.add_window_view(|window, cx| {
            let focus = cx.focus_handle();
            window.focus(&focus, cx);
            ShortcutProbe { focus, launches: 0 }
        });
        for (held, expected) in [(false, 1), (true, 1), (true, 1), (false, 2)] {
            cx.update(|window, cx| {
                window.dispatch_event(
                    gpui_kit::PlatformInput::KeyDown(gpui_kit::KeyDownEvent {
                        keystroke: gpui_kit::Keystroke::parse(NEW_TERMINAL_KEY).unwrap(),
                        is_held: held,
                        prefer_character_input: false,
                    }),
                    cx,
                );
            });
            cx.run_until_parked();
            view.read_with(cx, |view, _| assert_eq!(view.launches, expected));
        }
    }
}
