rust_i18n::i18n!("examples/locales", fallback = "en");
use gpui_kit::component::{ActiveTheme, Theme};
use gpui_kit::{
    AppContext, Context, Div, FocusHandle, MouseButton, MouseDownEvent, MouseMoveEvent, Render,
    Window, div, prelude::*, px,
};
use oneo_gpui::{
    AppShell, AppShellState, AppearanceChange, AppearanceControls, AppearancePreferences,
    OpenSettings, SettingsShell, SettingsStore, ShellWindow, TITLEBAR_CONTROL_INSET,
    TITLEBAR_CONTROL_SIZE, ToggleLeftPanel, WorkspacePanel, WorkspaceShell, WorkspaceShellState,
    WorkspaceSidebar, panel_resize_handle, titlebar_content_left_padding, titlebar_drag_region,
    titlebar_left_control_offset,
};
use rust_i18n::t;
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Preferences {
    #[serde(flatten)]
    appearance: AppearancePreferences,
    greeting: Option<String>,
}

struct Welcome {
    preferences: Preferences,
    shell: AppShellState,
    appearance_controls: AppearanceControls,
    store: SettingsStore,
    error: Option<String>,
    settings_focus: FocusHandle,
    focus: FocusHandle,
}

impl Welcome {
    fn new(
        preferences: Preferences,
        store: SettingsStore,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        preferences.appearance.apply(window, cx);
        oneo_gpui::appearance::observe_appearance(window, cx, |this, _| {
            this.preferences.appearance
        })
        .detach();
        let appearance_controls =
            AppearanceControls::new(preferences.appearance, window, cx, Self::change_appearance);
        Self {
            preferences,
            appearance_controls,
            store,
            error: None,
            settings_focus: cx.focus_handle(),
            shell: AppShellState::default(),
            focus,
        }
    }

    fn toggle(&mut self, cx: &mut Context<Self>) {
        if self.shell.is_settings_open() {
            return;
        }
        self.shell.workspace_mut().toggle(cx.reduce_motion());
        cx.notify();
    }

    fn change_appearance(
        &mut self,
        change: AppearanceChange,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !change.apply(&mut self.preferences.appearance, window, cx) {
            return;
        }
        self.appearance_controls
            .sync(self.preferences.appearance, window, cx);
        self.error = self
            .store
            .save(&self.preferences)
            .err()
            .map(|error| format!("{error:#}"));
        cx.notify();
    }

    fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.shell.open_settings();
        window.focus(&self.settings_focus, cx);
        cx.notify();
    }

    fn close_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.shell.close_settings();
        window.focus(&self.focus, cx);
        cx.notify();
    }

    fn render_settings(&self, _window: &mut Window, cx: &mut Context<Self>) -> Div {
        let language = self.preferences.appearance.language;
        let theme = cx.theme().clone();
        let label = |key| oneo_gpui::appearance::label(key, language);
        let additional = div().when_some(self.error.as_ref(), |view, error| {
            view.child(
                div()
                    .debug_selector(|| "settings-save-error".to_owned())
                    .text_sm()
                    .text_color(theme.danger)
                    .child(format!("{}: {error}", label("shell.save_error"))),
            )
        });
        SettingsShell::appearance(&self.appearance_controls, language, additional, cx)
            .into_element()
            .track_focus(&self.settings_focus)
    }

    fn sidebar(&self, title: String, inset: f32, cx: &mut Context<Self>) -> Div {
        let empty = t!(
            "empty",
            locale = self.preferences.appearance.language.locale()
        )
        .into_owned();
        let id = "left-resize";
        let panel = WorkspaceSidebar::new(
            div().size_full().px_3().pb_3().child(empty),
            cx.theme(),
            inset,
        )
        .header_end(div().truncate().child(title))
        .into_element();
        panel.bg(cx.theme().sidebar).child(
            panel_resize_handle(id, self.shell.workspace().resize_active(), cx.theme())
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                        if event.click_count == 2 {
                            this.shell
                                .workspace_mut()
                                .reset_width(WorkspaceShellState::default().rendered_width());
                        } else {
                            this.shell
                                .workspace_mut()
                                .begin_resize(f32::from(event.position.x));
                        }
                        cx.stop_propagation();
                        cx.notify();
                    }),
                ),
        )
    }
}

fn pane(id: &'static str, title: String, content: &str, left_inset: f32, theme: &Theme) -> Div {
    WorkspacePanel::new(
        div()
            .size_full()
            .p_4()
            .overflow_hidden()
            .child(content.to_owned()),
    )
    .header(oneo_gpui::content_titlebar(
        div()
            .debug_selector(move || id.to_owned())
            .h_full()
            .border_b_1()
            .border_color(theme.border)
            .child(
                titlebar_drag_region(id)
                    .w_full()
                    .flex()
                    .items_center()
                    .pl(px(left_inset))
                    .pr_4()
                    .child(div().truncate().child(title)),
            ),
    ))
    .into_element()
    .bg(theme.background)
}

impl Render for Welcome {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let locale = self.preferences.appearance.language.locale();
        let frame = self.shell.workspace_mut().layout(window).frame;
        let offset = titlebar_left_control_offset(window.is_fullscreen());
        let clearance = offset + TITLEBAR_CONTROL_SIZE + TITLEBAR_CONTROL_INSET;
        let content_inset = f32::from(window.rem_size());
        let left = self.sidebar(
            t!("navigation", locale = locale).into_owned(),
            clearance,
            cx,
        );
        let content = pane(
            "content-titlebar",
            t!("welcome", locale = locale).into_owned(),
            self.preferences
                .greeting
                .as_deref()
                .unwrap_or(&t!("welcome", locale = locale)),
            titlebar_content_left_padding(clearance, frame.left, content_inset),
            cx.theme(),
        );
        let workspace = WorkspaceShell::new(
            frame,
            left.into_any_element(),
            content.into_any_element(),
            window,
            cx,
        )
        .into_element();

        let content = if self.shell.is_settings_open() {
            self.render_settings(window, cx)
        } else {
            workspace
        };
        div()
            .size_full()
            .text_color(cx.theme().foreground)
            .track_focus(&self.focus)
            .on_action(
                cx.listener(|this, _: &OpenSettings, window, cx| this.open_settings(window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &oneo_gpui::CloseSettings, window, cx| {
                    this.close_settings(window, cx)
                }),
            )
            .on_action(cx.listener(|this, _: &ToggleLeftPanel, _, cx| {
                this.toggle(cx);
            }))
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, window, cx| {
                if this.shell.workspace_mut().resize(
                    f32::from(event.position.x),
                    f32::from(window.viewport_size().width),
                ) {
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.shell.workspace_mut().finish_resize() {
                        cx.notify();
                    }
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.shell.workspace_mut().finish_resize() {
                        cx.notify();
                    }
                }),
            )
            .child(content)
    }
}

fn main() {
    AppShell::new().run(|cx| {
        let store = SettingsStore::for_project("oneo-example")?;
        let mut preferences: Preferences = store.load()?;
        preferences.appearance.normalize();
        ShellWindow::new("ONEO example").open(cx, |window, cx| {
            cx.new(|cx| Welcome::new(preferences, store, window, cx))
        })
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::component::Root;
    use gpui_kit::{KeyBinding, TestAppContext};
    use oneo_gpui::TITLEBAR_HEIGHT;

    fn add_example(
        preferences: Preferences,
        store: SettingsStore,
        cx: &mut TestAppContext,
    ) -> (gpui_kit::Entity<Welcome>, &mut gpui_kit::VisualTestContext) {
        let mut view = None;
        let (_, cx) = cx.add_window_view(|window, cx| {
            let welcome = cx.new(|cx| Welcome::new(preferences, store, window, cx));
            view = Some(welcome.clone());
            Root::new(welcome, window, cx)
        });
        (view.unwrap(), cx)
    }

    #[gpui_kit::test]
    fn headers_render_and_panel_action_reopens_sidebar(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_kit::component::init(cx);
            cx.set_reduce_motion(true);
            cx.bind_keys([KeyBinding::new("secondary-b", ToggleLeftPanel, None)]);
        });
        let directory = tempfile::tempdir().unwrap();
        let store = SettingsStore::at_path(directory.path().join("settings.toml")).unwrap();
        let (view, cx) = cx
            .add_window_view(|window, cx| Welcome::new(Preferences::default(), store, window, cx));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let left = cx
            .debug_bounds("workspace-sidebar-header")
            .expect("navigation titlebar");
        let content = cx
            .debug_bounds("content-titlebar")
            .expect("content titlebar");
        assert_eq!(left.origin.y, content.origin.y);
        for header in [left, content] {
            assert!(header.size.width > px(0.0));
            assert_eq!(header.size.height, px(TITLEBAR_HEIGHT));
        }
        let footer = cx.debug_bounds("workspace-sidebar-footer").unwrap();
        let settings = cx.debug_bounds("open-settings").unwrap();
        assert_eq!(
            footer.right() - settings.right(),
            footer.bottom() - settings.bottom()
        );
        assert!(left.right() <= content.left());
        cx.update(|window, _| assert_eq!(content.right(), window.viewport_size().width));

        #[cfg(target_os = "macos")]
        let shortcut = "cmd-b";
        #[cfg(not(target_os = "macos"))]
        let shortcut = "ctrl-b";
        cx.simulate_keystrokes(shortcut);
        assert!(!view.read_with(cx, |view, _| { view.shell.workspace().visible() }));
        cx.simulate_keystrokes(shortcut);
        assert!(view.read_with(cx, |view, _| { view.shell.workspace().visible() }));
        cx.dispatch_action(ToggleLeftPanel);
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("workspace-left-toggle").is_some());
        cx.dispatch_action(ToggleLeftPanel);
    }
    #[gpui_kit::test]
    fn settings_button_opens_shared_controls_and_returns_to_workspace(cx: &mut TestAppContext) {
        cx.update(gpui_kit::component::init);
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.toml");
        let store = SettingsStore::at_path(path.clone()).unwrap();
        let preferences = Preferences {
            appearance: AppearancePreferences {
                language: oneo_gpui::Language::English,
                ..AppearancePreferences::default()
            },
            ..Preferences::default()
        };
        let (view, cx) = add_example(preferences, store, cx);
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let footer = cx
            .debug_bounds("open-settings")
            .expect("settings button in sidebar");
        cx.simulate_click(footer.center(), Default::default());
        assert!(view.read_with(cx, |view, _| view.shell.is_settings_open()));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        for key in [
            "settings.appearance.theme",
            "settings.appearance.language",
            "settings.appearance.interface_font_size",
        ] {
            let bounds = cx.debug_bounds(key).expect("shared appearance selector");
            assert!(bounds.size.width > px(0.0));
        }
        let theme = cx.debug_bounds("settings.appearance.theme").unwrap();
        cx.simulate_click(theme.center(), Default::default());
        cx.simulate_keystrokes("down down enter");
        assert_eq!(
            view.read_with(cx, |view, _| view.preferences.appearance.theme),
            oneo_gpui::ThemePreference::Dark
        );
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let language = cx.debug_bounds("settings.appearance.language").unwrap();
        cx.simulate_click(language.center(), Default::default());
        cx.simulate_keystrokes("up enter");
        assert_eq!(
            view.read_with(cx, |view, _| view.preferences.appearance.language),
            oneo_gpui::Language::Chinese
        );
        assert_eq!(&*gpui_kit::component::locale(), "zh-CN");
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let font_size = cx
            .debug_bounds("settings.appearance.interface_font_size")
            .unwrap();
        cx.simulate_click(font_size.center(), Default::default());
        cx.simulate_keystrokes("down down down enter");
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(cx.theme().font_size, px(20.0));
            assert_eq!(window.rem_size(), px(20.0));
            assert!(cx.theme().is_dark());
        });
        let saved: Preferences = SettingsStore::at_path(path).unwrap().load().unwrap();
        assert_eq!(saved.appearance.theme, oneo_gpui::ThemePreference::Dark);
        assert_eq!(saved.appearance.language, oneo_gpui::Language::Chinese);
        assert_eq!(saved.appearance.ui_font_size, 20.0);
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let back = cx.debug_bounds("settings-back").unwrap();
        cx.simulate_click(back.center(), Default::default());
        cx.update(|window, cx| {
            let view = view.read(cx);
            assert!(!view.shell.is_settings_open());
            assert!(view.focus.is_focused(window));
            assert!(view.shell.workspace().visible());
        });
    }
    #[gpui_kit::test]
    fn failed_save_is_visible_and_retry_preserves_the_current_choice(cx: &mut TestAppContext) {
        cx.update(gpui_kit::component::init);
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.toml");
        std::fs::create_dir(&path).unwrap();
        let store = SettingsStore::at_path(path.clone()).unwrap();
        let (view, cx) = cx
            .add_window_view(|window, cx| Welcome::new(Preferences::default(), store, window, cx));
        view.update_in(cx, |view, window, cx| {
            view.open_settings(window, cx);
            view.change_appearance(AppearanceChange::FontSize(18.0), window, cx);
            assert!(view.error.is_some());
            assert_eq!(view.preferences.appearance.ui_font_size, 18.0);
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("settings-save-error").is_some());
        std::fs::remove_dir(&path).unwrap();
        view.update_in(cx, |view, window, cx| {
            view.change_appearance(AppearanceChange::FontSize(20.0), window, cx);
            assert!(view.error.is_none());
        });
        let saved: Preferences = SettingsStore::at_path(path).unwrap().load().unwrap();
        assert_eq!(saved.appearance.ui_font_size, 20.0);
    }
}
