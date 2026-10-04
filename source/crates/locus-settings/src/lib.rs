rust_i18n::i18n!("locales", fallback = "en");

use rust_i18n::t;
use std::path::Path;

mod archive;
mod controls;
mod mcp;
mod pages;
mod skills;

use gpui_kit::component::Theme;
use gpui_kit::component::{
    IndexPath,
    input::{Input, InputEvent, InputState},
    select::{SearchableVec, SelectEvent, SelectItem, SelectState},
};
use gpui_kit::{
    App, Context, Entity, EventEmitter, FocusHandle, FontWeight, PathPromptOptions, Render,
    Stateful, StatefulInteractiveElement, Window, div, prelude::*, px,
};
use locus_library::ArchivedNote;
use locus_mcp::McpServerState;
use locus_preferences::config::{DEFAULT_CODE_FONT_FAMILY, DEFAULT_UI_FONT_FAMILY};
use locus_preferences::{
    AgentPreference, EditorPreference, Preferences, ShellPreference, SkillsLocation,
    TerminalPreference,
};
use locus_ui::{Icon, icon};
use oneo_gpui::Language;
use oneo_gpui::{AppearanceChange, AppearanceControls, FONT_SIZES};
use oneo_gpui::{
    SettingsGroup, SettingsItem, SettingsShell, settings_navigation_group, settings_navigation_row,
};

fn ui_rem(value: f32) -> gpui_kit::Rems {
    gpui_kit::rems(value / 16.0)
}

const SETTINGS_CONTENT_MAX_WIDTH: f32 = 720.0;
const SETTINGS_CONTROL_WIDTH_REM: f32 = 20.0;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum SettingsPage {
    #[default]
    General,
    Archive,
    Appearance,
    Terminal,
    Skills,
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
            Self::Skills => t!("settings.page.skills", locale = language.locale()).into_owned(),
            Self::Mcp => "MCP".to_owned(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Choice {
    Page(SettingsPage),
    Editor(EditorPreference),
    CodeFontSize(f32),
    Terminal(TerminalPreference),
    Agent(AgentPreference),
    Shell(ShellPreference),
    SkillsLocation(SkillsLocation),
}

impl Choice {
    fn label(self, language: Language) -> String {
        match self {
            Self::Page(page) => page.label(language),
            Self::Editor(editor) => editor.label(language),
            Self::CodeFontSize(size) => font_size_label(size).to_owned(),
            Self::Terminal(terminal) => terminal.label(language),
            Self::Agent(agent) => agent.label().to_owned(),
            Self::Shell(shell) => shell.label(language),
            Self::SkillsLocation(location) => location.label(language),
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
    SkillsLocation,
    Editor,
    CodeFontSize,
    Terminal,
    Agent,
    Shell,
}

impl Dropdown {
    const ALL: [Self; 6] = [
        Self::SkillsLocation,
        Self::Editor,
        Self::CodeFontSize,
        Self::Terminal,
        Self::Agent,
        Self::Shell,
    ];

    fn label(self, language: Language) -> String {
        let key = match self {
            Self::Editor => "settings.editor.label",
            Self::CodeFontSize => "settings.appearance.code_font_size.label",
            Self::Terminal => "settings.terminal.application.label",
            Self::Agent => "settings.terminal.agent.label",
            Self::Shell => "settings.terminal.shell.label",
            Self::SkillsLocation => "settings.skills.location",
        };
        t!(key, locale = language.locale()).into_owned()
    }

    fn choices(self) -> Vec<Choice> {
        match self {
            Self::SkillsLocation => SkillsLocation::ALL
                .into_iter()
                .map(Choice::SkillsLocation)
                .collect(),
            Self::Editor => EditorPreference::ALL
                .into_iter()
                .map(Choice::Editor)
                .collect(),
            Self::CodeFontSize => FONT_SIZES.into_iter().map(Choice::CodeFontSize).collect(),
            Self::Terminal => TerminalPreference::ALL
                .iter()
                .copied()
                .map(Choice::Terminal)
                .collect(),
            Self::Shell => ShellPreference::ALL
                .iter()
                .copied()
                .map(Choice::Shell)
                .collect(),
            Self::Agent => AgentPreference::ALL
                .into_iter()
                .map(Choice::Agent)
                .collect(),
        }
    }

    fn selected(self, preferences: &Preferences) -> Choice {
        match self {
            Self::Editor => Choice::Editor(preferences.editor),
            Self::CodeFontSize => Choice::CodeFontSize(preferences.code_font_size),
            Self::Terminal => Choice::Terminal(preferences.terminal),
            Self::Agent => Choice::Agent(preferences.agent),
            Self::Shell => Choice::Shell(preferences.agent_shell),
            Self::SkillsLocation => Choice::SkillsLocation(preferences.skills_location),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum FontKind {
    Ui,
    Code,
}

#[derive(Clone)]
struct ChoiceItem {
    label: gpui_kit::SharedString,
    value: Choice,
}

impl SelectItem for ChoiceItem {
    type Value = Choice;

    fn title(&self) -> gpui_kit::SharedString {
        self.label.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.value
    }
}

#[derive(Clone)]
struct FontItem {
    family: gpui_kit::SharedString,
}

impl SelectItem for FontItem {
    type Value = gpui_kit::SharedString;

    fn title(&self) -> gpui_kit::SharedString {
        self.family.clone()
    }

    fn display_title(&self) -> Option<gpui_kit::AnyElement> {
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
        }
    }

    fn selected(self, preferences: &Preferences) -> &str {
        match self {
            Self::Ui => preferences.resolved_ui_font_family(),
            Self::Code => preferences.resolved_code_font_family(),
        }
    }
}

pub enum SettingsEvent {
    Close,
    ArchiveRestored,
    LanguageChanged(Language),
    LibraryChanged(std::path::PathBuf),
    UiFontChanged,
    CodeFontChanged,
    McpConfigurationChanged,
}

pub struct Settings {
    focus: FocusHandle,
    return_focus: FocusHandle,
    page: SettingsPage,
    choice_selects: std::collections::HashMap<Dropdown, Entity<ChoiceSelect>>,
    font_selects: std::collections::HashMap<FontKind, Entity<FontSelect>>,
    preferences: Preferences,
    appearance_controls: AppearanceControls,
    mcp_port_input: Entity<InputState>,
    mcp_port_draft: String,
    mcp_port_error: Option<String>,
    mcp_server_state: McpServerState,
    archived_notes: Vec<ArchivedNote>,
    archive_load_error: Option<String>,
    archive_action_error: Option<String>,
    restoring_archive: Option<std::path::PathBuf>,
    error: Option<String>,
    skills_installing: bool,
    skills_result: Option<Result<std::path::PathBuf, String>>,
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
        let language = preferences.appearance.language;
        let settings_error = load_error.map(|error| {
            eprintln!("{error}");
            t!("settings.load_error", locale = language.locale()).into_owned()
        });
        let mut font_names = cx.text_system().all_font_names();
        for family in [
            DEFAULT_UI_FONT_FAMILY,
            DEFAULT_CODE_FONT_FAMILY,
            preferences.resolved_ui_font_family(),
            preferences.resolved_code_font_family(),
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
        for kind in [FontKind::Ui, FontKind::Code] {
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
        let mcp_port_draft = preferences.mcp_port.to_string();
        let mcp_port_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(mcp_port_draft.clone())
                .placeholder(
                    t!("settings.mcp.port.placeholder", locale = language.locale()).into_owned(),
                )
        });
        cx.subscribe(&mcp_port_input, |this, input, event: &InputEvent, cx| {
            if !matches!(event, InputEvent::Change) {
                return;
            }
            this.mcp_port_draft = input.read(cx).value().to_string();
            this.mcp_port_error = None;
            cx.notify();
        })
        .detach();
        let appearance_controls =
            AppearanceControls::new(preferences.appearance, window, cx, Self::change_appearance);
        Self {
            appearance_controls,
            focus: cx.focus_handle(),
            return_focus,
            page: SettingsPage::General,
            choice_selects,
            font_selects,
            preferences,
            mcp_port_input,
            mcp_port_draft,
            mcp_port_error: None,
            mcp_server_state,
            archived_notes,
            archive_load_error,
            archive_action_error: None,
            restoring_archive: None,
            error: settings_error,
            skills_installing: false,
            skills_result: None,
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
        let language = self.preferences.appearance.language;
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
        let language = self.preferences.appearance.language;
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

    fn choose_export_directory(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(
                t!(
                    "settings.export.choose_prompt",
                    locale = self.preferences.appearance.language.locale()
                )
                .into_owned()
                .into(),
            ),
        });
        cx.spawn(async move |this, cx| {
            let result = receiver.await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(Ok(Some(paths))) => {
                        if let Some(path) = paths.into_iter().next() {
                            let previous = this.preferences.export_directory.replace(path);
                            this.persist();
                            if this.error.is_some() {
                                this.preferences.export_directory = previous;
                            }
                        }
                    }
                    Ok(Ok(None)) => {}
                    _ => {
                        this.error = Some(
                            t!(
                                "settings.export.choose_error",
                                locale = this.preferences.appearance.language.locale()
                            )
                            .into_owned(),
                        );
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn sync_choice_selects(&self, window: &mut Window, cx: &mut Context<Self>) {
        let language = self.preferences.appearance.language;
        for dropdown in Dropdown::ALL {
            let items = choice_items(dropdown, language);
            let selected = dropdown.selected(&self.preferences);
            self.choice_selects[&dropdown].update(cx, |select, cx| {
                select.set_items(items, window, cx);
                select.set_selected_value(&selected, window, cx);
            });
        }
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
        match change {
            AppearanceChange::Language(language) => {
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
            AppearanceChange::FontSize(_) => cx.emit(SettingsEvent::UiFontChanged),
            AppearanceChange::Theme(_) => {}
        }
        self.persist();
        cx.notify();
    }

    fn apply_choice(&mut self, choice: Choice, _window: &mut Window, cx: &mut Context<Self>) {
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
            Choice::CodeFontSize(size) => {
                if self.preferences.code_font_size == size {
                    return;
                }
                self.preferences.code_font_size = size;
                cx.emit(SettingsEvent::CodeFontChanged);
            }
            Choice::Terminal(terminal) => self.preferences.terminal = terminal,
            Choice::Shell(shell) => self.preferences.agent_shell = shell,
            Choice::Agent(agent) => self.preferences.agent = agent,
            Choice::SkillsLocation(location) => {
                if self.skills_installing {
                    return;
                }
                let previous = self.preferences.skills_location;
                self.preferences.skills_location = location;
                self.skills_result = None;
                self.persist();
                if self.error.is_some() {
                    self.preferences.skills_location = previous;
                    self.sync_choice_selects(_window, cx);
                } else {
                    self.install_skills(cx);
                }
                cx.notify();
                return;
            }
        }
        self.persist();
        cx.notify();
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> Stateful<gpui_kit::Div> {
        let language = self.preferences.appearance.language;
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
            .child(settings_navigation_group(
                t!("settings.page.skills", locale = language.locale()).into_owned(),
                vec![self.render_navigation_row(
                    "settings-skills",
                    SettingsPage::Skills.label(language),
                    Icon::Settings,
                    SettingsPage::Skills,
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
    ) -> gpui_kit::component::button::Button {
        let selected = self.page == page;
        settings_navigation_row(id, label, icon_kind, selected, cx).on_click(cx.listener(
            move |this, _, window, cx| {
                this.apply_choice(Choice::Page(page), window, cx);
            },
        ))
    }

    fn render_content(&self, cx: &mut Context<Self>) -> Stateful<gpui_kit::Div> {
        let theme = Theme::global(cx).clone();
        let content = match self.page {
            SettingsPage::General => self.render_general(cx),
            SettingsPage::Archive => self.render_archive(cx),
            SettingsPage::Appearance => self.render_appearance(cx),
            SettingsPage::Terminal => self.render_terminal(cx),
            SettingsPage::Skills => self.render_skills(cx),
            SettingsPage::Mcp => self.render_mcp(cx),
        };

        div()
            .id("settings-content-scroll")
            .size_full()
            .overflow_y_scroll()
            .px(ui_rem(32.0))
            .pt(ui_rem(32.0))
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
                        SettingsGroup::new(self.page.label(self.preferences.appearance.language))
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx).clone();
        div()
            .key_context("Settings")
            .track_focus(&self.focus)
            .on_action(
                cx.listener(|this, _: &oneo_gpui::CloseSettings, window, cx| {
                    this.close(window, cx)
                }),
            )
            .size_full()
            .flex()
            .text_color(theme.foreground)
            .font_family(self.preferences.resolved_ui_font_family().to_owned())
            .child(
                SettingsShell::new(
                    self.render_sidebar(cx).into_any_element(),
                    self.render_content(cx).into_any_element(),
                    cx,
                )
                .into_element(),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_choices_reflect_the_runtime_preference() {
        let preferences = Preferences {
            agent_shell: ShellPreference::Fish,
            ..Preferences::default()
        };
        assert_eq!(
            Dropdown::Shell.selected(&preferences),
            Choice::Shell(ShellPreference::Fish)
        );
        for language in [Language::English, Language::Chinese] {
            let items = choice_items(Dropdown::Shell, language);
            assert_eq!(items.len(), ShellPreference::ALL.len());
            for (item, shell) in items.iter().zip(ShellPreference::ALL) {
                assert_eq!(item.value, Choice::Shell(*shell));
                assert_eq!(item.label.as_ref(), shell.label(language));
            }
        }
    }

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
