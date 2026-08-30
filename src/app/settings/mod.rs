use std::path::Path;

mod archive;
mod controls;
mod mcp;
mod pages;
pub(super) mod text_input;

use crate::config::{
    DEFAULT_CODE_FONT_FAMILY, DEFAULT_TERMINAL_FONT_FAMILY, DEFAULT_UI_FONT_FAMILY,
    WINDOW_TITLEBAR_HEIGHT, WINDOW_TRAFFIC_LIGHT_CLEARANCE,
};
use crate::document_library::ArchivedNote;
use crate::localization::Language;
use crate::mcp::McpServerState;
use crate::preferences::{
    EditorPreference, FONT_SIZES, Preferences, TerminalColorScheme, TerminalCursorStyle,
    terminal_color_scheme_label, terminal_cursor_style_label,
};
use crate::theme::{self, Theme, ThemePreference};
use crate::ui::{ActivationExt, Icon, MarkdownSelection, icon, selectable_code_text};
use gpui::{
    App, ClipboardItem, Context, Entity, EventEmitter, FocusHandle, Focusable, FontWeight,
    KeyDownEvent, MouseButton, PathPromptOptions, Render, Role, Stateful,
    StatefulInteractiveElement, Toggled, Window, deferred, div, prelude::*, px,
};
use oneo_gpui_ui::app_shell::{
    SettingsShell, settings_back_button, settings_navigation_group, settings_navigation_row,
    settings_sidebar_header,
};

const SETTINGS_SIDEBAR_WIDTH: f32 = 252.0;
const SETTINGS_CONTENT_MAX_WIDTH: f32 = 720.0;
const SETTINGS_TITLEBAR_CONTROL_INSET: f32 = 11.0;
const FOCUS_BACK: usize = 0;
const FOCUS_GENERAL_PAGE: usize = 1;
const FOCUS_APPEARANCE_PAGE: usize = 2;
const FOCUS_TERMINAL_PAGE: usize = 3;
const FOCUS_LIBRARY_PICKER: usize = 4;
const FOCUS_EDITOR: usize = 5;
const FOCUS_THEME: usize = 6;
const FOCUS_LANGUAGE: usize = 7;
const FOCUS_UI_FONT_FAMILY: usize = 8;
const FOCUS_UI_FONT_SIZE: usize = 9;
const FOCUS_CODE_FONT_FAMILY: usize = 10;
const FOCUS_CODE_FONT_SIZE: usize = 11;
const FOCUS_TERMINAL_FONT_FAMILY: usize = 12;
const FOCUS_TERMINAL_FONT_SIZE: usize = 13;
const FOCUS_TERMINAL_COLOR_SCHEME: usize = 14;
const FOCUS_TERMINAL_CURSOR_STYLE: usize = 15;
const FOCUS_MCP_PAGE: usize = 16;
const FOCUS_MCP_ENABLED: usize = 17;
const FOCUS_MCP_LOCAL_NETWORK: usize = 18;
const FOCUS_ARCHIVE_PAGE: usize = 19;
const FOCUS_MENU_START: usize = 20;
const FOCUS_MCP_PORT_APPLY: usize = 27;
const FOCUS_MCP_ENDPOINT_COPY: usize = 28;
const FOCUS_MCP_CODEX_TAB: usize = 29;
const FOCUS_MCP_CLAUDE_TAB: usize = 30;
const FOCUS_MCP_CURSOR_TAB: usize = 31;
const FOCUS_MCP_OPENCODE_TAB: usize = 32;
const FOCUS_MCP_GENERIC_TAB: usize = 33;
const FOCUS_MCP_GUIDE_COPY: usize = 34;
const FOCUS_MCP_GUIDE_CODE: usize = 35;

fn font_matches_search(family: &str, normalized_query: &str) -> bool {
    normalized_query.is_empty() || family.to_lowercase().contains(normalized_query)
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum SettingsPage {
    #[default]
    General,
    Archive,
    Appearance,
    Terminal,
    Mcp,
}

impl SettingsPage {
    fn label(self, language: Language) -> &'static str {
        match self {
            Self::General => language.text("通用", "General"),
            Self::Archive => language.text("归档", "Archive"),
            Self::Appearance => language.text("外观", "Appearance"),
            Self::Terminal => language.text("终端", "Terminal"),
            Self::Mcp => "MCP",
        }
    }
}

#[derive(Clone, Copy)]
enum Choice {
    Page(SettingsPage),
    Editor(EditorPreference),
    Theme(ThemePreference),
    Language(Language),
    UiFontSize(f32),
    CodeFontSize(f32),
    TerminalFontSize(f32),
    TerminalColorScheme(TerminalColorScheme),
    TerminalCursorStyle(TerminalCursorStyle),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Dropdown {
    Editor,
    Theme,
    Language,
    UiFontFamily,
    UiFontSize,
    CodeFontFamily,
    CodeFontSize,
    TerminalFontFamily,
    TerminalFontSize,
    TerminalColorScheme,
    TerminalCursorStyle,
}

impl Dropdown {
    fn trigger_focus_index(self) -> usize {
        match self {
            Self::Editor => FOCUS_EDITOR,
            Self::Theme => FOCUS_THEME,
            Self::Language => FOCUS_LANGUAGE,
            Self::UiFontFamily => FOCUS_UI_FONT_FAMILY,
            Self::UiFontSize => FOCUS_UI_FONT_SIZE,
            Self::CodeFontFamily => FOCUS_CODE_FONT_FAMILY,
            Self::CodeFontSize => FOCUS_CODE_FONT_SIZE,
            Self::TerminalFontFamily => FOCUS_TERMINAL_FONT_FAMILY,
            Self::TerminalFontSize => FOCUS_TERMINAL_FONT_SIZE,
            Self::TerminalColorScheme => FOCUS_TERMINAL_COLOR_SCHEME,
            Self::TerminalCursorStyle => FOCUS_TERMINAL_CURSOR_STYLE,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FontKind {
    Ui,
    Code,
    Terminal,
}

impl FontKind {
    fn dropdown(self) -> Dropdown {
        match self {
            Self::Ui => Dropdown::UiFontFamily,
            Self::Code => Dropdown::CodeFontFamily,
            Self::Terminal => Dropdown::TerminalFontFamily,
        }
    }

    fn focus_index(self) -> usize {
        self.dropdown().trigger_focus_index()
    }

    fn id(self) -> &'static str {
        match self {
            Self::Ui => "ui-font-family-selector",
            Self::Code => "code-font-family-selector",
            Self::Terminal => "terminal-font-family-selector",
        }
    }

    fn label(self, language: Language) -> &'static str {
        match self {
            Self::Ui => language.text("软件字体", "Interface font"),
            Self::Code => language.text("代码字体", "Code font"),
            Self::Terminal => language.text("终端字体", "Terminal font"),
        }
    }

    fn selected(self, preferences: &Preferences) -> &str {
        match self {
            Self::Ui => preferences.resolved_ui_font_family(),
            Self::Code => preferences.resolved_code_font_family(),
            Self::Terminal => preferences.resolved_terminal_font_family(),
        }
    }
}

pub enum SettingsEvent {
    Close,
    ArchiveRestored,
    LanguageChanged(Language),
    LibraryChanged(std::path::PathBuf),
    TerminalLaunchChanged,
    TerminalColorSchemeChanged(TerminalColorScheme),
    TerminalCursorStyleChanged(TerminalCursorStyle),
    UiFontChanged,
    CodeFontChanged,
    TerminalFontChanged(String, f32),
    McpConfigurationChanged,
}

pub struct Settings {
    focus: FocusHandle,
    return_focus: FocusHandle,
    controls: Vec<FocusHandle>,
    page: SettingsPage,
    open_dropdown: Option<Dropdown>,
    preferences: Preferences,
    font_names: Vec<String>,
    font_option_focus: Vec<FocusHandle>,
    font_search: String,
    font_search_input: Entity<text_input::TextInput>,
    terminal_shell_input: Entity<text_input::TextInput>,
    terminal_command_input: Entity<text_input::TextInput>,
    mcp_port_input: Entity<text_input::TextInput>,
    mcp_port_draft: String,
    mcp_port_error: Option<String>,
    mcp_server_state: McpServerState,
    mcp_copied: Option<&'static str>,
    mcp_client: mcp::McpClient,
    mcp_code_selection: MarkdownSelection,
    archived_notes: Vec<ArchivedNote>,
    archive_action_focus: Vec<FocusHandle>,
    archive_load_error: Option<String>,
    archive_action_error: Option<String>,
    restoring_archive: Option<std::path::PathBuf>,
    error: Option<String>,
    titlebar_drag_armed: bool,
}

impl EventEmitter<SettingsEvent> for Settings {}

impl Settings {
    pub fn new(
        preferences: Preferences,
        load_error: Option<String>,
        mcp_server_state: McpServerState,
        archived_notes: Vec<ArchivedNote>,
        archive_load_error: Option<String>,
        return_focus: FocusHandle,
        cx: &mut Context<Self>,
    ) -> Self {
        let language = preferences.language;
        let mut font_names = cx.text_system().all_font_names();
        for family in [
            DEFAULT_UI_FONT_FAMILY,
            DEFAULT_CODE_FONT_FAMILY,
            DEFAULT_TERMINAL_FONT_FAMILY,
            preferences.resolved_ui_font_family(),
            preferences.resolved_code_font_family(),
            preferences.resolved_terminal_font_family(),
        ] {
            if !font_names.iter().any(|font| font == family) {
                font_names.push(family.to_owned());
            }
        }
        font_names.sort_unstable_by_key(|font| font.to_lowercase());
        let font_option_focus = font_names.iter().map(|_| cx.focus_handle()).collect();
        let archive_action_focus = archived_notes.iter().map(|_| cx.focus_handle()).collect();
        let font_search_input = cx.new(|cx| {
            text_input::TextInput::new(
                "font-search-input",
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
            &font_search_input,
            |this, _, event: &text_input::TextInputEvent, cx| {
                let text_input::TextInputEvent::Changed(value) = event;
                this.font_search = value.clone();
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
            controls: (0..36).map(|_| cx.focus_handle()).collect(),
            page: SettingsPage::General,
            open_dropdown: None,
            preferences,
            font_names,
            font_option_focus,
            font_search: String::new(),
            font_search_input,
            terminal_shell_input,
            terminal_command_input,
            mcp_port_input,
            mcp_port_draft,
            mcp_port_error: None,
            mcp_server_state,
            mcp_copied: None,
            mcp_client: mcp::McpClient::default(),
            mcp_code_selection: MarkdownSelection::default(),
            archived_notes,
            archive_action_focus,
            archive_load_error,
            archive_action_error: None,
            restoring_archive: None,
            error: load_error,
            titlebar_drag_armed: false,
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

    pub fn set_archive_state(
        &mut self,
        archived_notes: Vec<ArchivedNote>,
        error: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.archive_action_error = None;
        if self.restoring_archive.as_ref().is_some_and(|pending| {
            archived_notes
                .iter()
                .all(|note| &note.document_path != pending)
        }) {
            self.restoring_archive = None;
        }
        self.archive_action_focus = archived_notes
            .iter()
            .map(|note| {
                self.archived_notes
                    .iter()
                    .position(|current| current.path == note.path)
                    .and_then(|index| self.archive_action_focus.get(index).cloned())
                    .unwrap_or_else(|| cx.focus_handle())
            })
            .collect();
        self.archived_notes = archived_notes;
        self.archive_load_error = error;
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
                self.font_search_input.update(cx, |input, cx| {
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
            Choice::UiFontSize(size) => {
                if self.preferences.ui_font_size == size {
                    return;
                }
                self.preferences.ui_font_size = size;
                window.set_rem_size(px(size));
                window.refresh();
                cx.emit(SettingsEvent::UiFontChanged);
            }
            Choice::CodeFontSize(size) => {
                if self.preferences.code_font_size == size {
                    return;
                }
                self.preferences.code_font_size = size;
                cx.emit(SettingsEvent::CodeFontChanged);
            }
            Choice::TerminalFontSize(size) => {
                if self.preferences.terminal_font_size == size {
                    return;
                }
                self.preferences.terminal_font_size = size;
                cx.emit(SettingsEvent::TerminalFontChanged(
                    self.preferences.resolved_terminal_font_family().to_owned(),
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

    fn render_titlebar_drag_region(
        &self,
        id: &'static str,
        cx: &mut Context<Self>,
    ) -> Stateful<gpui::Div> {
        div()
            .id(id)
            .h_full()
            .on_click(|event, window, _| {
                if event.click_count() == 2 {
                    window.titlebar_double_click();
                }
            })
            .on_mouse_down_out(cx.listener(|this, _, _, _| {
                this.titlebar_drag_armed = false;
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, _| {
                    this.titlebar_drag_armed = true;
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| {
                    this.titlebar_drag_armed = false;
                }),
            )
            .on_mouse_move(cx.listener(|this, _, window, _| {
                if this.titlebar_drag_armed {
                    this.titlebar_drag_armed = false;
                    window.start_window_move();
                }
            }))
    }

    fn render_sidebar_header(&self, fullscreen: bool, cx: &mut Context<Self>) -> gpui::Div {
        let theme = Theme::current(cx);
        let language = self.preferences.language;
        let back_label = language.text("返回", "Back");
        let traffic_light_clearance = if cfg!(target_os = "macos") && !fullscreen {
            WINDOW_TRAFFIC_LIGHT_CLEARANCE
        } else {
            SETTINGS_TITLEBAR_CONTROL_INSET
        };

        settings_sidebar_header(
            traffic_light_clearance,
            self.render_titlebar_drag_region("settings-sidebar-traffic-light-drag-region", cx)
                .into_any_element(),
            settings_back_button(
                "settings-back",
                back_label,
                &self.controls[FOCUS_BACK],
                theme,
            )
            .on_activation(cx, |this, window, cx| this.close(window, cx))
            .into_any_element(),
        )
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> Stateful<gpui::Div> {
        let language = self.preferences.language;
        let theme = Theme::current(cx);
        div()
            .id("settings-sidebar-scroll")
            .size_full()
            .overflow_y_scroll()
            .px(px(12.0))
            .pt(px(12.0))
            .pb(px(20.0))
            .flex()
            .flex_col()
            .gap(px(14.0))
            .child(settings_navigation_group(
                language.text("应用", "Application"),
                vec![
                    self.render_navigation_row(
                        "settings-general",
                        SettingsPage::General.label(language),
                        Icon::Settings,
                        SettingsPage::General,
                        FOCUS_GENERAL_PAGE,
                        cx,
                    ),
                    self.render_navigation_row(
                        "settings-appearance",
                        SettingsPage::Appearance.label(language),
                        Icon::Appearance,
                        SettingsPage::Appearance,
                        FOCUS_APPEARANCE_PAGE,
                        cx,
                    ),
                    self.render_navigation_row(
                        "settings-terminal",
                        SettingsPage::Terminal.label(language),
                        Icon::Terminal,
                        SettingsPage::Terminal,
                        FOCUS_TERMINAL_PAGE,
                        cx,
                    ),
                ],
                theme,
            ))
            .child(settings_navigation_group(
                language.text("文档库", "Library"),
                vec![self.render_navigation_row(
                    "settings-archive",
                    SettingsPage::Archive.label(language),
                    Icon::Archive,
                    SettingsPage::Archive,
                    FOCUS_ARCHIVE_PAGE,
                    cx,
                )],
                theme,
            ))
            .child(settings_navigation_group(
                language.text("连接", "Connections"),
                vec![self.render_navigation_row(
                    "settings-mcp",
                    SettingsPage::Mcp.label(language),
                    Icon::Plug,
                    SettingsPage::Mcp,
                    FOCUS_MCP_PAGE,
                    cx,
                )],
                theme,
            ))
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
        settings_navigation_row(
            id,
            label,
            icon_kind,
            &self.controls[focus_index],
            selected,
            theme,
        )
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

    fn render_content_header(&self, cx: &mut Context<Self>) -> Stateful<gpui::Div> {
        let title = self.page.label(self.preferences.language);
        self.render_titlebar_drag_region("settings-content-titlebar", cx)
            .size_full()
            .px(px(32.0))
            .flex()
            .items_center()
            .text_sm()
            .font_weight(FontWeight::SEMIBOLD)
            .child(title)
    }

    fn render_content(&self, cx: &mut Context<Self>) -> Stateful<gpui::Div> {
        let theme = Theme::current(cx);
        let content = match self.page {
            SettingsPage::General => self.render_general(cx),
            SettingsPage::Archive => self.render_archive(cx),
            SettingsPage::Appearance => self.render_appearance(cx),
            SettingsPage::Terminal => self.render_terminal(cx),
            SettingsPage::Mcp => self.render_mcp(cx),
        };

        div()
            .id("settings-content-scroll")
            .size_full()
            .overflow_y_scroll()
            .px(px(32.0))
            .pb(px(48.0))
            .child(
                div()
                    .w_full()
                    .max_w(px(SETTINGS_CONTENT_MAX_WIDTH))
                    .mx_auto()
                    .pb(px(190.0))
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
            )
    }
}

fn font_size_label(value: f32) -> &'static str {
    match value as u32 {
        12 => "12",
        14 => "14",
        15 => "15",
        16 => "16",
        17 => "17",
        18 => "18",
        20 => "20",
        _ => unreachable!("unsupported font size"),
    }
}

impl Render for Settings {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
            .font_family(self.preferences.resolved_ui_font_family().to_owned())
            .child(
                SettingsShell::new(
                    self.render_sidebar_header(window.is_fullscreen(), cx)
                        .into_any_element(),
                    self.render_sidebar(cx).into_any_element(),
                    self.render_content_header(cx).into_any_element(),
                    self.render_content(cx).into_any_element(),
                    theme,
                )
                .sidebar_width(SETTINGS_SIDEBAR_WIDTH)
                .titlebar_height(WINDOW_TITLEBAR_HEIGHT)
                .into_element(),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setting_control_focus_ranges_do_not_overlap() {
        let persistent_indices = [
            FOCUS_BACK,
            FOCUS_GENERAL_PAGE,
            FOCUS_APPEARANCE_PAGE,
            FOCUS_TERMINAL_PAGE,
            FOCUS_LIBRARY_PICKER,
            FOCUS_EDITOR,
            FOCUS_THEME,
            FOCUS_LANGUAGE,
            FOCUS_UI_FONT_FAMILY,
            FOCUS_UI_FONT_SIZE,
            FOCUS_CODE_FONT_FAMILY,
            FOCUS_CODE_FONT_SIZE,
            FOCUS_TERMINAL_FONT_FAMILY,
            FOCUS_TERMINAL_FONT_SIZE,
            FOCUS_TERMINAL_COLOR_SCHEME,
            FOCUS_TERMINAL_CURSOR_STYLE,
            FOCUS_MCP_PAGE,
            FOCUS_MCP_ENABLED,
            FOCUS_MCP_LOCAL_NETWORK,
            FOCUS_MCP_PORT_APPLY,
            FOCUS_MCP_ENDPOINT_COPY,
            FOCUS_MCP_CODEX_TAB,
            FOCUS_MCP_CLAUDE_TAB,
            FOCUS_MCP_CURSOR_TAB,
            FOCUS_MCP_OPENCODE_TAB,
            FOCUS_MCP_GENERIC_TAB,
            FOCUS_MCP_GUIDE_COPY,
            FOCUS_MCP_GUIDE_CODE,
            FOCUS_ARCHIVE_PAGE,
        ];
        let mut unique = std::collections::HashSet::new();

        for index in persistent_indices {
            assert!(unique.insert(index));
        }
        for index in FOCUS_MENU_START..FOCUS_MENU_START + FONT_SIZES.len() {
            assert!(unique.insert(index));
        }
    }

    #[test]
    fn font_search_is_case_insensitive_and_matches_substrings() {
        assert!(font_matches_search("JetBrains Mono", "brains"));
        assert!(font_matches_search("SF Mono", "sf mono"));
        assert!(!font_matches_search("Menlo", "serif"));
    }

    #[test]
    fn every_supported_font_size_has_its_own_label() {
        assert_eq!(
            FONT_SIZES.map(font_size_label),
            ["12", "14", "15", "16", "17", "18", "20"]
        );
    }
}
