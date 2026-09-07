rust_i18n::i18n!("locales", fallback = "en");

use rust_i18n::t;
use std::path::Path;

mod archive;
mod controls;
mod mcp;
mod pages;

use gpui::{
    App, ClipboardItem, Context, Entity, EventEmitter, FocusHandle, FontWeight, MouseButton,
    PathPromptOptions, Render, Role, Stateful, StatefulInteractiveElement, Window, div, prelude::*,
    px,
};
use gpui_component::{
    IndexPath,
    input::{Input, InputEvent, InputState},
    select::{SearchableVec, SelectEvent, SelectItem, SelectState},
};
use locus_library::ArchivedNote;
use locus_mcp::McpServerState;
use locus_preferences::config::{
    DEFAULT_CODE_FONT_FAMILY, DEFAULT_LEFT_SIDEBAR_WIDTH, DEFAULT_TERMINAL_FONT_FAMILY,
    DEFAULT_UI_FONT_FAMILY, WINDOW_TITLEBAR_HEIGHT, WINDOW_TRAFFIC_LIGHT_CLEARANCE,
};
use locus_preferences::localization::Language;
use locus_preferences::theme::{self, Theme, ThemePreference};
use locus_preferences::{
    EditorPreference, FONT_SIZES, Preferences, TerminalColorScheme, TerminalCursorStyle,
    terminal_color_scheme_label, terminal_cursor_style_label,
};
use locus_ui::{Icon, MarkdownSelection, SelectableCodeStyle, icon, selectable_code_text};
use oneo_gpui::{
    SettingsShell, settings_back_button, settings_navigation_group, settings_navigation_row,
    settings_sidebar_header,
};

fn ui_rem(value: f32) -> gpui::Rems {
    gpui::rems(value / 16.0)
}

const SETTINGS_CONTENT_MAX_WIDTH: f32 = 720.0;
const SETTINGS_CONTROL_WIDTH_REM: f32 = 20.0;
const SETTINGS_TITLEBAR_CONTROL_INSET: f32 = 11.0;

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
    fn label(self, language: Language) -> String {
        match self {
            Self::General => t!("settings.page.general", locale = language.locale()).into_owned(),
            Self::Archive => t!("settings.page.archive", locale = language.locale()).into_owned(),
            Self::Appearance => {
                t!("settings.page.appearance", locale = language.locale()).into_owned()
            }
            Self::Terminal => t!("settings.page.terminal", locale = language.locale()).into_owned(),
            Self::Mcp => "MCP".to_owned(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
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

impl Choice {
    fn label(self, language: Language) -> String {
        match self {
            Self::Page(page) => page.label(language),
            Self::Editor(editor) => editor.label(language),
            Self::Theme(ThemePreference::System) => t!(
                "settings.appearance.theme.system",
                locale = language.locale()
            )
            .into_owned(),
            Self::Theme(ThemePreference::Light) => t!(
                "settings.appearance.theme.light",
                locale = language.locale()
            )
            .into_owned(),
            Self::Theme(ThemePreference::Dark) => {
                t!("settings.appearance.theme.dark", locale = language.locale()).into_owned()
            }
            Self::Language(language) => language.label(),
            Self::UiFontSize(size) | Self::CodeFontSize(size) | Self::TerminalFontSize(size) => {
                font_size_label(size).to_owned()
            }
            Self::TerminalColorScheme(scheme) => terminal_color_scheme_label(scheme, language),
            Self::TerminalCursorStyle(style) => terminal_cursor_style_label(style, language),
        }
    }
}

fn choice_items(dropdown: Dropdown, language: Language) -> Vec<ChoiceItem> {
    dropdown
        .choices()
        .into_iter()
        .map(|choice| ChoiceItem {
            label: choice.label(language).into(),
            value: choice,
        })
        .collect()
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum Dropdown {
    Editor,
    Theme,
    Language,
    UiFontSize,
    CodeFontSize,
    TerminalFontSize,
    TerminalColorScheme,
    TerminalCursorStyle,
}

impl Dropdown {
    const ALL: [Self; 8] = [
        Self::Editor,
        Self::Theme,
        Self::Language,
        Self::UiFontSize,
        Self::CodeFontSize,
        Self::TerminalFontSize,
        Self::TerminalColorScheme,
        Self::TerminalCursorStyle,
    ];

    fn label(self, language: Language) -> String {
        let key = match self {
            Self::Editor => "settings.editor.label",
            Self::Theme => "settings.appearance.theme.label",
            Self::Language => "settings.appearance.language.label",
            Self::UiFontSize => "settings.appearance.interface_font_size.label",
            Self::CodeFontSize => "settings.appearance.code_font_size.label",
            Self::TerminalFontSize => "settings.terminal.font_size.label",
            Self::TerminalColorScheme => "settings.terminal.color_scheme.label",
            Self::TerminalCursorStyle => "settings.terminal.cursor_style.label",
        };
        t!(key, locale = language.locale()).into_owned()
    }

    fn choices(self) -> Vec<Choice> {
        match self {
            Self::Editor => EditorPreference::ALL
                .into_iter()
                .map(Choice::Editor)
                .collect(),
            Self::Theme => [
                ThemePreference::System,
                ThemePreference::Light,
                ThemePreference::Dark,
            ]
            .into_iter()
            .map(Choice::Theme)
            .collect(),
            Self::Language => Language::ALL.into_iter().map(Choice::Language).collect(),
            Self::UiFontSize => FONT_SIZES.into_iter().map(Choice::UiFontSize).collect(),
            Self::CodeFontSize => FONT_SIZES.into_iter().map(Choice::CodeFontSize).collect(),
            Self::TerminalFontSize => FONT_SIZES
                .into_iter()
                .map(Choice::TerminalFontSize)
                .collect(),
            Self::TerminalColorScheme => TerminalColorScheme::ALL
                .into_iter()
                .map(Choice::TerminalColorScheme)
                .collect(),
            Self::TerminalCursorStyle => TerminalCursorStyle::ALL
                .into_iter()
                .map(Choice::TerminalCursorStyle)
                .collect(),
        }
    }

    fn selected(self, preferences: &Preferences) -> Choice {
        match self {
            Self::Editor => Choice::Editor(preferences.editor),
            Self::Theme => Choice::Theme(preferences.theme),
            Self::Language => Choice::Language(preferences.language),
            Self::UiFontSize => Choice::UiFontSize(preferences.ui_font_size),
            Self::CodeFontSize => Choice::CodeFontSize(preferences.code_font_size),
            Self::TerminalFontSize => Choice::TerminalFontSize(preferences.terminal_font_size),
            Self::TerminalColorScheme => {
                Choice::TerminalColorScheme(preferences.terminal_color_scheme)
            }
            Self::TerminalCursorStyle => {
                Choice::TerminalCursorStyle(preferences.terminal_cursor_style)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum FontKind {
    Ui,
    Code,
    Terminal,
}

#[derive(Clone)]
struct ChoiceItem {
    label: gpui::SharedString,
    value: Choice,
}

impl SelectItem for ChoiceItem {
    type Value = Choice;

    fn title(&self) -> gpui::SharedString {
        self.label.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.value
    }
}

#[derive(Clone)]
struct FontItem {
    family: gpui::SharedString,
}

impl SelectItem for FontItem {
    type Value = gpui::SharedString;

    fn title(&self) -> gpui::SharedString {
        self.family.clone()
    }

    fn display_title(&self) -> Option<gpui::AnyElement> {
        Some(
            div()
                .min_w_0()
                .truncate()
                .font_family(self.family.clone())
                .child(self.family.clone())
                .into_any_element(),
        )
    }

    fn value(&self) -> &Self::Value {
        &self.family
    }
}

type ChoiceSelect = SelectState<Vec<ChoiceItem>>;
type FontSelect = SelectState<SearchableVec<FontItem>>;

impl FontKind {
    fn label(self, language: Language) -> String {
        match self {
            Self::Ui => t!(
                "settings.appearance.interface_font.label",
                locale = language.locale()
            )
            .into_owned(),
            Self::Code => t!(
                "settings.appearance.code_font.label",
                locale = language.locale()
            )
            .into_owned(),
            Self::Terminal => {
                t!("settings.terminal.font.label", locale = language.locale()).into_owned()
            }
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
    mcp_code_focus: FocusHandle,
    page: SettingsPage,
    choice_selects: std::collections::HashMap<Dropdown, Entity<ChoiceSelect>>,
    font_selects: std::collections::HashMap<FontKind, Entity<FontSelect>>,
    preferences: Preferences,
    terminal_shell_input: Entity<InputState>,
    terminal_command_input: Entity<InputState>,
    mcp_port_input: Entity<InputState>,
    mcp_port_draft: String,
    mcp_port_error: Option<String>,
    mcp_server_state: McpServerState,
    mcp_copied: Option<&'static str>,
    mcp_client: mcp::McpClient,
    mcp_code_selection: MarkdownSelection,
    archived_notes: Vec<ArchivedNote>,
    archive_load_error: Option<String>,
    archive_action_error: Option<String>,
    restoring_archive: Option<std::path::PathBuf>,
    error: Option<String>,
    titlebar_drag_armed: bool,
}

impl EventEmitter<SettingsEvent> for Settings {}

pub struct SettingsInit {
    pub preferences: Preferences,
    pub load_error: Option<String>,
    pub mcp_server_state: McpServerState,
    pub archived_notes: Vec<ArchivedNote>,
    pub archive_load_error: Option<String>,
}

impl Settings {
    pub fn new(
        window: &mut Window,
        init: SettingsInit,
        return_focus: FocusHandle,
        cx: &mut Context<Self>,
    ) -> Self {
        let SettingsInit {
            preferences,
            load_error,
            mcp_server_state,
            archived_notes,
            archive_load_error,
        } = init;
        let language = preferences.language;
        let settings_error = load_error.map(|error| {
            eprintln!("{error}");
            t!("settings.load_error", locale = language.locale()).into_owned()
        });
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
        let mut choice_selects = std::collections::HashMap::new();
        for dropdown in Dropdown::ALL {
            let selected = dropdown.selected(&preferences);
            let items = choice_items(dropdown, language);
            let selected_index = items
                .iter()
                .position(|item| item.value == selected)
                .map(|ix| IndexPath::default().row(ix));
            let select = cx.new(|cx| SelectState::new(items, selected_index, window, cx));
            cx.subscribe_in(
                &select,
                window,
                |this, _, event: &SelectEvent<Vec<ChoiceItem>>, window, cx| {
                    let SelectEvent::Confirm(Some(choice)) = event else {
                        return;
                    };
                    this.apply_choice(*choice, window, cx);
                },
            )
            .detach();
            choice_selects.insert(dropdown, select);
        }

        let font_items = font_names
            .iter()
            .cloned()
            .map(|family| FontItem {
                family: family.into(),
            })
            .collect::<Vec<_>>();
        let mut font_selects = std::collections::HashMap::new();
        for kind in [FontKind::Ui, FontKind::Code, FontKind::Terminal] {
            let selected = kind.selected(&preferences);
            let selected_index = font_items
                .iter()
                .position(|item| item.family.as_ref() == selected)
                .map(|ix| IndexPath::default().row(ix));
            let select = cx.new(|cx| {
                SelectState::new(
                    SearchableVec::new(font_items.clone()),
                    selected_index,
                    window,
                    cx,
                )
                .searchable(true)
            });
            cx.subscribe_in(
                &select,
                window,
                move |this, _, event: &SelectEvent<SearchableVec<FontItem>>, window, cx| {
                    let SelectEvent::Confirm(Some(family)) = event else {
                        return;
                    };
                    this.choose_font(kind, family.to_string(), window, cx);
                },
            )
            .detach();
            font_selects.insert(kind, select);
        }
        let terminal_shell_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(preferences.terminal_shell.clone())
                .placeholder(
                    t!(
                        "settings.terminal.shell.placeholder",
                        locale = language.locale()
                    )
                    .into_owned(),
                )
        });
        let terminal_command_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(preferences.terminal_command.clone())
                .placeholder(
                    t!(
                        "settings.terminal.command.placeholder",
                        locale = language.locale()
                    )
                    .into_owned(),
                )
        });
        let mcp_port_draft = preferences.mcp_port.to_string();
        let mcp_port_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(mcp_port_draft.clone())
                .placeholder(
                    t!("settings.mcp.port.placeholder", locale = language.locale()).into_owned(),
                )
        });
        cx.subscribe(
            &terminal_shell_input,
            |this, input, event: &InputEvent, cx| {
                if !matches!(event, InputEvent::Change) {
                    return;
                }
                this.preferences.terminal_shell = input.read(cx).value().to_string();
                this.persist();
                cx.emit(SettingsEvent::TerminalLaunchChanged);
            },
        )
        .detach();
        cx.subscribe(
            &terminal_command_input,
            |this, input, event: &InputEvent, cx| {
                if !matches!(event, InputEvent::Change) {
                    return;
                }
                this.preferences.terminal_command = input.read(cx).value().to_string();
                this.persist();
                cx.emit(SettingsEvent::TerminalLaunchChanged);
            },
        )
        .detach();
        cx.subscribe(&mcp_port_input, |this, input, event: &InputEvent, cx| {
            if !matches!(event, InputEvent::Change) {
                return;
            }
            this.mcp_port_draft = input.read(cx).value().to_string();
            this.mcp_port_error = None;
            cx.notify();
        })
        .detach();
        Self {
            focus: cx.focus_handle(),
            return_focus,
            mcp_code_focus: cx.focus_handle(),
            page: SettingsPage::General,
            choice_selects,
            font_selects,
            preferences,
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
            archive_load_error,
            archive_action_error: None,
            restoring_archive: None,
            error: settings_error,
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
        self.archived_notes = archived_notes;
        self.archive_load_error = error;
        cx.notify();
    }

    fn persist(&mut self) {
        let language = self.preferences.language;
        self.error = match self.preferences.save() {
            Ok(()) => None,
            Err(error) => {
                eprintln!("{error}");
                Some(t!("settings.save_error", locale = language.locale()).into_owned())
            }
        };
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
            prompt: Some(
                t!("settings.library.choose_prompt", locale = language.locale())
                    .into_owned()
                    .into(),
            ),
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

    fn sync_choice_selects(&self, window: &mut Window, cx: &mut Context<Self>) {
        let language = self.preferences.language;
        for dropdown in Dropdown::ALL {
            let items = choice_items(dropdown, language);
            let selected = dropdown.selected(&self.preferences);
            self.choice_selects[&dropdown].update(cx, |select, cx| {
                select.set_items(items, window, cx);
                select.set_selected_value(&selected, window, cx);
            });
        }
    }

    fn apply_choice(&mut self, choice: Choice, window: &mut Window, cx: &mut Context<Self>) {
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
                theme::apply_preference(preference, self.preferences.ui_font_size, window, cx);
            }
            Choice::Language(language) => {
                if self.preferences.language == language {
                    return;
                }
                self.preferences.language = language;
                self.terminal_shell_input.update(cx, |input, cx| {
                    input.set_placeholder(
                        t!(
                            "settings.terminal.shell.placeholder",
                            locale = language.locale()
                        )
                        .into_owned(),
                        window,
                        cx,
                    );
                });
                self.terminal_command_input.update(cx, |input, cx| {
                    input.set_placeholder(
                        t!(
                            "settings.terminal.command.placeholder",
                            locale = language.locale()
                        )
                        .into_owned(),
                        window,
                        cx,
                    );
                });
                self.mcp_port_input.update(cx, |input, cx| {
                    input.set_placeholder(
                        t!("settings.mcp.port.placeholder", locale = language.locale())
                            .into_owned(),
                        window,
                        cx,
                    );
                });
                self.sync_choice_selects(window, cx);
                cx.emit(SettingsEvent::LanguageChanged(language));
            }
            Choice::UiFontSize(size) => {
                if self.preferences.ui_font_size == size {
                    return;
                }
                self.preferences.ui_font_size = size;
                theme::apply_ui_font_size(size, window, cx);
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
        let theme = Theme::global(cx).clone();
        let language = self.preferences.language;
        let back_label = t!("settings.navigation.back", locale = language.locale()).into_owned();
        let traffic_light_clearance = if cfg!(target_os = "macos") && !fullscreen {
            WINDOW_TRAFFIC_LIGHT_CLEARANCE
        } else {
            SETTINGS_TITLEBAR_CONTROL_INSET
        };

        settings_sidebar_header(
            traffic_light_clearance,
            self.render_titlebar_drag_region("settings-sidebar-traffic-light-drag-region", cx)
                .into_any_element(),
            settings_back_button("settings-back", back_label, &theme)
                .on_click(cx.listener(|this, _, window, cx| this.close(window, cx)))
                .into_any_element(),
        )
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> Stateful<gpui::Div> {
        let language = self.preferences.language;
        let theme = Theme::global(cx).clone();
        div()
            .id("settings-sidebar-scroll")
            .size_full()
            .overflow_y_scroll()
            .px(ui_rem(12.0))
            .pt(ui_rem(12.0))
            .pb(ui_rem(20.0))
            .flex()
            .flex_col()
            .gap(ui_rem(14.0))
            .child(settings_navigation_group(
                t!(
                    "settings.navigation.application",
                    locale = language.locale()
                )
                .into_owned(),
                vec![
                    self.render_navigation_row(
                        "settings-general",
                        SettingsPage::General.label(language),
                        Icon::Settings,
                        SettingsPage::General,
                        cx,
                    ),
                    self.render_navigation_row(
                        "settings-appearance",
                        SettingsPage::Appearance.label(language),
                        Icon::Palette,
                        SettingsPage::Appearance,
                        cx,
                    ),
                    self.render_navigation_row(
                        "settings-terminal",
                        SettingsPage::Terminal.label(language),
                        Icon::SquareTerminal,
                        SettingsPage::Terminal,
                        cx,
                    ),
                ],
                &theme,
            ))
            .child(settings_navigation_group(
                t!("settings.navigation.library", locale = language.locale()).into_owned(),
                vec![self.render_navigation_row(
                    "settings-archive",
                    SettingsPage::Archive.label(language),
                    Icon::Inbox,
                    SettingsPage::Archive,
                    cx,
                )],
                &theme,
            ))
            .child(settings_navigation_group(
                t!(
                    "settings.navigation.connections",
                    locale = language.locale()
                )
                .into_owned(),
                vec![self.render_navigation_row(
                    "settings-mcp",
                    SettingsPage::Mcp.label(language),
                    Icon::Network,
                    SettingsPage::Mcp,
                    cx,
                )],
                &theme,
            ))
    }

    fn render_navigation_row(
        &self,
        id: &'static str,
        label: String,
        icon_kind: Icon,
        page: SettingsPage,
        cx: &mut Context<Self>,
    ) -> gpui_component::button::Button {
        let selected = self.page == page;
        settings_navigation_row(id, label, icon_kind, selected, cx).on_click(cx.listener(
            move |this, _, window, cx| {
                this.apply_choice(Choice::Page(page), window, cx);
            },
        ))
    }

    fn render_content_header(&self, cx: &mut Context<Self>) -> Stateful<gpui::Div> {
        let title = self.page.label(self.preferences.language);
        self.render_titlebar_drag_region("settings-content-titlebar", cx)
            .size_full()
            .px(ui_rem(32.0))
            .flex()
            .items_center()
            .text_sm()
            .font_weight(FontWeight::SEMIBOLD)
            .child(title)
    }

    fn render_content(&self, cx: &mut Context<Self>) -> Stateful<gpui::Div> {
        let theme = Theme::global(cx).clone();
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
            .px(ui_rem(32.0))
            .pb(ui_rem(48.0))
            .child(
                div()
                    .w_full()
                    .max_w(ui_rem(SETTINGS_CONTENT_MAX_WIDTH))
                    .mx_auto()
                    .pb(ui_rem(190.0))
                    .when_some(self.error.as_ref(), |view, error| {
                        view.child(
                            div()
                                .mt(ui_rem(14.0))
                                .rounded(ui_rem(8.0))
                                .border_1()
                                .border_color(theme.danger)
                                .px(ui_rem(12.0))
                                .py(ui_rem(9.0))
                                .text_sm()
                                .text_color(theme.danger)
                                .child(error.clone()),
                        )
                    })
                    .child(
                        div()
                            .w_full()
                            .min_w_0()
                            .mt(ui_rem(15.0))
                            .rounded(ui_rem(13.0))
                            .bg(theme.muted)
                            .flex()
                            .flex_col()
                            .items_stretch()
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
        let theme = Theme::global(cx).clone();
        div()
            .key_context("Settings")
            .track_focus(&self.focus)
            .size_full()
            .flex()
            .bg(theme.background)
            .text_color(theme.foreground)
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
                .sidebar_width(DEFAULT_LEFT_SIDEBAR_WIDTH)
                .titlebar_height(WINDOW_TITLEBAR_HEIGHT)
                .into_element(),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_supported_font_size_has_its_own_label() {
        assert_eq!(
            FONT_SIZES.map(font_size_label),
            ["12", "14", "15", "16", "17", "18", "20"]
        );
    }
}

#[cfg(test)]
mod localization_tests;
