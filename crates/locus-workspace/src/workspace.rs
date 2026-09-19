use rust_i18n::t;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

mod canvas;
mod command_palette;
mod navigation;
mod note;
mod note_atmosphere;
mod note_menu;
mod pane;
mod sidebar;
mod terminal;
mod window_chrome;

use gpui_kit::component::{ElementExt as _, Root};
use gpui_kit::{
    AnyElement, App, Bounds, Context, Div, Entity, FocusHandle, FontWeight, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, NavigationDirection, Pixels, Render,
    ScrollHandle, Stateful, StyleRefinement, Subscription, Task, WeakFocusHandle, Window, div,
    prelude::*, px,
};

use self::canvas::NoteCanvas;
use self::navigation::LibraryNavigation;
use self::pane::LocusPane;
use crate::sidebar_order::SidebarOrderState;
use crate::state::{
    InterfaceState, InterfaceStateWriter, LEFT_SIDEBAR_MAX_WIDTH, LEFT_SIDEBAR_MIN_WIDTH,
};
use crate::{NewTerminal, OpenSettings, ReloadCurrentNote, ToggleLeftPanel};
use gpui_kit::component::Theme;
use locus_library::{LibraryTree, LibraryWatchEvent, LibraryWatcher, load_note};
use locus_mcp::{McpConfiguration, McpServerController};
use locus_preferences::Preferences;
use locus_preferences::config::DEFAULT_LEFT_SIDEBAR_WIDTH;
use locus_settings::{Settings, SettingsEvent, SettingsInit};
use locus_ui::{Icon, MarkdownCodeBlocks, MarkdownDocument, MarkdownSelection};
use oneo_gpui::Language;
use oneo_gpui::{
    AppShellState, CloseWindow, PanelSize, TITLEBAR_CONTROL_INSET, TITLEBAR_CONTROL_SIZE,
    TITLEBAR_HEIGHT, WorkspaceShell, WorkspaceShellConfig, WorkspaceShellState,
    panel_resize_handle, titlebar_content_left_padding, titlebar_left_control_offset,
};

const TITLEBAR_CONTROL_GAP: f32 = 8.0;
const TITLEBAR_NAVIGATION_INSET: f32 = 6.0;
const TITLEBAR_NAVIGATION_GAP: f32 = 2.0;

fn mcp_configuration(preferences: &Preferences) -> McpConfiguration {
    McpConfiguration {
        library_path: preferences.library_path.clone(),
        enabled: preferences.mcp_enabled,
        local_network: preferences.mcp_local_network,
        port: preferences.mcp_port,
    }
}

const TITLEBAR_NAVIGATION_WIDTH: f32 = TITLEBAR_CONTROL_SIZE * 2.0 + TITLEBAR_NAVIGATION_GAP;
const WORKSPACE_SHELL_CONFIG: WorkspaceShellConfig = WorkspaceShellConfig {
    content_min_width: locus_preferences::config::CONTENT_MIN_WIDTH,
    left: PanelSize {
        min: LEFT_SIDEBAR_MIN_WIDTH,
        max: LEFT_SIDEBAR_MAX_WIDTH,
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
    window_handle: gpui_kit::AnyWindowHandle,
    root_focus: FocusHandle,
    app_shell: AppShellState,
    settings: Entity<Settings>,
    left_pane: Entity<LocusPane>,
    content_pane: Entity<LocusPane>,
    mcp_server: McpServerController,
    note_canvas: Option<NoteCanvas>,
    canvas_download_pending: bool,
    terminal_launch_pending: bool,
    library_tree: LibraryTree,
    library_watcher: Option<LibraryWatcher>,
    library_watch_task: Option<Task<()>>,
    library_watcher_error: Option<String>,
    sidebar_order: SidebarOrderState,
    dragged_library_node: Option<String>,
    library_reorder_animation: Option<sidebar::LibraryReorderAnimation>,
    expanded_library_nodes: HashSet<String>,
    selected_library_node: Option<String>,
    library_navigation: LibraryNavigation,
    selected_note: Option<SelectedNote>,
    note_load_generation: u64,
    note_load_task: Option<Task<()>>,
    library_tree_state: Entity<gpui_kit::base::TreeState>,
    scroll_to_selected_library_node: bool,
    note_text_focus: FocusHandle,
    note_text_selection: MarkdownSelection,
    note_code_blocks: MarkdownCodeBlocks,
    command_palette: Entity<gpui_kit::component::command::CommandState>,
    command_palette_overlay_focus: FocusHandle,
    command_palette_state: command_palette::CommandPaletteState,
    command_palette_close_generation: u64,
    command_palette_previous_focus: Option<WeakFocusHandle>,
    pending_note_action: Option<PathBuf>,
    copied_library_node: Option<String>,
    note_copy_generation: u64,
    note_scroll_handle: ScrollHandle,
    window_bounds: Option<Bounds<Pixels>>,
    window_bounds_save_task: Option<Task<()>>,
    interface_state_writer: InterfaceStateWriter,
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
        let command_palette =
            cx.new(|cx| gpui_kit::component::command::CommandState::new(window, cx));
        window.focus(&root_focus, cx);
        preferences.appearance.apply(window, cx);
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
        let library_tree_state = cx.new(|cx| {
            let mut state = gpui_kit::base::TreeState::new(cx).items(sidebar::library_tree_items(
                &library_tree.nodes,
                &expanded_library_nodes,
            ));
            let ix = selected_library_node
                .as_ref()
                .and_then(|path| state.index_of(&path.clone().into()));
            state.set_selected_index(ix, cx);
            state
        });
        Self::observe_library_tree(&library_tree_state, cx);
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
        let mcp_server = McpServerController::new(&mcp_configuration(&preferences));
        let mcp_server_state = mcp_server.state();
        let archived_notes = library_tree.archived_notes.clone();
        let archive_load_error = library_tree.error.clone();
        let settings = cx.new(|cx| {
            Settings::new(
                window,
                SettingsInit {
                    preferences,
                    load_error,
                    mcp_server_state,
                    archived_notes,
                    archive_load_error,
                },
                root_focus.clone(),
                cx,
            )
        });
        cx.subscribe(
            &settings,
            |this, _, event: &SettingsEvent, cx| match event {
                SettingsEvent::Close => {
                    this.app_shell.close_settings();
                    cx.notify();
                }
                SettingsEvent::ArchiveRestored => {
                    this.reload_library_projection(cx);
                }
                SettingsEvent::LanguageChanged(language) => {
                    language.activate();
                    crate::set_app_menus(cx, *language, this.can_reload_current_note());
                    cx.notify();
                }
                SettingsEvent::LibraryChanged(path) => {
                    this.change_library(path, cx);
                    this.restart_mcp_server(cx);
                }
                SettingsEvent::UiFontChanged | SettingsEvent::CodeFontChanged => {
                    cx.notify();
                }
                SettingsEvent::McpConfigurationChanged => {
                    this.restart_mcp_server(cx);
                }
            },
        )
        .detach();
        oneo_gpui::appearance::observe_appearance(window, cx, |this, cx| {
            this.settings.read(cx).preferences().appearance
        })
        .detach();
        cx.observe_window_bounds(window, |this, window, cx| {
            this.window_bounds = Some(window.inner_window_bounds().get_bounds());
            this.window_bounds_save_task = Some(cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(WINDOW_BOUNDS_SAVE_DEBOUNCE)
                    .await;
                this.update(cx, |this, cx| {
                    this.save_interface_state(cx);
                })
                .ok();
            }));
        })
        .detach();
        cx.on_app_quit(|this, cx| {
            this.window_bounds_save_task.take();
            this.save_interface_state(cx);
            let pending_save = this.interface_state_writer.finish();
            async move {
                if let Some(task) = pending_save {
                    task.await;
                }
            }
        })
        .detach();

        cx.defer_in(window, |this, window, cx| {
            this.apply_note_canvas_selection(window, cx)
        });

        Self {
            window_handle: window.window_handle(),
            root_focus,
            app_shell: AppShellState::new(WorkspaceShellState::new(
                WORKSPACE_SHELL_CONFIG,
                interface_state.left_sidebar_visible,
                interface_state.left_sidebar_width,
            )),
            settings,
            left_pane,
            content_pane,
            mcp_server,
            note_canvas: None,
            canvas_download_pending: false,
            terminal_launch_pending: false,
            library_tree,
            library_watcher,
            library_watch_task,
            library_watcher_error,
            sidebar_order,
            dragged_library_node: None,
            library_reorder_animation: None,
            expanded_library_nodes,
            selected_library_node,
            library_navigation: LibraryNavigation::default(),
            selected_note,
            note_load_generation,
            note_load_task,
            library_tree_state,
            scroll_to_selected_library_node,
            note_text_focus: cx.focus_handle(),
            note_text_selection: MarkdownSelection::default(),
            note_code_blocks: MarkdownCodeBlocks::default(),
            command_palette,
            command_palette_overlay_focus: cx.focus_handle(),
            command_palette_state: command_palette::CommandPaletteState::Hidden,
            command_palette_close_generation: 0,
            command_palette_previous_focus: None,
            pending_note_action: None,
            copied_library_node: None,
            note_copy_generation: 0,
            note_scroll_handle: ScrollHandle::new(),
            window_bounds: interface_state.window_bounds,
            window_bounds_save_task: None,
            interface_state_writer: InterfaceStateWriter::new(),
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
                if !this
                    .selected_note
                    .as_ref()
                    .is_some_and(|note| note.path == path)
                {
                    return;
                }
                match result {
                    Ok(document) => {
                        document.synchronize_interaction_state(
                            &this.note_text_selection,
                            &this.note_code_blocks,
                        );
                        let note = this.selected_note.as_mut().expect("selected note checked");
                        note.document = Some(document);
                        note.error = None;
                    }
                    Err(error) => {
                        let note = this.selected_note.as_mut().expect("selected note checked");
                        note.error = Some(error);
                    }
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
        self.release_note_canvas(cx);
        self.library_watch_task = None;
        self.library_watcher = None;
        let (watcher, task, watcher_error) = Self::start_library_watcher(Some(path), cx);
        self.library_watcher = watcher;
        self.library_watch_task = task;
        self.library_watcher_error = watcher_error;
        self.library_reorder_animation = None;
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
        self.clear_library_selection(cx);
        self.expanded_library_nodes.clear();
        self.sync_library_tree(cx);
        self.scroll_to_selected_library_node = false;
        self.save_interface_state(cx);
        self.update_app_menus_if_reload_state_changed(could_reload_current_note, cx);
        cx.notify();
    }

    fn restart_mcp_server(&mut self, cx: &mut Context<Self>) {
        let preferences = self.settings.read(cx).preferences().clone();
        let state = self.mcp_server.apply(&mcp_configuration(&preferences));
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
        self.library_reorder_animation = None;
        self.library_tree = tree;
        self.expanded_library_nodes.retain(|path| {
            self.library_tree
                .node(path)
                .is_some_and(|node| !node.children.is_empty())
        });
        self.library_navigation
            .retain(|path| self.library_tree.node(path).is_some());

        let selection_still_valid = selected_path.as_deref().is_some_and(|path| {
            self.library_tree
                .node(path)
                .is_some_and(|node| !selected_was_document || node.document_path.is_some())
        });
        if selection_still_valid {
            if let Some(path) = selected_path.as_deref() {
                let is_document = self
                    .library_tree
                    .node(path)
                    .is_some_and(|node| node.document_path.is_some());
                if is_document && (!selected_was_document || reload_selected_note) {
                    self.begin_selected_note_load(cx);
                }
            }
        } else if selected_path.is_some() {
            self.clear_library_selection(cx);
            self.expanded_library_nodes.clear();
            self.save_interface_state(cx);
        }
        self.sync_library_tree(cx);
        self.schedule_note_canvas_selection(cx);
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

    fn clear_library_selection(&mut self, cx: &mut Context<Self>) {
        self.note_load_generation = self.note_load_generation.wrapping_add(1);
        self.note_load_task = None;
        self.selected_note = None;
        self.selected_library_node = None;
        self.schedule_note_canvas_selection(cx);
        self.copied_library_node = None;
        self.note_copy_generation = self.note_copy_generation.wrapping_add(1);
        self.note_text_selection.reset();
        self.note_code_blocks.reset();
        self.note_scroll_handle
            .set_offset(gpui_kit::point(px(0.0), px(0.0)));
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

    pub fn can_reload_current_note(&self) -> bool {
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
            left_sidebar_visible: shell.left_visible,
            selected_library_node: self.selected_library_node.clone(),
            window_bounds: self.window_bounds,
        }
    }

    fn save_interface_state(&mut self, cx: &Context<Self>) {
        self.interface_state_writer
            .save(self.interface_state(), cx.background_executor());
    }

    fn toggle_left_sidebar_action(
        &mut self,
        _: &ToggleLeftPanel,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_panel(cx);
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
        self.settings.read(cx).preferences().appearance.language
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

    fn reload_current_note_action(
        &mut self,
        _: &ReloadCurrentNote,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.reload_focused_canvas(window, cx) {
            return;
        }
        self.begin_selected_note_load(cx);
    }

    fn render_panel_resize_handle(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let theme = Theme::global(cx).clone();
        let active = self.app_shell.workspace().resize_active();

        panel_resize_handle("left-sidebar-resize-handle", active, &theme).on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                if event.click_count == 2 {
                    this.reset_panel_width(cx);
                    cx.stop_propagation();
                    return;
                }
                this.app_shell
                    .workspace_mut()
                    .begin_resize(f32::from(event.position.x));
                cx.stop_propagation();
                cx.notify();
            }),
        )
    }

    fn reset_panel_width(&mut self, cx: &mut Context<Self>) {
        let width = DEFAULT_LEFT_SIDEBAR_WIDTH;
        self.app_shell.workspace_mut().reset_width(width);
        self.save_interface_state(cx);
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
            self.save_interface_state(cx);
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
        oneo_gpui::WorkspaceSidebar::new(
            self.render_library_sidebar(language, cx),
            Theme::global(cx),
            left_control_clearance,
        )
        .footer_start(self.render_mcp_status(cx))
        .into_element()
        .w(px(width))
        .flex_none()
        .bg(oneo_gpui::appearance::theme::workspace_background(
            Theme::global(cx),
        ))
        .when(show_resize, |panel| {
            panel.child(self.render_panel_resize_handle(cx))
        })
    }

    fn render_content(
        &self,
        left_control_clearance: f32,
        content_width: f32,
        cx: &mut Context<Self>,
    ) -> Div {
        let theme = Theme::global(cx).clone();
        let selected_note = self
            .selected_library_node
            .as_deref()
            .and_then(|path| self.library_tree.node(path))
            .filter(|node| node.document_path.is_some());

        let panel = if let Some(node) = selected_note {
            let note = self.render_note(
                node,
                self.selected_note.as_ref(),
                left_control_clearance,
                content_width,
                cx,
            );
            oneo_gpui::WorkspacePanel::new(note)
        } else {
            oneo_gpui::WorkspacePanel::new(div().size_full())
                .header(self.render_content_titlebar(left_control_clearance, cx))
        };
        let native_canvas = cfg!(target_os = "macos")
            && selected_note.is_some_and(|node| node.canvas_path.is_some());
        panel
            .into_element()
            .when(!native_canvas, |panel| panel.bg(theme.background))
    }

    fn left_pane_content(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let workspace = self.app_shell.workspace();
        let left_content = workspace.panel_content_width(f32::from(window.viewport_size().width));
        let left_control_clearance = left_titlebar_control_clearance(window.is_fullscreen());
        self.render_left_sidebar(
            left_content,
            workspace.visible() && !workspace.transitioning(),
            left_control_clearance,
            cx,
        )
        .into_any_element()
    }

    fn content_pane_content(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let left_control_clearance = left_titlebar_control_clearance(window.is_fullscreen());
        let content_width =
            f32::from(window.viewport_size().width) - self.app_shell.workspace().rendered_width();
        self.render_content(left_control_clearance, content_width, cx)
            .into_any_element()
    }

    fn advance_render_lifecycle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self
            .library_reorder_animation
            .as_ref()
            .is_some_and(|animation| {
                cx.reduce_motion() || animation.is_finished(std::time::Instant::now())
            })
        {
            self.library_reorder_animation = None;
        }
        let layout = self.app_shell.workspace_mut().layout(window);
        self.synchronize_canvases(window, cx);
        let panels = layout.frame;
        if layout.left_transition_finished {
            self.left_pane.update(cx, |_, cx| cx.notify());
        }
        if panels.left > 0.0 && self.scroll_to_selected_library_node {
            self.library_tree_state.update(cx, |state, _| {
                if let Some(ix) = state.selected_index() {
                    state.scroll_to_item(ix, gpui_kit::ScrollStrategy::Center);
                }
            });
            self.scroll_to_selected_library_node = false;
        }
    }
}

impl Render for Locus {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sheet_layer = Root::render_sheet_layer(window, cx);
        let dialog_layer = Root::render_dialog_layer(window, cx);
        let notification_layer = Root::render_notification_layer(window, cx);
        let command_palette_overlay = self.render_command_palette_overlay(window, cx);
        let theme = Theme::global(cx).clone();
        let ui_font_family = self
            .settings
            .read(cx)
            .preferences()
            .resolved_ui_font_family()
            .to_owned();
        let panels = self.app_shell.workspace().frame(window);
        let left_toggle_offset = left_toggle_offset(window.is_fullscreen());
        let navigation_left_offset = navigation_left_offset(
            left_toggle_offset,
            panels.left,
            self.app_shell.workspace().visible(),
        );
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
            window,
            cx,
        )
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

        let locus = cx.weak_entity();
        div()
            .key_context("Locus")
            .track_focus(&self.root_focus)
            .capture_key_down(crate::commands::capture_terminal_shortcut)
            .on_action(cx.listener(Self::new_terminal_action))
            .on_action(cx.listener(Self::open_command_palette_action))
            .on_action(cx.listener(Self::open_note_action))
            .on_action(cx.listener(Self::open_settings_action))
            .on_action(cx.listener(Self::copy_current_note_reference_action))
            .on_action(cx.listener(Self::edit_current_note_action))
            .on_action(cx.listener(Self::open_current_note_directory_action))
            .on_action(cx.listener(Self::choose_current_note_icon_action))
            .on_action(cx.listener(Self::archive_current_note_action))
            .on_action(cx.listener(Self::reload_current_note_action))
            .on_action(cx.listener(Self::toggle_left_sidebar_action))
            .capture_any_mouse_down(cx.listener(Self::navigation_mouse_down))
            .child(workspace_drag_events(
                cx.listener(Self::resize_panel_mouse_move),
                cx.listener(Self::finish_panel_resize),
            ))
            .size_full()
            .relative()
            .flex()
            .on_prepaint(move |_, window, cx| {
                let _ = locus.update(cx, |this, cx| {
                    this.advance_render_lifecycle(window, cx);
                });
            })
            .when(
                !cfg!(target_os = "macos") || self.app_shell.is_settings_open(),
                |root| root.bg(oneo_gpui::appearance::theme::workspace_background(&theme)),
            )
            .font_family(ui_font_family)
            .text_color(theme.foreground)
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
            .children(sheet_layer)
            .children(command_palette_overlay)
            .children(dialog_layer)
            .children(notification_layer)
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

fn workspace_drag_events(
    on_move: impl Fn(&MouseMoveEvent, &mut Window, &mut App) + 'static,
    on_up: impl Fn(&MouseUpEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    gpui_kit::canvas(
        |_, _, _| (),
        move |_, _, window, _| {
            window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                if phase.capture() {
                    on_move(event, window, cx);
                }
            });
            window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                if phase.capture() {
                    on_up(event, window, cx);
                }
            });
        },
    )
    .absolute()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui_kit::test]
    fn sidebar_drag_crosses_occluding_content_and_stops_on_release(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        struct Harness {
            shell: WorkspaceShellState,
        }
        impl Render for Harness {
            fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
                div()
                    .size_full()
                    .child(workspace_drag_events(
                        cx.listener(|this: &mut Self, event: &MouseMoveEvent, _, cx| {
                            if this.shell.resize(f32::from(event.position.x), 1200.) {
                                cx.notify();
                            }
                        }),
                        cx.listener(|this, _, _, cx| {
                            this.shell.finish_resize();
                            cx.notify();
                        }),
                    ))
                    .child(div().w(px(50.)).h_full().on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, event: &MouseDownEvent, _, cx| {
                            this.shell.begin_resize(f32::from(event.position.x));
                            cx.notify();
                        }),
                    ))
                    .child(
                        div()
                            .absolute()
                            .left(px(50.))
                            .top_0()
                            .w(px(600.))
                            .h_full()
                            .occlude(),
                    )
            }
        }
        let (view, cx) = cx.add_window_view(|_, _| Harness {
            shell: WorkspaceShellState::default(),
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let initial = view.read_with(cx, |v, _| v.shell.snapshot().left_width);
        cx.simulate_mouse_down(
            gpui_kit::point(px(25.), px(25.)),
            MouseButton::Left,
            gpui_kit::Modifiers::default(),
        );
        cx.simulate_mouse_move(
            gpui_kit::point(px(125.), px(25.)),
            Some(MouseButton::Left),
            gpui_kit::Modifiers::default(),
        );
        assert_eq!(
            view.read_with(cx, |v, _| v.shell.snapshot().left_width),
            initial + 100.
        );
        cx.simulate_mouse_move(
            gpui_kit::point(px(75.), px(25.)),
            Some(MouseButton::Left),
            gpui_kit::Modifiers::default(),
        );
        assert_eq!(
            view.read_with(cx, |v, _| v.shell.snapshot().left_width),
            initial + 50.
        );
        cx.simulate_mouse_up(
            gpui_kit::point(px(75.), px(25.)),
            MouseButton::Left,
            gpui_kit::Modifiers::default(),
        );
        cx.simulate_mouse_move(
            gpui_kit::point(px(200.), px(25.)),
            None,
            gpui_kit::Modifiers::default(),
        );
        assert_eq!(
            view.read_with(cx, |v, _| v.shell.snapshot().left_width),
            initial + 50.
        );
        assert!(!view.read_with(cx, |v, _| v.shell.resize_active()));
    }

    #[test]
    fn mcp_receives_the_selected_library_and_listener_preferences() {
        let preferences = Preferences {
            library_path: Some(PathBuf::from("/tmp/locus-library")),
            mcp_enabled: false,
            mcp_local_network: true,
            mcp_port: 4321,
            ..Preferences::default()
        };
        let configuration = mcp_configuration(&preferences);
        let controller = McpServerController::new(&configuration);
        assert_eq!(
            configuration.library_path.as_deref(),
            Some(Path::new("/tmp/locus-library"))
        );
        assert!(matches!(
            controller.state(),
            locus_mcp::McpServerState::Stopped
        ));
        assert!(configuration.local_network);
        assert_eq!(configuration.port, 4321);
    }

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
