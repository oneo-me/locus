use std::path::Path;

mod controls;
mod pages;

use crate::config::DEFAULT_UI_FONT_SIZE;
use crate::localization::Language;
use crate::preferences::{EditorPreference, FONT_SIZES, Preferences};
use crate::theme::{self, Theme, ThemePreference};
use crate::ui::{ActivationExt, Icon, icon};
use gpui::{
    App, Context, EventEmitter, FocusHandle, FontWeight, KeyDownEvent, MouseButton,
    PathPromptOptions, Rems, Render, Role, Stateful, StatefulInteractiveElement, Window, deferred,
    div, prelude::*, px, rems,
};

const SETTINGS_SIDEBAR_WIDTH: f32 = 252.0;
const SETTINGS_CONTENT_MAX_WIDTH: f32 = 720.0;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum SettingsPage {
    #[default]
    General,
    Appearance,
}

#[derive(Clone, Copy)]
enum Choice {
    Page(SettingsPage),
    Editor(EditorPreference),
    Theme(ThemePreference),
    Language(Language),
    UiFont(f32),
    CodeFont(f32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Dropdown {
    Editor,
    Theme,
    Language,
    UiFont,
    CodeFont,
}

impl Dropdown {
    fn trigger_focus_index(self) -> usize {
        match self {
            Self::Editor => 4,
            Self::Theme => 5,
            Self::Language => 6,
            Self::UiFont => 7,
            Self::CodeFont => 8,
        }
    }
}

pub enum SettingsEvent {
    Close,
    LanguageChanged(Language),
    LibraryChanged(std::path::PathBuf),
}

pub struct Settings {
    focus: FocusHandle,
    return_focus: FocusHandle,
    controls: Vec<FocusHandle>,
    page: SettingsPage,
    open_dropdown: Option<Dropdown>,
    preferences: Preferences,
    error: Option<String>,
}

impl EventEmitter<SettingsEvent> for Settings {}

impl Settings {
    pub fn new(
        preferences: Preferences,
        load_error: Option<String>,
        return_focus: FocusHandle,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            focus: cx.focus_handle(),
            return_focus,
            controls: (0..24).map(|_| cx.focus_handle()).collect(),
            page: SettingsPage::General,
            open_dropdown: None,
            preferences,
            error: load_error,
        }
    }

    pub fn preferences(&self) -> &Preferences {
        &self.preferences
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        window.focus(&self.focus, cx);
    }

    fn persist(&mut self) {
        let language = self.preferences.language;
        self.error = self.preferences.save().err().map(|error| {
            format!(
                "{}：{error}",
                language.text("无法保存设置", "Could not save settings")
            )
        });
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.return_focus, cx);
        cx.emit(SettingsEvent::Close);
    }

    fn choose_library(&mut self, cx: &mut Context<Self>) {
        let language = self.preferences.language;
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(language.text("选择文档库", "Choose Library").into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = receiver.await
                && let Some(path) = paths.into_iter().next()
            {
                let _ = this.update(cx, |this, cx| {
                    this.preferences.library_path = Some(path.clone());
                    this.persist();
                    cx.emit(SettingsEvent::LibraryChanged(path));
                    cx.notify();
                });
            }
        })
        .detach();
    }

    fn apply_choice(&mut self, choice: Choice, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(dropdown) = self.open_dropdown.take() {
            window.focus(&self.controls[dropdown.trigger_focus_index()], cx);
        }
        cx.notify();
        match choice {
            Choice::Page(page) => {
                self.page = page;
                cx.notify();
                return;
            }
            Choice::Editor(editor) => {
                if self.preferences.editor == editor {
                    return;
                }
                self.preferences.editor = editor;
            }
            Choice::Theme(preference) => {
                if self.preferences.theme == preference {
                    return;
                }
                self.preferences.theme = preference;
                theme::apply_preference(preference, window, cx);
            }
            Choice::Language(language) => {
                if self.preferences.language == language {
                    return;
                }
                self.preferences.language = language;
                cx.emit(SettingsEvent::LanguageChanged(language));
            }
            Choice::UiFont(size) => {
                if self.preferences.ui_font_size == size {
                    return;
                }
                self.preferences.ui_font_size = size;
                window.set_rem_size(px(size));
                window.refresh();
            }
            Choice::CodeFont(size) => {
                if self.preferences.code_font_size == size {
                    return;
                }
                self.preferences.code_font_size = size;
            }
        }
        self.persist();
        cx.notify();
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> gpui::Div {
        let theme = Theme::current(cx);
        let language = self.preferences.language;
        let back_label = language.text("返回", "Back");

        div()
            .w(px(SETTINGS_SIDEBAR_WIDTH))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .child(div().h(px(48.0)).flex_none())
            .child(
                div().px(px(12.0)).child(
                    div()
                        .id("settings-back")
                        .role(Role::Button)
                        .aria_label(back_label)
                        .track_focus(&self.controls[0])
                        .tab_index(0)
                        .tab_stop(true)
                        .h(px(34.0))
                        .px(px(11.0))
                        .rounded(px(8.0))
                        .flex()
                        .items_center()
                        .cursor_default()
                        .text_size(scaled_ui_pixels(13.0))
                        .text_color(theme.text_secondary)
                        .hover(|style| style.bg(theme.control_hover))
                        .active(|style| style.bg(theme.control_active))
                        .focus_visible(|style| style.border_1().border_color(theme.focus))
                        .gap(px(9.0))
                        .child(icon(Icon::ArrowLeft, 15.0, theme.text_tertiary))
                        .child(back_label)
                        .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                this.close(window, cx);
                                cx.stop_propagation();
                            }
                        }))
                        .on_click(cx.listener(|this, _, window, cx| this.close(window, cx))),
                ),
            )
            .child(div().h(px(18.0)))
            .child(
                div()
                    .px(px(12.0))
                    .flex()
                    .flex_col()
                    .gap(px(3.0))
                    .child(self.render_navigation_row(
                        "settings-general",
                        language.text("通用", "General"),
                        Icon::Settings,
                        SettingsPage::General,
                        1,
                        cx,
                    ))
                    .child(self.render_navigation_row(
                        "settings-appearance",
                        language.text("外观", "Appearance"),
                        Icon::Appearance,
                        SettingsPage::Appearance,
                        2,
                        cx,
                    )),
            )
    }

    fn render_navigation_row(
        &self,
        id: &'static str,
        label: &'static str,
        icon_kind: Icon,
        page: SettingsPage,
        focus_index: usize,
        cx: &mut Context<Self>,
    ) -> Stateful<gpui::Div> {
        let theme = Theme::current(cx);
        let selected = self.page == page;
        div()
            .id(id)
            .role(Role::Button)
            .aria_label(label)
            .track_focus(&self.controls[focus_index])
            .tab_index(0)
            .tab_stop(true)
            .h(px(36.0))
            .px(px(11.0))
            .rounded(px(8.0))
            .flex()
            .items_center()
            .gap(px(10.0))
            .cursor_default()
            .text_size(scaled_ui_pixels(13.0))
            .text_color(if selected {
                theme.text
            } else {
                theme.text_secondary
            })
            .when(selected, |row| row.bg(theme.control_selected))
            .hover(|style| style.bg(theme.control_selected))
            .active(|style| style.bg(theme.control_selected))
            .focus_visible(|style| style.border_1().border_color(theme.focus))
            .child(icon(
                icon_kind,
                15.0,
                if selected {
                    theme.text_secondary
                } else {
                    theme.text_tertiary
                },
            ))
            .child(label)
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    this.apply_choice(Choice::Page(page), window, cx);
                    cx.stop_propagation();
                }
            }))
            .on_click(cx.listener(move |this, _, window, cx| {
                this.apply_choice(Choice::Page(page), window, cx);
            }))
    }

    fn render_content(&self, cx: &mut Context<Self>) -> gpui::Div {
        let theme = Theme::current(cx);
        let language = self.preferences.language;
        let title = match self.page {
            SettingsPage::General => language.text("通用", "General"),
            SettingsPage::Appearance => language.text("外观", "Appearance"),
        };
        let content = match self.page {
            SettingsPage::General => self.render_general(cx),
            SettingsPage::Appearance => self.render_appearance(cx),
        };

        div()
            .min_w_0()
            .h_full()
            .flex_1()
            .flex()
            .flex_col()
            .border_l_1()
            .border_color(theme.sidebar_border)
            .bg(theme.surface)
            .child(div().h(px(48.0)).flex_none())
            .child(
                div()
                    .id("settings-content-scroll")
                    .min_h_0()
                    .flex_1()
                    .overflow_y_scroll()
                    .px(px(32.0))
                    .pb(px(48.0))
                    .child(
                        div()
                            .w_full()
                            .max_w(px(SETTINGS_CONTENT_MAX_WIDTH))
                            .mx_auto()
                            .pb(px(190.0))
                            .child(div().text_lg().font_weight(FontWeight::MEDIUM).child(title))
                            .when_some(self.error.as_ref(), |view, error| {
                                view.child(
                                    div()
                                        .mt(px(14.0))
                                        .rounded(px(8.0))
                                        .border_1()
                                        .border_color(theme.error)
                                        .px(px(12.0))
                                        .py(px(9.0))
                                        .text_sm()
                                        .text_color(theme.error)
                                        .child(error.clone()),
                                )
                            })
                            .child(
                                div()
                                    .mt(px(15.0))
                                    .rounded(px(13.0))
                                    .bg(theme.raised)
                                    .child(content),
                            ),
                    ),
            )
    }
}

fn font_size_label(value: f32) -> &'static str {
    match value as u32 {
        14 => "14",
        15 => "15",
        16 => "16",
        17 => "17",
        18 => "18",
        _ => "20",
    }
}

fn scaled_ui_pixels(value: f32) -> Rems {
    rems(value / DEFAULT_UI_FONT_SIZE)
}

impl Render for Settings {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::current(cx);
        div()
            .key_context("Settings")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape"
                    && let Some(dropdown) = this.open_dropdown.take()
                {
                    window.focus(&this.controls[dropdown.trigger_focus_index()], cx);
                    cx.notify();
                    cx.stop_propagation();
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.open_dropdown.take().is_some() {
                        cx.notify();
                    }
                }),
            )
            .size_full()
            .flex()
            .bg(theme.surface)
            .text_color(theme.text)
            .font_family(".SystemUIFont")
            .child(self.render_sidebar(cx))
            .child(self.render_content(cx))
    }
}
