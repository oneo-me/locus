use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

mod navigation;
mod note;
mod note_atmosphere;
mod note_menu;
mod pane;
mod sidebar;
mod window_chrome;

use gpui::{
    AnyElement, Bounds, ClipboardItem, Context, Div, Entity, FocusHandle, FontWeight, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, NavigationDirection, Pixels, Render, Role,
    ScrollAnchor, ScrollHandle, Stateful, StyleRefinement, Task, Window, div, prelude::*, px,
    transparent_black,
};

use self::navigation::LibraryNavigation;
use self::pane::LocusPane;
use super::settings::{Settings, SettingsEvent};
use crate::config::{DEFAULT_LEFT_SIDEBAR_WIDTH, DEFAULT_RIGHT_SIDEBAR_WIDTH, WINDOW_MIN_WIDTH};
use crate::document_library::{LibraryTree, LibraryWatchEvent, LibraryWatcher, load_note};
use crate::localization::Language;
use crate::mcp::McpServerController;
use crate::preferences::Preferences;
use crate::sidebar_order::SidebarOrderState;
use crate::state::{
    InterfaceState, LEFT_SIDEBAR_MAX_WIDTH, LEFT_SIDEBAR_MIN_WIDTH, RIGHT_SIDEBAR_MAX_WIDTH,
    RIGHT_SIDEBAR_MIN_WIDTH,
};
use crate::theme::Theme;
use crate::ui::{
    ActivationExt, Icon, MarkdownCodeBlocks, MarkdownDocument, MarkdownSelection, ScrollbarState,
    icon, toolbar_icon_button,
};
use crate::{
    CloseWindow, OpenSettings, ReloadCurrentNote, ToggleFpsCounter, ToggleLeftPanel,
    ToggleRightPanel,
};
use oneo_gpui_terminal::{TerminalConfiguration, TerminalLabels, TerminalView};
use oneo_gpui_ui::app_shell::{
    AppShellState, PanelSide, PanelSize, TITLEBAR_CONTROL_INSET, TITLEBAR_CONTROL_SIZE,
    TITLEBAR_HEIGHT, WorkspaceShell, WorkspaceShellConfig, WorkspaceShellState,
    panel_resize_handle, panel_toggle_button, titlebar_content_left_padding,
    titlebar_left_control_offset,
};

const TITLEBAR_CONTROL_GAP: f32 = 8.0;
const TITLEBAR_NAVIGATION_INSET: f32 = 6.0;
const TITLEBAR_NAVIGATION_GAP: f32 = 2.0;

fn terminal_labels(language: Language) -> TerminalLabels {
    TerminalLabels {
        accessibility_label: language.text("终端", "Terminal").into(),
        could_not_start: language
            .text("无法启动终端", "Could not start terminal")
            .into(),
        starting: language.text("正在启动终端…", "Starting terminal…").into(),
        process_exited: language.text("进程已退出", "Process exited").into(),
    }
}

fn terminal_configuration(preferences: &Preferences) -> TerminalConfiguration {
    TerminalConfiguration {
        shell: preferences.terminal_shell.clone(),
        startup_command: preferences.terminal_command.clone(),
        color_scheme: preferences.terminal_color_scheme,
        cursor_style: preferences.terminal_cursor_style,
        font_family: preferences
            .resolved_terminal_font_family()
            .to_owned()
            .into(),
        font_size: preferences.terminal_font_size,
        labels: terminal_labels(preferences.language),
    }
}
const TITLEBAR_NAVIGATION_WIDTH: f32 = TITLEBAR_CONTROL_SIZE * 2.0 + TITLEBAR_NAVIGATION_GAP;
const WORKSPACE_SHELL_CONFIG: WorkspaceShellConfig = WorkspaceShellConfig {
    content_min_width: 360.0,
    left: PanelSize {
        min: LEFT_SIDEBAR_MIN_WIDTH,
        max: LEFT_SIDEBAR_MAX_WIDTH,
    },
    right: PanelSize {
        min: RIGHT_SIDEBAR_MIN_WIDTH,
        max: RIGHT_SIDEBAR_MAX_WIDTH,
    },
};
const COPY_FEEDBACK_DURATION: Duration = Duration::from_secs(2);
const LIBRARY_EVENT_DEBOUNCE: Duration = Duration::from_millis(180);
const WINDOW_BOUNDS_SAVE_DEBOUNCE: Duration = Duration::from_millis(100);

struct SelectedNote {
    path: String,
    document: Option<MarkdownDocument>,
    error: Option<String>,
}

pub struct Locus {
    root_focus: FocusHandle,
    app_shell: AppShellState,
    header_drag_armed: bool,
    left_toggle_focus: FocusHandle,
    navigation_back_focus: FocusHandle,
    navigation_forward_focus: FocusHandle,
    right_toggle_focus: FocusHandle,
    mcp_status_focus: FocusHandle,
    settings_focus: FocusHandle,
    settings: Entity<Settings>,
    left_pane: Entity<LocusPane>,
    content_pane: Entity<LocusPane>,
    right_pane: Entity<LocusPane>,
    mcp_server: McpServerController,
    terminal: Option<Entity<TerminalView>>,
    terminal_needs_restart: bool,
    library_tree: LibraryTree,
    library_watcher: Option<LibraryWatcher>,
    library_watch_task: Option<Task<()>>,
    library_watcher_error: Option<String>,
    sidebar_order: SidebarOrderState,
    dragged_library_node: Option<String>,
    library_reorder_offsets: HashMap<String, f32>,
    library_reorder_generation: u64,
    expanded_library_nodes: HashSet<String>,
    selected_library_node: Option<String>,
    library_navigation: LibraryNavigation,
    selected_note: Option<SelectedNote>,
    note_load_generation: u64,
    note_load_task: Option<Task<()>>,
    library_scroll_handle: ScrollHandle,
    library_scrollbar_state: Rc<ScrollbarState>,
    selected_library_scroll_anchor: ScrollAnchor,
    scroll_to_selected_library_node: bool,
    note_copy_focus: FocusHandle,
    note_text_focus: FocusHandle,
    note_text_selection: MarkdownSelection,
    note_code_blocks: MarkdownCodeBlocks,
    note_edit_focus: FocusHandle,
    note_open_directory_focus: FocusHandle,
    note_menu_focus: FocusHandle,
    note_menu_upload_focus: FocusHandle,
    note_menu_archive_focus: FocusHandle,
    note_menu_open: bool,
    note_action_pending: bool,
    copied_library_node: Option<String>,
    note_copy_generation: u64,
    mcp_status_copied: bool,
    mcp_status_copy_generation: u64,
    note_scroll_handle: ScrollHandle,
    note_scrollbar_state: Rc<ScrollbarState>,
    window_bounds: Option<Bounds<Pixels>>,
    window_bounds_save_task: Option<Task<()>>,
}

impl Locus {
    pub fn new(
        window: &mut Window,
        preferences: Preferences,
        load_error: Option<String>,
        interface_state: InterfaceState,
        cx: &mut Context<Self>,
    ) -> Self {
        let root_focus = cx.focus_handle();
        let locus = cx.entity();
        let left_pane = LocusPane::new(&locus, Locus::left_pane_content, cx);
        let content_pane = LocusPane::new(&locus, Locus::content_pane_content, cx);
        let right_pane = LocusPane::new(&locus, Locus::right_pane_content, cx);
        window.focus(&root_focus, cx);
        crate::theme::apply_preference(preferences.theme, window, cx);
        window.set_rem_size(px(preferences.ui_font_size));
        let library_path = preferences.library_path.clone();
        let (library_watcher, library_watch_task, library_watcher_error) =
            Self::start_library_watcher(library_path.as_deref(), cx);
        let (sidebar_order, sidebar_order_error) = SidebarOrderState::load();
        if let Some(error) = sidebar_order_error {
            eprintln!("{error}");
        }
        let mut library_tree = LibraryTree::load(library_path.as_deref());
        if let Some(path) = library_path.as_deref() {
            sidebar_order.apply(path, &mut library_tree.nodes);
        }
        let selected_library_node = interface_state
            .selected_library_node
            .clone()
            .filter(|path| library_tree.node(path).is_some());
        let expanded_library_nodes = selected_library_node
            .as_deref()
            .map(|path| {
                sidebar::expanded_paths_for_selection(
                    path,
                    library_tree
                        .node(path)
                        .is_some_and(|node| !node.children.is_empty()),
                )
            })
            .unwrap_or_default();
        let library_scroll_handle = ScrollHandle::new();
        let library_scrollbar_state = ScrollbarState::new();
        let selected_library_scroll_anchor =
            ScrollAnchor::for_handle(library_scroll_handle.clone());
        let scroll_to_selected_library_node = selected_library_node.is_some();
        let note_load_generation = 1;
        let (selected_note, note_load_task) = selected_library_node
            .as_deref()
            .and_then(|path| {
                library_tree
                    .node(path)
                    .and_then(|node| node.document_path.as_ref())
                    .map(|document_path| {
                        (
                            Some(SelectedNote {
                                path: path.to_owned(),
                                document: None,
                                error: None,
                            }),
                            Some(Self::spawn_note_load(
                                path.to_owned(),
                                document_path.clone(),
                                note_load_generation,
                                cx,
                            )),
                        )
                    })
            })
            .unwrap_or((None, None));
        let terminal_configuration = terminal_configuration(&preferences);
        let terminal = library_path
            .clone()
            .map(|path| cx.new(|cx| TerminalView::new(path, terminal_configuration.clone(), cx)));
        let mcp_server = McpServerController::new(&preferences);
        let mcp_server_state = mcp_server.state();
        let archived_notes = library_tree.archived_notes.clone();
        let archive_load_error = library_tree.error.clone();
        let settings = cx.new(|cx| {
            Settings::new(
                preferences,
                load_error,
                mcp_server_state,
                archived_notes,
                archive_load_error,
                root_focus.clone(),
                cx,
            )
        });
        cx.subscribe(
            &settings,
            |this, _, event: &SettingsEvent, cx| match event {
                SettingsEvent::Close => {
                    this.app_shell.close_settings();
                    if this.terminal_needs_restart {
                        this.restart_terminal(cx);
                    }
                    cx.notify();
                }
                SettingsEvent::ArchiveRestored => {
                    this.reload_library_projection(cx);
                }
                SettingsEvent::LanguageChanged(language) => {
                    crate::set_app_menus(cx, *language, this.can_reload_current_note());
                    if let Some(terminal) = &this.terminal {
                        terminal.update(cx, |terminal, cx| {
                            terminal.set_labels(terminal_labels(*language), cx);
                        });
                    }
                    cx.notify();
                }
                SettingsEvent::LibraryChanged(path) => {
                    this.change_library(path, cx);
                    this.restart_terminal(cx);
                    this.restart_mcp_server(cx);
                }
                SettingsEvent::TerminalLaunchChanged => {
                    this.terminal_needs_restart = true;
                }
                SettingsEvent::TerminalColorSchemeChanged(color_scheme) => {
                    if let Some(terminal) = &this.terminal {
                        terminal.update(cx, |terminal, cx| {
                            terminal.set_color_scheme(*color_scheme, cx);
                        });
                    }
                }
                SettingsEvent::TerminalCursorStyleChanged(cursor_style) => {
                    if let Some(terminal) = &this.terminal {
                        terminal.update(cx, |terminal, cx| {
                            terminal.set_cursor_style(*cursor_style, cx);
                        });
                    }
                }
                SettingsEvent::UiFontChanged | SettingsEvent::CodeFontChanged => {
                    cx.notify();
                }
                SettingsEvent::TerminalFontChanged(family, size) => {
                    if let Some(terminal) = &this.terminal {
                        terminal.update(cx, |terminal, cx| {
                            terminal.set_font(family.clone(), *size, cx);
                        });
                    }
                }
                SettingsEvent::McpConfigurationChanged => {
                    this.restart_mcp_server(cx);
                }
            },
        )
        .detach();
        cx.observe_window_appearance(window, |this, window, cx| {
            let preference = this.settings.read(cx).preferences().theme;
            if preference == crate::theme::ThemePreference::System {
                crate::theme::apply_preference(preference, window, cx);
                cx.notify();
            }
        })
        .detach();
        cx.observe_window_bounds(window, |this, window, cx| {
            this.window_bounds = Some(window.inner_window_bounds().get_bounds());
            if this.window_bounds_save_task.is_none() {
                this.window_bounds_save_task = Some(cx.spawn(async move |this, cx| {
                    cx.background_executor()
                        .timer(WINDOW_BOUNDS_SAVE_DEBOUNCE)
                        .await;
                    this.update(cx, |this, _| {
                        this.window_bounds_save_task.take();
                        this.save_interface_state();
                    })
                    .ok();
                }));
            }
        })
        .detach();
        cx.on_app_quit(|this, _| {
            this.window_bounds_save_task.take();
            this.save_interface_state();
            async {}
        })
        .detach();

        Self {
            root_focus,
            app_shell: AppShellState::new(WorkspaceShellState::new(
                WORKSPACE_SHELL_CONFIG,
                interface_state.left_sidebar_visible,
                interface_state.right_sidebar_visible,
                interface_state.left_sidebar_width,
                interface_state.right_sidebar_width,
            ))
            .with_fps_counter_visible(interface_state.fps_counter_visible),
            header_drag_armed: false,
            left_toggle_focus: cx.focus_handle(),
            navigation_back_focus: cx.focus_handle(),
            navigation_forward_focus: cx.focus_handle(),
            right_toggle_focus: cx.focus_handle(),
            settings_focus: cx.focus_handle(),
            settings,
            left_pane,
            content_pane,
            right_pane,
            mcp_server,
            terminal,
            terminal_needs_restart: false,
            library_tree,
            library_watcher,
            library_watch_task,
            library_watcher_error,
            sidebar_order,
            dragged_library_node: None,
            library_reorder_offsets: HashMap::new(),
            library_reorder_generation: 0,
            expanded_library_nodes,
            selected_library_node,
            library_navigation: LibraryNavigation::default(),
            selected_note,
            note_load_generation,
            note_load_task,
            library_scroll_handle,
            library_scrollbar_state,
            selected_library_scroll_anchor,
            scroll_to_selected_library_node,
            note_copy_focus: cx.focus_handle(),
            note_text_focus: cx.focus_handle(),
            note_text_selection: MarkdownSelection::default(),
            note_code_blocks: MarkdownCodeBlocks::default(),
            note_edit_focus: cx.focus_handle(),
            note_open_directory_focus: cx.focus_handle(),
            note_menu_focus: cx.focus_handle(),
            note_menu_upload_focus: cx.focus_handle(),
            note_menu_archive_focus: cx.focus_handle(),
            note_menu_open: false,
            note_action_pending: false,
            copied_library_node: None,
            note_copy_generation: 0,
            mcp_status_focus: cx.focus_handle(),
            mcp_status_copied: false,
            mcp_status_copy_generation: 0,
            note_scroll_handle: ScrollHandle::new(),
            note_scrollbar_state: ScrollbarState::new(),
            window_bounds: interface_state.window_bounds,
            window_bounds_save_task: None,
        }
    }

    fn start_library_watcher(
        root: Option<&Path>,
        cx: &mut Context<Self>,
    ) -> (Option<LibraryWatcher>, Option<Task<()>>, Option<String>) {
        let Some(root) = root else {
            return (None, None, None);
        };
        let root = root.to_path_buf();
        let (sender, receiver) = async_channel::unbounded();
        let watcher = match LibraryWatcher::start(&root, move |event| {
            let _ = sender.try_send(event);
        }) {
            Ok(watcher) => watcher,
            Err(error) => return (None, None, Some(error)),
        };
        let task_root = root.clone();
        let task = cx.spawn(async move |this, cx| {
            while let Ok(event) = receiver.recv().await {
                let mut changed_paths = HashSet::new();
                let mut rescan_all = false;
                let mut watcher_error = None;
                merge_library_watch_event(
                    event,
                    &mut changed_paths,
                    &mut rescan_all,
                    &mut watcher_error,
                );
                cx.background_executor().timer(LIBRARY_EVENT_DEBOUNCE).await;
                while let Ok(event) = receiver.try_recv() {
                    merge_library_watch_event(
                        event,
                        &mut changed_paths,
                        &mut rescan_all,
                        &mut watcher_error,
                    );
                }

                if !rescan_all && changed_paths.is_empty() {
                    let _ = this.update(cx, |this, cx| {
                        if this.current_library_path(cx) == Some(task_root.as_path()) {
                            this.library_watcher_error = watcher_error;
                            cx.notify();
                        }
                    });
                    continue;
                }

                let scan_root = task_root.clone();
                let tree = cx
                    .background_spawn(async move { LibraryTree::load(Some(&scan_root)) })
                    .await;
                let _ = this.update(cx, |this, cx| {
                    if this.current_library_path(cx) != Some(task_root.as_path()) {
                        return;
                    }
                    if let Some(error) = tree.error.clone() {
                        this.library_watcher_error = Some(error);
                        cx.notify();
                        return;
                    }
                    this.library_watcher_error = watcher_error;
                    let reload_selected_note = this
                        .selected_library_node
                        .as_deref()
                        .and_then(|path| this.library_tree.node(path))
                        .and_then(|node| node.document_path.as_deref())
                        .is_some_and(|document_path| {
                            event_affects_document(
                                task_root.as_path(),
                                document_path,
                                &changed_paths,
                                rescan_all,
                            )
                        });
                    this.apply_library_tree(tree, reload_selected_note, cx);
                });
            }
        });
        (Some(watcher), Some(task), None)
    }

    fn spawn_note_load(
        path: String,
        document_path: std::path::PathBuf,
        generation: u64,
        cx: &mut Context<Self>,
    ) -> Task<()> {
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    load_note(&document_path)
                        .map(|content| {
                            MarkdownDocument::parse_with_hidden_code_blocks(
                                &content,
                                &["gallery", "attachments"],
                            )
                        })
                        .map_err(|error| error.to_string())
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.note_load_generation != generation
                    || this.selected_library_node.as_deref() != Some(path.as_str())
                {
                    return;
                }
                let Some(note) = this.selected_note.as_mut().filter(|note| note.path == path)
                else {
                    return;
                };
                match result {
                    Ok(document) => {
                        note.document = Some(document);
                        note.error = None;
                    }
                    Err(error) => note.error = Some(error),
                }
                cx.notify();
            });
        })
    }

    fn current_library_path<'a>(&self, cx: &'a Context<Self>) -> Option<&'a Path> {
        self.settings.read(cx).preferences().library_path.as_deref()
    }

    fn change_library(&mut self, path: &Path, cx: &mut Context<Self>) {
        let could_reload_current_note = self.can_reload_current_note();
        self.library_watch_task = None;
        self.library_watcher = None;
        let (watcher, task, watcher_error) = Self::start_library_watcher(Some(path), cx);
        self.library_watcher = watcher;
        self.library_watch_task = task;
        self.library_watcher_error = watcher_error;
        self.library_tree = LibraryTree::load(Some(path));
        self.settings.update(cx, |settings, cx| {
            settings.set_archive_state(
                self.library_tree.archived_notes.clone(),
                self.library_tree.error.clone(),
                cx,
            );
        });
        self.sidebar_order.apply(path, &mut self.library_tree.nodes);
        self.library_navigation.clear();
        self.clear_library_selection();
        self.expanded_library_nodes.clear();
        self.scroll_to_selected_library_node = false;
        self.library_scrollbar_state.reset_visibility();
        self.save_interface_state();
        self.update_app_menus_if_reload_state_changed(could_reload_current_note, cx);
        cx.notify();
    }

    fn restart_terminal(&mut self, cx: &mut Context<Self>) {
        let preferences = self.settings.read(cx).preferences().clone();
        self.terminal = preferences.library_path.clone().map(|path| {
            let configuration = terminal_configuration(&preferences);
            cx.new(|cx| TerminalView::new(path, configuration, cx))
        });
        self.terminal_needs_restart = false;
        cx.notify();
    }

    fn restart_mcp_server(&mut self, cx: &mut Context<Self>) {
        let preferences = self.settings.read(cx).preferences().clone();
        let state = self.mcp_server.apply(&preferences);
        self.settings.update(cx, |settings, cx| {
            settings.set_mcp_server_state(state, cx);
        });
    }

    fn apply_library_tree(
        &mut self,
        mut tree: LibraryTree,
        reload_selected_note: bool,
        cx: &mut Context<Self>,
    ) {
        let could_reload_current_note = self.can_reload_current_note();
        if let Some(path) = self.current_library_path(cx).map(Path::to_path_buf) {
            self.sidebar_order.apply(&path, &mut tree.nodes);
        }
        let selected_path = self.selected_library_node.clone();
        let selected_was_document = selected_path.as_deref().is_some_and(|path| {
            self.library_tree
                .node(path)
                .is_some_and(|node| node.document_path.is_some())
        });
        self.settings.update(cx, |settings, cx| {
            settings.set_archive_state(tree.archived_notes.clone(), tree.error.clone(), cx);
        });
        self.library_tree = tree;
        self.library_navigation
            .retain(|path| self.library_tree.node(path).is_some());

        let selection_still_valid = selected_path.as_deref().is_some_and(|path| {
            self.library_tree
                .node(path)
                .is_some_and(|node| !selected_was_document || node.document_path.is_some())
        });
        if selection_still_valid {
            if let Some(path) = selected_path.as_deref() {
                let (has_children, is_document) = self
                    .library_tree
                    .node(path)
                    .map(|node| (!node.children.is_empty(), node.document_path.is_some()))
                    .unwrap_or_default();
                self.expanded_library_nodes =
                    sidebar::expanded_paths_for_selection(path, has_children);
                if is_document && (!selected_was_document || reload_selected_note) {
                    self.begin_selected_note_load(cx);
                }
            }
        } else if selected_path.is_some() {
            self.clear_library_selection();
            self.expanded_library_nodes.clear();
            self.save_interface_state();
        }
        self.update_app_menus_if_reload_state_changed(could_reload_current_note, cx);
        cx.notify();
    }

    fn reload_library_projection(&mut self, cx: &mut Context<Self>) {
        let Some(root) = self.current_library_path(cx).map(Path::to_path_buf) else {
            return;
        };
        cx.spawn(async move |this, cx| {
            let scan_root = root.clone();
            let tree = cx
                .background_spawn(async move { LibraryTree::load(Some(&scan_root)) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.current_library_path(cx) == Some(root.as_path()) {
                    this.apply_library_tree(tree, false, cx);
                }
            });
        })
        .detach();
    }

    fn clear_library_selection(&mut self) {
        self.note_load_generation = self.note_load_generation.wrapping_add(1);
        self.note_load_task = None;
        self.selected_note = None;
        self.selected_library_node = None;
        self.copied_library_node = None;
        self.note_menu_open = false;
        self.note_copy_generation = self.note_copy_generation.wrapping_add(1);
        self.note_text_selection.reset();
        self.note_code_blocks.reset();
        self.note_scroll_handle
            .set_offset(gpui::point(px(0.0), px(0.0)));
        self.note_scrollbar_state.reset_visibility();
    }

    fn begin_selected_note_load(&mut self, cx: &mut Context<Self>) {
        let Some((path, document_path)) = self.selected_library_node.as_deref().and_then(|path| {
            self.library_tree
                .node(path)
                .and_then(|node| node.document_path.clone())
                .map(|document_path| (path.to_owned(), document_path))
        }) else {
            self.note_load_generation = self.note_load_generation.wrapping_add(1);
            self.note_load_task = None;
            self.selected_note = None;
            return;
        };

        let previous_document = self
            .selected_note
            .as_ref()
            .filter(|note| note.path == path)
            .and_then(|note| note.document.clone());
        self.note_load_generation = self.note_load_generation.wrapping_add(1);
        let generation = self.note_load_generation;
        self.selected_note = Some(SelectedNote {
            path: path.clone(),
            document: previous_document,
            error: None,
        });
        self.note_load_task = Some(Self::spawn_note_load(path, document_path, generation, cx));
        cx.notify();
    }

    pub(crate) fn can_reload_current_note(&self) -> bool {
        self.selected_library_node.as_deref().is_some_and(|path| {
            self.library_tree
                .node(path)
                .is_some_and(|node| node.document_path.is_some())
        })
    }

    fn update_app_menus_if_reload_state_changed(
        &self,
        could_reload_current_note: bool,
        cx: &mut Context<Self>,
    ) {
        let can_reload_current_note = self.can_reload_current_note();
        if can_reload_current_note != could_reload_current_note {
            let language = self.language(cx);
            crate::set_app_menus(cx, language, can_reload_current_note);
        }
    }

    fn interface_state(&self) -> InterfaceState {
        let shell = self.app_shell.workspace().snapshot();
        InterfaceState {
            left_sidebar_width: shell.left_width,
            right_sidebar_width: shell.right_width,
            left_sidebar_visible: shell.left_visible,
            right_sidebar_visible: shell.right_visible,
            fps_counter_visible: self.app_shell.fps_counter().visible(),
            selected_library_node: self.selected_library_node.clone(),
            window_bounds: self.window_bounds,
        }
    }

    fn save_interface_state(&self) {
        if let Err(error) = self.interface_state().save() {
            eprintln!("无法保存界面状态：{error}");
        }
    }

    fn toggle_left_sidebar_action(
        &mut self,
        _: &ToggleLeftPanel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_panel(PanelSide::Left, window, cx);
    }

    fn toggle_right_sidebar_action(
        &mut self,
        _: &ToggleRightPanel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_panel(PanelSide::Right, window, cx);
    }

    fn navigate_back(&mut self, cx: &mut Context<Self>) {
        let current = self.selected_library_node.clone();
        let Some(path) = self.library_navigation.go_back(current.as_deref()) else {
            return;
        };
        self.restore_library_selection(path, cx);
    }

    fn navigate_forward(&mut self, cx: &mut Context<Self>) {
        let current = self.selected_library_node.clone();
        let Some(path) = self.library_navigation.go_forward(current.as_deref()) else {
            return;
        };
        self.restore_library_selection(path, cx);
    }

    fn navigation_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.app_shell.is_settings_open() {
            return;
        }
        match event.button {
            MouseButton::Navigate(NavigationDirection::Back) => {
                cx.stop_propagation();
                self.navigate_back(cx);
            }
            MouseButton::Navigate(NavigationDirection::Forward) => {
                cx.stop_propagation();
                self.navigate_forward(cx);
            }
            _ => {}
        }
    }

    fn language(&self, cx: &Context<Self>) -> Language {
        self.settings.read(cx).preferences().language
    }

    fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.app_shell.open_settings();
        self.settings
            .update(cx, |settings, cx| settings.focus(window, cx));
        cx.notify();
    }

    fn open_settings_action(
        &mut self,
        _: &OpenSettings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_settings(window, cx);
    }

    fn close_window_action(
        &mut self,
        _: &CloseWindow,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        oneo_gpui_ui::close_main_window(window, cx);
    }

    fn reload_current_note_action(
        &mut self,
        _: &ReloadCurrentNote,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.begin_selected_note_load(cx);
    }

    fn toggle_fps_counter_action(
        &mut self,
        _: &ToggleFpsCounter,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.app_shell.fps_counter_mut().toggle();
        self.save_interface_state();
        cx.notify();
    }

    fn render_panel_resize_handle(&self, side: PanelSide, cx: &mut Context<Self>) -> Stateful<Div> {
        let theme = Theme::current(cx);
        let active = self.app_shell.workspace().resize_active(side);

        panel_resize_handle(
            match side {
                PanelSide::Left => "left-sidebar-resize-handle",
                PanelSide::Right => "right-sidebar-resize-handle",
            },
            side,
            active,
            theme,
        )
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                if event.click_count == 2 {
                    this.reset_panel_width(side, cx);
                    cx.stop_propagation();
                    return;
                }
                this.app_shell
                    .workspace_mut()
                    .begin_resize(side, f32::from(event.position.x));
                cx.stop_propagation();
                cx.notify();
            }),
        )
    }

    fn reset_panel_width(&mut self, side: PanelSide, cx: &mut Context<Self>) {
        let width = match side {
            PanelSide::Left => DEFAULT_LEFT_SIDEBAR_WIDTH,
            PanelSide::Right => DEFAULT_RIGHT_SIDEBAR_WIDTH,
        };
        self.app_shell.workspace_mut().reset_width(side, width);
        self.save_interface_state();
        cx.notify();
    }

    fn resize_panel_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.app_shell.workspace_mut().resize(
            f32::from(event.position.x),
            f32::from(window.viewport_size().width),
        ) {
            cx.notify();
        }
    }

    fn finish_panel_resize(
        &mut self,
        event: &MouseUpEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.button != MouseButton::Left {
            return;
        }
        let panel_resized = self.app_shell.workspace_mut().finish_resize();
        let library_drag_ended = self.dragged_library_node.take().is_some();
        if panel_resized {
            self.save_interface_state();
        }
        if panel_resized || library_drag_ended {
            cx.notify();
        }
    }

    fn render_left_sidebar(
        &self,
        width: f32,
        show_resize: bool,
        left_control_clearance: f32,
        cx: &mut Context<Self>,
    ) -> Div {
        let language = self.language(cx);
        div()
            .relative()
            .w(px(width))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .bg(transparent_black())
            .child(self.render_left_titlebar(left_control_clearance, cx))
            .child(
                div()
                    .min_h_0()
                    .flex_1()
                    .child(self.render_library_sidebar(language, cx)),
            )
            .child(
                div()
                    .h(px(40.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(10.0))
                    .child(self.render_mcp_status(cx))
                    .child(self.render_settings_button(cx)),
            )
            .when(show_resize, |panel| {
                panel.child(self.render_panel_resize_handle(PanelSide::Left, cx))
            })
    }

    fn render_content(
        &self,
        left_control_clearance: f32,
        content_width: f32,
        cx: &mut Context<Self>,
    ) -> Div {
        let theme = Theme::current(cx);
        let selected_note = self
            .selected_library_node
            .as_deref()
            .and_then(|path| self.library_tree.node(path))
            .filter(|node| node.document_path.is_some());

        div()
            .relative()
            .min_w_0()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.surface)
            .when_some(selected_note, |content, node| {
                content.child(self.render_note(
                    node,
                    self.selected_note.as_ref(),
                    left_control_clearance,
                    content_width,
                    cx,
                ))
            })
            .when(selected_note.is_none(), |content| {
                content
                    .child(self.render_content_titlebar(left_control_clearance, cx))
                    .child(div().min_h_0().flex_1())
            })
    }

    fn render_right_sidebar(&self, width: f32, show_resize: bool, cx: &mut Context<Self>) -> Div {
        let theme = Theme::current(cx);
        let language = self.language(cx);

        div()
            .relative()
            .w(px(width))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .min_w_0()
            .bg(theme.surface)
            .child(self.render_right_titlebar(cx))
            .child(
                div()
                    .min_h_0()
                    .flex_1()
                    .when_some(self.terminal.clone(), |body, terminal| body.child(terminal))
                    .when(self.terminal.is_none(), |body| {
                        body.flex()
                            .items_center()
                            .justify_center()
                            .px(px(24.0))
                            .text_sm()
                            .text_color(theme.text_secondary)
                            .child(language.text(
                                "请先在设置中选择文档库",
                                "Choose a library in Settings first",
                            ))
                    }),
            )
            .when(show_resize, |panel| {
                panel.child(self.render_panel_resize_handle(PanelSide::Right, cx))
            })
    }

    fn left_pane_content(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let workspace = self.app_shell.workspace();
        let left_content =
            workspace.panel_content_width(PanelSide::Left, f32::from(window.viewport_size().width));
        let left_control_clearance = left_titlebar_control_clearance(window.is_fullscreen());
        self.render_left_sidebar(
            left_content,
            workspace.visible(PanelSide::Left) && !workspace.transitioning(PanelSide::Left),
            left_control_clearance,
            cx,
        )
        .into_any_element()
    }

    fn content_pane_content(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let left_control_clearance = left_titlebar_control_clearance(window.is_fullscreen());
        let content_width = f32::from(window.viewport_size().width)
            - self.app_shell.workspace().rendered_width(PanelSide::Left)
            - self.app_shell.workspace().rendered_width(PanelSide::Right);
        self.render_content(left_control_clearance, content_width, cx)
            .into_any_element()
    }

    fn right_pane_content(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let workspace = self.app_shell.workspace();
        let right_content = workspace
            .panel_content_width(PanelSide::Right, f32::from(window.viewport_size().width));
        self.render_right_sidebar(
            right_content,
            workspace.visible(PanelSide::Right) && !workspace.transitioning(PanelSide::Right),
            cx,
        )
        .into_any_element()
    }
}

impl Render for Locus {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::current(cx);
        let ui_font_family = self
            .settings
            .read(cx)
            .preferences()
            .resolved_ui_font_family()
            .to_owned();
        let layout = self.app_shell.workspace_mut().layout(window);
        let panels = layout.frame;
        let left_toggle_offset = left_toggle_offset(window.is_fullscreen());
        let navigation_left_offset = navigation_left_offset(
            left_toggle_offset,
            panels.left,
            self.app_shell.workspace().visible(PanelSide::Left),
        );
        if let Some(terminal) = &self.terminal {
            terminal.update(cx, |terminal, _| {
                terminal.set_available_size(
                    panels.right_content,
                    (f32::from(window.viewport_size().height) - TITLEBAR_HEIGHT).max(120.0),
                );
            });
        }
        if layout.left_transition_finished {
            self.left_pane.update(cx, |_, cx| cx.notify());
        }
        if layout.right_transition_finished {
            self.right_pane.update(cx, |_, cx| cx.notify());
        }
        if !self.app_shell.is_settings_open() && self.app_shell.fps_counter_mut().tick(window) {
            self.content_pane.update(cx, |_, cx| cx.notify());
        }
        if panels.left > 0.0 && self.scroll_to_selected_library_node {
            self.selected_library_scroll_anchor.scroll_to(window, cx);
            self.library_scrollbar_state.reset_visibility();
            self.scroll_to_selected_library_node = false;
        }

        let workspace = WorkspaceShell::new(
            panels,
            self.left_pane
                .clone()
                .cached(StyleRefinement::default().size_full())
                .into_any_element(),
            self.content_pane
                .clone()
                .cached(StyleRefinement::default().size_full())
                .into_any_element(),
            self.right_pane
                .clone()
                .cached(StyleRefinement::default().size_full())
                .into_any_element(),
            self.render_panel_toggle(PanelSide::Left, cx)
                .into_any_element(),
            self.render_panel_toggle(PanelSide::Right, cx)
                .into_any_element(),
        )
        .left_toggle_offset(left_toggle_offset)
        .control_inset(TITLEBAR_CONTROL_INSET)
        .sidebar_divider_color(theme.sidebar_border)
        .into_element()
        .child(
            div()
                .absolute()
                .top(px(TITLEBAR_CONTROL_INSET))
                .left(px(navigation_left_offset))
                .flex()
                .items_center()
                .gap(px(TITLEBAR_NAVIGATION_GAP))
                .child(self.render_history_button(NavigationDirection::Back, cx))
                .child(self.render_history_button(NavigationDirection::Forward, cx)),
        );

        div()
            .key_context("Locus")
            .track_focus(&self.root_focus)
            .on_action(cx.listener(Self::close_window_action))
            .on_action(cx.listener(Self::open_settings_action))
            .on_action(cx.listener(Self::reload_current_note_action))
            .on_action(cx.listener(Self::toggle_fps_counter_action))
            .on_action(cx.listener(Self::toggle_left_sidebar_action))
            .on_action(cx.listener(Self::toggle_right_sidebar_action))
            .capture_any_mouse_down(cx.listener(Self::navigation_mouse_down))
            .on_mouse_move(cx.listener(Self::resize_panel_mouse_move))
            .capture_any_mouse_up(cx.listener(Self::finish_panel_resize))
            .size_full()
            .relative()
            .flex()
            .bg(theme.window_background)
            .font_family(ui_font_family)
            .text_color(theme.text)
            .when(self.app_shell.is_settings_open(), |root| {
                root.child(
                    self.settings
                        .clone()
                        .cached(StyleRefinement::default().size_full()),
                )
            })
            .when(!self.app_shell.is_settings_open(), |root| {
                root.child(workspace)
            })
    }
}

fn merge_library_watch_event(
    event: LibraryWatchEvent,
    changed_paths: &mut HashSet<PathBuf>,
    rescan_all: &mut bool,
    error: &mut Option<String>,
) {
    match event {
        LibraryWatchEvent::Changed {
            paths,
            rescan_all: event_rescan_all,
        } => {
            changed_paths.extend(paths);
            *rescan_all |= event_rescan_all;
        }
        LibraryWatchEvent::Error(next_error) => *error = Some(next_error),
    }
}

fn event_affects_document(
    library_root: &Path,
    document_path: &Path,
    changed_paths: &HashSet<PathBuf>,
    rescan_all: bool,
) -> bool {
    if rescan_all {
        return true;
    }
    let Ok(relative_document_path) = document_path.strip_prefix(library_root) else {
        return false;
    };
    changed_paths.iter().any(|changed_path| {
        changed_path.as_os_str().is_empty()
            || changed_path == relative_document_path
            || relative_document_path.starts_with(changed_path)
    })
}

fn left_toggle_offset(fullscreen: bool) -> f32 {
    titlebar_left_control_offset(fullscreen)
}

fn left_titlebar_control_clearance(fullscreen: bool) -> f32 {
    left_toggle_offset(fullscreen)
        + TITLEBAR_CONTROL_SIZE
        + TITLEBAR_NAVIGATION_INSET
        + TITLEBAR_NAVIGATION_WIDTH
        + TITLEBAR_CONTROL_GAP
}

fn navigation_left_offset(
    left_toggle_offset: f32,
    rendered_left_width: f32,
    left_sidebar_visible: bool,
) -> f32 {
    let collapsed_offset = left_toggle_offset + TITLEBAR_CONTROL_SIZE + TITLEBAR_NAVIGATION_INSET;
    if !left_sidebar_visible {
        return collapsed_offset;
    }
    (rendered_left_width - TITLEBAR_CONTROL_INSET - TITLEBAR_NAVIGATION_WIDTH).max(collapsed_offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fullscreen_left_toggle_uses_the_window_inset() {
        assert_eq!(left_toggle_offset(true), TITLEBAR_CONTROL_INSET);
    }

    #[test]
    fn titlebar_clearance_includes_navigation_buttons() {
        assert_eq!(
            left_titlebar_control_clearance(true),
            TITLEBAR_CONTROL_INSET
                + TITLEBAR_CONTROL_SIZE
                + TITLEBAR_NAVIGATION_INSET
                + TITLEBAR_NAVIGATION_WIDTH
                + TITLEBAR_CONTROL_GAP
        );
    }

    #[test]
    fn navigation_buttons_follow_the_sidebar_right_edge_when_expanded() {
        assert_eq!(
            navigation_left_offset(TITLEBAR_CONTROL_INSET, 280.0, true),
            280.0 - TITLEBAR_CONTROL_INSET - TITLEBAR_NAVIGATION_WIDTH
        );
    }

    #[test]
    fn navigation_buttons_follow_the_toggle_when_the_sidebar_is_collapsed() {
        assert_eq!(
            navigation_left_offset(TITLEBAR_CONTROL_INSET, 0.0, false),
            TITLEBAR_CONTROL_INSET + TITLEBAR_CONTROL_SIZE + TITLEBAR_NAVIGATION_INSET
        );
    }

    #[test]
    fn note_events_match_the_document_or_its_parent() {
        let root = Path::new("/library");
        let document = Path::new("/library/notes/index.md");

        assert!(event_affects_document(
            root,
            document,
            &HashSet::from([PathBuf::from("notes/index.md")]),
            false
        ));
        assert!(event_affects_document(
            root,
            document,
            &HashSet::from([PathBuf::from("notes")]),
            false
        ));
        assert!(!event_affects_document(
            root,
            document,
            &HashSet::from([PathBuf::from("notes/image.png")]),
            false
        ));
        assert!(!event_affects_document(
            root,
            document,
            &HashSet::from([PathBuf::from(".git/index")]),
            false
        ));
        assert!(event_affects_document(
            root,
            document,
            &HashSet::new(),
            true
        ));
    }

    #[test]
    fn watcher_events_merge_paths_and_full_rescan_state() {
        let mut paths = HashSet::new();
        let mut rescan_all = false;
        let mut error = None;

        merge_library_watch_event(
            LibraryWatchEvent::Changed {
                paths: vec![PathBuf::from("notes/index.md")],
                rescan_all: false,
            },
            &mut paths,
            &mut rescan_all,
            &mut error,
        );
        merge_library_watch_event(
            LibraryWatchEvent::Changed {
                paths: Vec::new(),
                rescan_all: true,
            },
            &mut paths,
            &mut rescan_all,
            &mut error,
        );

        assert_eq!(paths, HashSet::from([PathBuf::from("notes/index.md")]));
        assert!(rescan_all);
        assert_eq!(error, None);
    }
}
