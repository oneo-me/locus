use std::fs;
use std::path::{Path, PathBuf};

use gpui::{
    App, Context, EventEmitter, FocusHandle, FontWeight, KeyDownEvent, MouseButton,
    PathPromptOptions, Render, Role, Stateful, StatefulInteractiveElement, Window, deferred, div,
    prelude::*, px,
};
use serde::{Deserialize, Serialize};

use crate::theme::{self, Theme, ThemePreference};
use crate::ui::{Icon, icon};

const FONT_SIZES: [f32; 6] = [14.0, 15.0, 16.0, 17.0, 18.0, 20.0];
const SETTINGS_SIDEBAR_WIDTH: f32 = 252.0;
const SETTINGS_CONTENT_MAX_WIDTH: f32 = 720.0;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Language {
    #[default]
    Chinese,
    English,
}

impl Language {
    pub const ALL: [Self; 2] = [Self::Chinese, Self::English];

    pub fn label(self) -> &'static str {
        match self {
            Self::Chinese => "中文",
            Self::English => "English",
        }
    }

    pub fn text(self, chinese: &'static str, english: &'static str) -> &'static str {
        match self {
            Self::Chinese => chinese,
            Self::English => english,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub library_path: Option<PathBuf>,
    pub theme: ThemePreference,
    pub language: Language,
    pub ui_font_size: f32,
    pub code_font_size: f32,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            library_path: None,
            theme: ThemePreference::System,
            language: Language::Chinese,
            ui_font_size: 16.0,
            code_font_size: 14.0,
        }
    }
}

impl Preferences {
    pub fn load() -> (Self, Option<String>) {
        let path = match settings_path() {
            Ok(path) => path,
            Err(error) => return (Self::default(), Some(error)),
        };
        if !path.exists() {
            return (Self::default(), None);
        }

        match fs::read(&path)
            .map_err(|error| error.to_string())
            .and_then(|bytes| {
                serde_json::from_slice::<Self>(&bytes).map_err(|error| error.to_string())
            }) {
            Ok(mut preferences) => {
                preferences.normalize();
                (preferences, None)
            }
            Err(error) => (Self::default(), Some(format!("无法读取设置：{error}"))),
        }
    }

    fn save(&self) -> Result<(), String> {
        let path = settings_path()?;
        let directory = path.parent().ok_or_else(|| "设置路径无效".to_string())?;
        fs::create_dir_all(directory).map_err(|error| error.to_string())?;
        let data = serde_json::to_vec_pretty(self).map_err(|error| error.to_string())?;
        fs::write(&path, data).map_err(|error| error.to_string())
    }

    fn normalize(&mut self) {
        let defaults = Self::default();
        if !FONT_SIZES.contains(&self.ui_font_size) {
            self.ui_font_size = defaults.ui_font_size;
        }
        if !FONT_SIZES.contains(&self.code_font_size) {
            self.code_font_size = defaults.code_font_size;
        }
    }
}

fn settings_path() -> Result<PathBuf, String> {
    dirs::home_dir()
        .map(|home| {
            home.join(".oneo")
                .join("locus_native")
                .join("settings.json")
        })
        .ok_or_else(|| "无法确定用户主目录".to_string())
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum SettingsPage {
    #[default]
    General,
    Appearance,
}

#[derive(Clone, Copy)]
enum Choice {
    Page(SettingsPage),
    Theme(ThemePreference),
    Language(Language),
    UiFont(f32),
    CodeFont(f32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Dropdown {
    Theme,
    Language,
    UiFont,
    CodeFont,
}

impl Dropdown {
    fn trigger_focus_index(self) -> usize {
        match self {
            Self::Theme => 4,
            Self::Language => 5,
            Self::UiFont => 6,
            Self::CodeFont => 7,
        }
    }
}

pub enum SettingsEvent {
    Close,
    LanguageChanged(Language),
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
                    this.preferences.library_path = Some(path);
                    this.persist();
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

    fn toggle_dropdown(&mut self, dropdown: Dropdown, window: &mut Window, cx: &mut Context<Self>) {
        if self.open_dropdown == Some(dropdown) {
            self.open_dropdown = None;
            window.focus(&self.controls[dropdown.trigger_focus_index()], cx);
        } else {
            self.open_dropdown = Some(dropdown);
            window.focus(&self.controls[8], cx);
        }
        cx.notify();
    }

    fn render_dropdown(
        &self,
        id: &'static str,
        label: &'static str,
        dropdown: Dropdown,
        choices: Vec<(&'static str, bool, Choice)>,
        trigger_focus_index: usize,
        cx: &mut Context<Self>,
    ) -> Stateful<gpui::Div> {
        let theme = Theme::current(cx);
        let open = self.open_dropdown == Some(dropdown);
        let trigger = div()
            .id(format!("{id}-trigger"))
            .role(Role::Button)
            .aria_label(label)
            .track_focus(&self.controls[trigger_focus_index])
            .tab_index(0)
            .tab_stop(true)
            .w(px(116.0))
            .h(px(30.0))
            .px(px(10.0))
            .rounded(px(7.0))
            .border_1()
            .border_color(theme.border_strong)
            .bg(if open {
                theme.control_selected
            } else {
                theme.raised
            })
            .flex()
            .items_center()
            .justify_between()
            .gap(px(6.0))
            .cursor_default()
            .text_sm()
            .text_color(theme.muted_text)
            .focus_visible(|style| style.border_color(theme.focus))
            .hover(|style| style.bg(theme.control_hover))
            .active(|style| style.bg(theme.control_active))
            .child(
                div()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .child(label),
            )
            .child(icon(Icon::ChevronDown, 10.5, theme.muted_text))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    this.toggle_dropdown(dropdown, window, cx);
                    cx.stop_propagation();
                }
            }))
            .on_click(cx.listener(move |this, _, window, cx| {
                this.toggle_dropdown(dropdown, window, cx);
                cx.stop_propagation();
            }));

        let menu = choices.into_iter().enumerate().fold(
            div()
                .id(format!("{id}-menu"))
                .absolute()
                .top(px(36.0))
                .right_0()
                .min_w(px(176.0))
                .p(px(4.0))
                .rounded(px(9.0))
                .border_1()
                .border_color(theme.border_strong)
                .bg(theme.raised)
                .shadow_lg()
                .occlude()
                .flex()
                .flex_col(),
            |menu, (index, (item_label, selected, choice))| {
                menu.child(
                    div()
                        .id(format!("{id}-option-{index}"))
                        .role(Role::Button)
                        .aria_label(item_label)
                        .track_focus(&self.controls[8 + index])
                        .tab_index(0)
                        .tab_stop(true)
                        .h(px(32.0))
                        .px(px(9.0))
                        .rounded(px(6.0))
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(10.0))
                        .cursor_default()
                        .text_sm()
                        .text_color(if selected {
                            theme.text
                        } else {
                            theme.muted_text
                        })
                        .hover(|style| style.bg(theme.control_hover))
                        .active(|style| style.bg(theme.control_active))
                        .focus_visible(|style| style.border_1().border_color(theme.focus))
                        .child(item_label)
                        .child(div().w(px(12.0)).when(selected, |mark| {
                            mark.child(icon(Icon::Check, 11.0, theme.muted_text))
                        }))
                        .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                this.apply_choice(choice, window, cx);
                                cx.stop_propagation();
                            }
                        }))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.apply_choice(choice, window, cx);
                            cx.stop_propagation();
                        })),
                )
            },
        );

        div()
            .id(id)
            .relative()
            .flex()
            .justify_end()
            .child(trigger)
            .when(open, |control| {
                control.child(deferred(menu).with_priority(1))
            })
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
                        .h(px(36.0))
                        .px(px(11.0))
                        .rounded(px(8.0))
                        .flex()
                        .items_center()
                        .cursor_default()
                        .text_sm()
                        .text_color(theme.muted_text)
                        .hover(|style| style.bg(theme.control_hover))
                        .active(|style| style.bg(theme.control_active))
                        .focus_visible(|style| style.border_1().border_color(theme.focus))
                        .gap(px(9.0))
                        .child(icon(Icon::ArrowLeft, 15.0, theme.muted_text))
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
                    .gap(px(4.0))
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
            .text_sm()
            .font_weight(if selected {
                FontWeight::MEDIUM
            } else {
                FontWeight::NORMAL
            })
            .text_color(if selected {
                theme.text
            } else {
                theme.muted_text
            })
            .when(selected, |row| row.bg(theme.control_selected))
            .hover(|style| style.bg(theme.control_hover))
            .active(|style| style.bg(theme.control_active))
            .focus_visible(|style| style.border_1().border_color(theme.focus))
            .child(icon(
                icon_kind,
                15.0,
                if selected {
                    theme.text
                } else {
                    theme.muted_text
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

    fn render_setting_row(
        &self,
        title: &'static str,
        description: &'static str,
        control: impl IntoElement,
        cx: &App,
    ) -> gpui::Div {
        let theme = Theme::current(cx);
        div()
            .min_h(px(68.0))
            .px(px(20.0))
            .py(px(13.0))
            .flex()
            .items_center()
            .gap(px(24.0))
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(title))
                    .child(
                        div()
                            .mt(px(4.0))
                            .text_xs()
                            .text_color(theme.muted_text)
                            .child(description),
                    ),
            )
            .child(control)
    }

    fn render_general(&self, cx: &mut Context<Self>) -> gpui::Div {
        let theme = Theme::current(cx);
        let language = self.preferences.language;
        let path = self
            .preferences
            .library_path
            .as_deref()
            .and_then(Path::to_str)
            .unwrap_or(language.text("未选择", "Not selected"));
        let choose_label = language.text("选择…", "Choose…");
        let path_control = div()
            .min_w_0()
            .w(px(320.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_sm()
                    .text_color(theme.muted_text)
                    .child(path.to_string()),
            )
            .child(
                div()
                    .id("choose-library")
                    .role(Role::Button)
                    .aria_label(choose_label)
                    .track_focus(&self.controls[3])
                    .tab_index(0)
                    .tab_stop(true)
                    .h(px(32.0))
                    .px(px(11.0))
                    .rounded(px(7.0))
                    .border_1()
                    .border_color(theme.border_strong)
                    .bg(theme.raised)
                    .flex()
                    .items_center()
                    .text_sm()
                    .cursor_default()
                    .hover(|style| style.bg(theme.control_hover))
                    .active(|style| style.bg(theme.control_active))
                    .focus_visible(|style| style.border_color(theme.focus))
                    .child(choose_label)
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            this.choose_library(cx);
                            cx.stop_propagation();
                        }
                    }))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(cx.listener(|this, _, _, cx| this.choose_library(cx))),
            );

        self.render_setting_row(
            language.text("文档库路径", "Library path"),
            language.text(
                "Markdown 笔记与附件所在的根目录",
                "Root folder containing Markdown notes and attachments",
            ),
            path_control,
            cx,
        )
    }

    fn render_appearance(&self, cx: &mut Context<Self>) -> gpui::Div {
        let theme = Theme::current(cx);
        let language = self.preferences.language;
        let theme_choices = [
            (language.text("跟随系统", "System"), ThemePreference::System),
            (language.text("亮色", "Light"), ThemePreference::Light),
            (language.text("暗色", "Dark"), ThemePreference::Dark),
        ]
        .into_iter()
        .map(|(label, value)| (label, value == self.preferences.theme, Choice::Theme(value)))
        .collect();
        let language_choices = Language::ALL
            .into_iter()
            .map(|value| (value.label(), value == language, Choice::Language(value)))
            .collect();
        let ui_font_choices = FONT_SIZES
            .into_iter()
            .map(|value| {
                (
                    font_size_label(value),
                    value == self.preferences.ui_font_size,
                    Choice::UiFont(value),
                )
            })
            .collect();
        let code_font_choices = FONT_SIZES
            .into_iter()
            .map(|value| {
                (
                    font_size_label(value),
                    value == self.preferences.code_font_size,
                    Choice::CodeFont(value),
                )
            })
            .collect();
        let selected_theme = match self.preferences.theme {
            ThemePreference::System => language.text("跟随系统", "System"),
            ThemePreference::Light => language.text("亮色", "Light"),
            ThemePreference::Dark => language.text("暗色", "Dark"),
        };

        div()
            .flex()
            .flex_col()
            .child(self.render_setting_row(
                language.text("主题", "Theme"),
                language.text(
                    "控制窗口和内容表面的明暗外观",
                    "Controls window and content appearance",
                ),
                self.render_dropdown(
                    "theme-selector",
                    selected_theme,
                    Dropdown::Theme,
                    theme_choices,
                    4,
                    cx,
                ),
                cx,
            ))
            .child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border))
            .child(self.render_setting_row(
                language.text("语言", "Language"),
                language.text(
                    "切换界面与系统菜单使用的语言",
                    "Language used by the interface and application menus",
                ),
                self.render_dropdown(
                    "language-selector",
                    language.label(),
                    Dropdown::Language,
                    language_choices,
                    5,
                    cx,
                ),
                cx,
            ))
            .child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border))
            .child(self.render_setting_row(
                language.text("界面字号", "Interface font size"),
                language.text(
                    "调整整个窗口的界面文字大小",
                    "Adjusts interface text throughout the window",
                ),
                self.render_dropdown(
                    "ui-font-size-selector",
                    font_size_label(self.preferences.ui_font_size),
                    Dropdown::UiFont,
                    ui_font_choices,
                    6,
                    cx,
                ),
                cx,
            ))
            .child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border))
            .child(self.render_setting_row(
                language.text("代码字号", "Code font size"),
                language.text("用于代码内容的文字大小", "Font size used for code content"),
                self.render_dropdown(
                    "code-font-size-selector",
                    font_size_label(self.preferences.code_font_size),
                    Dropdown::CodeFont,
                    code_font_choices,
                    7,
                    cx,
                ),
                cx,
            ))
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
            .border_color(theme.border_strong)
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
            .bg(theme.sidebar)
            .text_color(theme.text)
            .font_family(".SystemUIFont")
            .child(self.render_sidebar(cx))
            .child(self.render_content(cx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferences_round_trip() {
        let preferences = Preferences {
            library_path: Some(PathBuf::from("/tmp/笔记")),
            theme: ThemePreference::Dark,
            language: Language::English,
            ui_font_size: 18.0,
            code_font_size: 15.0,
        };
        let json = serde_json::to_string(&preferences).unwrap();
        let restored: Preferences = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.library_path, preferences.library_path);
        assert_eq!(restored.theme, preferences.theme);
        assert_eq!(restored.language, preferences.language);
        assert_eq!(restored.ui_font_size, preferences.ui_font_size);
        assert_eq!(restored.code_font_size, preferences.code_font_size);
    }

    #[test]
    fn unsupported_font_sizes_return_to_defaults() {
        let mut preferences = Preferences {
            ui_font_size: f32::NAN,
            code_font_size: 99.0,
            ..Preferences::default()
        };

        preferences.normalize();

        assert_eq!(preferences.ui_font_size, 16.0);
        assert_eq!(preferences.code_font_size, 14.0);
    }
}
