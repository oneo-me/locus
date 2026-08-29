use std::path::Path;

mod controls;
mod mcp;
mod pages;
pub(super) mod text_input;

use crate::config::{DEFAULT_CODE_FONT_FAMILY, DEFAULT_UI_FONT_SIZE};
use crate::localization::Language;
use crate::mcp::McpServerState;
use crate::preferences::{
    EditorPreference, FONT_SIZES, Preferences, TerminalColorScheme, TerminalCursorStyle,
};
use crate::theme::{self, Theme, ThemePreference};
use crate::ui::{ActivationExt, Icon, icon};
use gpui::{
    App, ClipboardItem, Context, Entity, EventEmitter, FocusHandle, Focusable, FontWeight,
    KeyDownEvent, MouseButton, PathPromptOptions, Rems, Render, Role, Stateful,
    StatefulInteractiveElement, Toggled, Window, deferred, div, prelude::*, px, rems,
};

const SETTINGS_SIDEBAR_WIDTH: f32 = 252.0;
const SETTINGS_CONTENT_MAX_WIDTH: f32 = 720.0;
const FOCUS_BACK: usize = 0;
const FOCUS_GENERAL_PAGE: usize = 1;
const FOCUS_APPEARANCE_PAGE: usize = 2;
const FOCUS_TERMINAL_PAGE: usize = 3;
const FOCUS_LIBRARY_PICKER: usize = 4;
const FOCUS_EDITOR: usize = 5;
const FOCUS_THEME: usize = 6;
const FOCUS_LANGUAGE: usize = 7;
const FOCUS_UI_FONT: usize = 8;
const FOCUS_CODE_FONT: usize = 9;
const FOCUS_CODE_FONT_FAMILY: usize = 10;
const FOCUS_TERMINAL_COLOR_SCHEME: usize = 11;
const FOCUS_TERMINAL_CURSOR_STYLE: usize = 12;
const FOCUS_MCP_PAGE: usize = 13;
const FOCUS_MCP_ENABLED: usize = 14;
const FOCUS_MCP_LOCAL_NETWORK: usize = 15;
const FOCUS_MCP_PORT_APPLY: usize = 22;
const FOCUS_MCP_ENDPOINT_COPY: usize = 23;
const FOCUS_MCP_CODEX_COPY: usize = 24;
const FOCUS_MCP_CLAUDE_COPY: usize = 25;
const FOCUS_MCP_CURSOR_COPY: usize = 26;
const FOCUS_MCP_OPENCODE_COPY: usize = 27;
const FOCUS_MCP_GENERIC_COPY: usize = 28;

fn font_matches_search(family: &str, normalized_query: &str) -> bool {
    normalized_query.is_empty() || family.to_lowercase().contains(normalized_query)
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum SettingsPage {
    #[default]
    General,
    Appearance,
    Terminal,
    Mcp,
}

#[derive(Clone, Copy)]
enum Choice {
    Page(SettingsPage),
    Editor(EditorPreference),
    Theme(ThemePreference),
    Language(Language),
    UiFont(f32),
    CodeFont(f32),
    TerminalColorScheme(TerminalColorScheme),
    TerminalCursorStyle(TerminalCursorStyle),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Dropdown {
    Editor,
    Theme,
    Language,
    UiFont,
    CodeFont,
    CodeFontFamily,
    TerminalColorScheme,
    TerminalCursorStyle,
}

impl Dropdown {
    fn trigger_focus_index(self) -> usize {
        match self {
            Self::Editor => FOCUS_EDITOR,
            Self::Theme => FOCUS_THEME,
            Self::Language => FOCUS_LANGUAGE,
            Self::UiFont => FOCUS_UI_FONT,
            Self::CodeFont => FOCUS_CODE_FONT,
            Self::CodeFontFamily => FOCUS_CODE_FONT_FAMILY,
            Self::TerminalColorScheme => FOCUS_TERMINAL_COLOR_SCHEME,
            Self::TerminalCursorStyle => FOCUS_TERMINAL_CURSOR_STYLE,
        }
    }
}

pub enum SettingsEvent {
    Close,
    LanguageChanged(Language),
    LibraryChanged(std::path::PathBuf),
    TerminalLaunchChanged,
    TerminalColorSchemeChanged(TerminalColorScheme),
    TerminalCursorStyleChanged(TerminalCursorStyle),
    CodeFontChanged(String, f32),
    McpConfigurationChanged,
}

pub struct Settings {
    focus: FocusHandle,
    return_focus: FocusHandle,
    controls: Vec<FocusHandle>,
    page: SettingsPage,
    open_dropdown: Option<Dropdown>,
    preferences: Preferences,
    code_font_names: Vec<String>,
    code_font_option_focus: Vec<FocusHandle>,
    code_font_search: String,
    code_font_search_input: Entity<text_input::TextInput>,
    terminal_shell_input: Entity<text_input::TextInput>,
    terminal_command_input: Entity<text_input::TextInput>,
    mcp_port_input: Entity<text_input::TextInput>,
    mcp_port_draft: String,
    mcp_port_error: Option<String>,
    mcp_server_state: McpServerState,
    mcp_copied: Option<&'static str>,
    error: Option<String>,
}

impl EventEmitter<SettingsEvent> for Settings {}

impl Settings {
    pub fn new(
        preferences: Preferences,
        load_error: Option<String>,
        mcp_server_state: McpServerState,
        return_focus: FocusHandle,
        cx: &mut Context<Self>,
    ) -> Self {
        let language = preferences.language;
        let mut code_font_names = cx.text_system().all_font_names();
        if !code_font_names
            .iter()
            .any(|font| font == DEFAULT_CODE_FONT_FAMILY)
        {
            code_font_names.push(DEFAULT_CODE_FONT_FAMILY.to_owned());
            code_font_names.sort_unstable_by_key(|font| font.to_lowercase());
        }
        let code_font_option_focus = code_font_names.iter().map(|_| cx.focus_handle()).collect();
        let code_font_search_input = cx.new(|cx| {
            text_input::TextInput::new(
                "code-font-search-input",
                String::new(),
                language.text("搜索字体", "Search fonts"),
                language.text("搜索字体", "Search fonts"),
                cx,
            )
        });
        let terminal_shell_input = cx.new(|cx| {
            text_input::TextInput::new(
                "terminal-shell-input",
                preferences.terminal_shell.clone(),
                language.text("启动 Shell", "Launch shell"),
                language.text("使用系统登录 Shell", "Use system login shell"),
                cx,
            )
        });
        let terminal_command_input = cx.new(|cx| {
            text_input::TextInput::new(
                "terminal-command-input",
                preferences.terminal_command.clone(),
                language.text("默认命令", "Default command"),
                language.text("不执行默认命令", "No default command"),
                cx,
            )
        });
        let mcp_port_draft = preferences.mcp_port.to_string();
        let mcp_port_input = cx.new(|cx| {
            text_input::TextInput::new(
                "mcp-port-input",
                mcp_port_draft.clone(),
                language.text("MCP 端口", "MCP port"),
                language.text("1 到 65535", "1 to 65535"),
                cx,
            )
            .with_width(px(164.0))
        });
        cx.subscribe(
            &code_font_search_input,
            |this, _, event: &text_input::TextInputEvent, cx| {
                let text_input::TextInputEvent::Changed(value) = event;
                this.code_font_search = value.clone();
                cx.notify();
            },
        )
        .detach();
        cx.subscribe(
            &terminal_shell_input,
            |this, _, event: &text_input::TextInputEvent, cx| {
                let text_input::TextInputEvent::Changed(value) = event;
                this.preferences.terminal_shell = value.clone();
                this.persist();
                cx.emit(SettingsEvent::TerminalLaunchChanged);
            },
        )
        .detach();
        cx.subscribe(
            &terminal_command_input,
            |this, _, event: &text_input::TextInputEvent, cx| {
                let text_input::TextInputEvent::Changed(value) = event;
                this.preferences.terminal_command = value.clone();
                this.persist();
                cx.emit(SettingsEvent::TerminalLaunchChanged);
            },
        )
        .detach();
        cx.subscribe(
            &mcp_port_input,
            |this, _, event: &text_input::TextInputEvent, cx| {
                let text_input::TextInputEvent::Changed(value) = event;
                this.mcp_port_draft = value.clone();
                this.mcp_port_error = None;
                cx.notify();
            },
        )
        .detach();
        Self {
            focus: cx.focus_handle(),
            return_focus,
            controls: (0..32).map(|_| cx.focus_handle()).collect(),
            page: SettingsPage::General,
            open_dropdown: None,
            preferences,
            code_font_names,
            code_font_option_focus,
            code_font_search: String::new(),
            code_font_search_input,
            terminal_shell_input,
            terminal_command_input,
            mcp_port_input,
            mcp_port_draft,
            mcp_port_error: None,
            mcp_server_state,
            mcp_copied: None,
            error: load_error,
        }
    }

    pub fn preferences(&self) -> &Preferences {
        &self.preferences
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        window.focus(&self.focus, cx);
    }

    pub fn set_mcp_server_state(&mut self, state: McpServerState, cx: &mut Context<Self>) {
        self.mcp_server_state = state;
        cx.notify();
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
                self.code_font_search_input.update(cx, |input, cx| {
                    input.set_label(language.text("搜索字体", "Search fonts"), cx);
                    input.set_placeholder(language.text("搜索字体", "Search fonts"), cx);
                });
                self.terminal_shell_input.update(cx, |input, cx| {
                    input.set_label(language.text("启动 Shell", "Launch shell"), cx);
                    input.set_placeholder(
                        language.text("使用系统登录 Shell", "Use system login shell"),
                        cx,
                    );
                });
                self.terminal_command_input.update(cx, |input, cx| {
                    input.set_label(language.text("默认命令", "Default command"), cx);
                    input
                        .set_placeholder(language.text("不执行默认命令", "No default command"), cx);
                });
                self.mcp_port_input.update(cx, |input, cx| {
                    input.set_label(language.text("MCP 端口", "MCP port"), cx);
                    input.set_placeholder(language.text("1 到 65535", "1 to 65535"), cx);
                });
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
                cx.emit(SettingsEvent::CodeFontChanged(
                    self.preferences.resolved_code_font_family().to_owned(),
                    size,
                ));
            }
            Choice::TerminalColorScheme(color_scheme) => {
                if self.preferences.terminal_color_scheme == color_scheme {
                    return;
                }
                self.preferences.terminal_color_scheme = color_scheme;
                cx.emit(SettingsEvent::TerminalColorSchemeChanged(color_scheme));
            }
            Choice::TerminalCursorStyle(cursor_style) => {
                if self.preferences.terminal_cursor_style == cursor_style {
                    return;
                }
                self.preferences.terminal_cursor_style = cursor_style;
                cx.emit(SettingsEvent::TerminalCursorStyleChanged(cursor_style));
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
                        .track_focus(&self.controls[FOCUS_BACK])
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
                        FOCUS_GENERAL_PAGE,
                        cx,
                    ))
                    .child(self.render_navigation_row(
                        "settings-appearance",
                        language.text("外观", "Appearance"),
                        Icon::Appearance,
                        SettingsPage::Appearance,
                        FOCUS_APPEARANCE_PAGE,
                        cx,
                    ))
                    .child(self.render_navigation_row(
                        "settings-terminal",
                        language.text("终端", "Terminal"),
                        Icon::Terminal,
                        SettingsPage::Terminal,
                        FOCUS_TERMINAL_PAGE,
                        cx,
                    ))
                    .child(self.render_navigation_row(
                        "settings-mcp",
                        "MCP",
                        Icon::Plug,
                        SettingsPage::Mcp,
                        FOCUS_MCP_PAGE,
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
            SettingsPage::Terminal => language.text("终端", "Terminal"),
            SettingsPage::Mcp => "MCP",
        };
        let content = match self.page {
            SettingsPage::General => self.render_general(cx),
            SettingsPage::Appearance => self.render_appearance(cx),
            SettingsPage::Terminal => self.render_terminal(cx),
            SettingsPage::Mcp => self.render_mcp(cx),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persistent_setting_controls_have_distinct_focus_handles() {
        let indices = [
            FOCUS_BACK,
            FOCUS_GENERAL_PAGE,
            FOCUS_APPEARANCE_PAGE,
            FOCUS_TERMINAL_PAGE,
            FOCUS_LIBRARY_PICKER,
            FOCUS_EDITOR,
            FOCUS_THEME,
            FOCUS_LANGUAGE,
            FOCUS_UI_FONT,
            FOCUS_CODE_FONT,
            FOCUS_CODE_FONT_FAMILY,
            FOCUS_TERMINAL_COLOR_SCHEME,
            FOCUS_TERMINAL_CURSOR_STYLE,
            FOCUS_MCP_PAGE,
            FOCUS_MCP_ENABLED,
            FOCUS_MCP_LOCAL_NETWORK,
            FOCUS_MCP_PORT_APPLY,
            FOCUS_MCP_ENDPOINT_COPY,
            FOCUS_MCP_CODEX_COPY,
            FOCUS_MCP_CLAUDE_COPY,
            FOCUS_MCP_CURSOR_COPY,
            FOCUS_MCP_OPENCODE_COPY,
            FOCUS_MCP_GENERIC_COPY,
        ];
        let unique = indices
            .into_iter()
            .collect::<std::collections::HashSet<_>>();

        assert_eq!(unique.len(), indices.len());
    }

    #[test]
    fn font_search_is_case_insensitive_and_matches_substrings() {
        assert!(font_matches_search("JetBrains Mono", "brains"));
        assert!(font_matches_search("SF Mono", "sf mono"));
        assert!(!font_matches_search("Menlo", "serif"));
    }
}
