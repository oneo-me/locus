use rust_i18n::t;
use std::time::Duration;

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, FocusTrapElement as _, WindowExt as _,
    command::{Command, CommandGroup, CommandItem},
    h_flex,
};
use gpui_kit::{
    Animation, AnimationExt as _, AnyElement, ClipboardItem, Context, Focusable as _,
    InteractiveElement as _, IntoElement as _, MouseButton, MouseDownEvent, ObjectFit,
    ParentElement as _, Role, SharedString, StatefulInteractiveElement as _, Styled as _,
    StyledImage as _, Window, div, ease_in_out, px,
};

use super::Locus;
use crate::{
    ArchiveCurrentNote, ChooseCurrentNoteIcon, CopyCurrentNoteReference, EditCurrentNote,
    NewTerminal, OpenCommandPalette, OpenCurrentNoteDirectory, OpenSettings, ReloadCurrentNote,
    ToggleFullScreen, ToggleLeftPanel, ToggleRightPanel,
};
use locus_library::{DocumentIcon, LIBRARY_REFERENCE_SCHEME, LibraryNode};
use locus_ui::{Icon, document_icon_image};

const COMMAND_PALETTE_TRANSITION: Duration = Duration::from_millis(240);
const COMMAND_PALETTE_WIDTH: f32 = 448.0;
const COMMAND_PALETTE_VERTICAL_POSITION: f32 = 0.16;
const COMMAND_PALETTE_MOTION: f32 = 8.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CommandPaletteState {
    Hidden,
    Visible,
    Hiding,
}

#[derive(Clone, Debug, Eq, PartialEq, gpui_kit::Action)]
#[action(namespace = locus, no_json, no_register)]
pub(super) struct OpenNote {
    path: String,
}

impl Locus {
    pub(super) fn open_command_palette_action(
        &mut self,
        _: &OpenCommandPalette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.command_palette_state == CommandPaletteState::Visible {
            self.begin_closing_command_palette(window, cx);
            return;
        }
        if self.command_palette_state == CommandPaletteState::Hiding {
            return;
        }
        if window.has_active_dialog(cx) {
            return;
        }

        self.command_palette_close_generation =
            self.command_palette_close_generation.wrapping_add(1);
        self.command_palette_state = CommandPaletteState::Visible;
        self.command_palette_previous_focus = window.focused(cx).map(|focus| focus.downgrade());
        self.command_palette.update(cx, |palette, cx| {
            palette.set_query("", window, cx);
        });
        let focus = self.command_palette.read(cx).focus_handle(cx);
        window.defer(cx, move |window, cx| {
            focus.focus(window, cx);
        });
        cx.notify();
    }

    fn build_command_palette(
        &self,
        palette: &gpui_kit::Entity<gpui_kit::component::command::CommandState>,
        locus: gpui_kit::WeakEntity<Self>,
        cx: &gpui_kit::App,
    ) -> Command {
        let language = self.settings.read(cx).preferences().appearance.language;
        let locus_for_cancel = locus.clone();
        let application = CommandGroup::new()
            .label(
                t!(
                    "workspace.command_palette.application",
                    locale = language.locale()
                )
                .into_owned(),
            )
            .item(
                CommandItem::new()
                    .label(t!("menu.application.settings", locale = language.locale()).into_owned())
                    .icon(Icon::Settings)
                    .keywords(["settings", "preferences"])
                    .action(Box::new(OpenSettings)),
            )
            .item(
                CommandItem::new()
                    .label(t!("workspace.terminal.new", locale = language.locale()).into_owned())
                    .icon(Icon::SquareTerminal)
                    .keywords(["terminal", "shell"])
                    .action(Box::new(NewTerminal))
                    .disabled(self.settings.read(cx).preferences().library_path.is_none()),
            )
            .item(
                CommandItem::new()
                    .label(
                        t!("menu.view.toggle_left_sidebar", locale = language.locale())
                            .into_owned(),
                    )
                    .icon(Icon::PanelLeft)
                    .keywords(["left sidebar", "navigation"])
                    .action(Box::new(ToggleLeftPanel)),
            )
            .item(
                CommandItem::new()
                    .label(
                        t!("menu.view.toggle_right_sidebar", locale = language.locale())
                            .into_owned(),
                    )
                    .icon(Icon::PanelRight)
                    .keywords(["right sidebar", "terminal panel"])
                    .action(Box::new(ToggleRightPanel)),
            )
            .item(
                CommandItem::new()
                    .label(
                        t!("menu.view.enter_full_screen", locale = language.locale()).into_owned(),
                    )
                    .icon(Icon::Maximize)
                    .keywords(["full screen", "fullscreen"])
                    .action(Box::new(ToggleFullScreen)),
            );

        let mut command = Command::new(palette).group(application);
        if self.current_note_node().is_some() {
            let current_note = CommandGroup::new()
                .label(
                    t!(
                        "workspace.command_palette.current_note",
                        locale = language.locale()
                    )
                    .into_owned(),
                )
                .item(
                    CommandItem::new()
                        .label(
                            t!("workspace.note.copy_reference", locale = language.locale())
                                .into_owned(),
                        )
                        .icon(Icon::Copy)
                        .keywords(["copy", "reference", "link", "uri"])
                        .action(Box::new(CopyCurrentNoteReference)),
                )
                .item(
                    CommandItem::new()
                        .label(t!("workspace.note.edit", locale = language.locale()).into_owned())
                        .icon(Icon::File)
                        .keywords(["edit", "editor", "vscode"])
                        .action(Box::new(EditCurrentNote)),
                )
                .item(
                    CommandItem::new()
                        .label(
                            t!("workspace.note.open_directory", locale = language.locale())
                                .into_owned(),
                        )
                        .icon(Icon::FolderOpen)
                        .keywords(["directory", "folder", "reveal"])
                        .action(Box::new(OpenCurrentNoteDirectory)),
                )
                .item(
                    CommandItem::new()
                        .label(
                            t!("menu.view.reload_current_note", locale = language.locale())
                                .into_owned(),
                        )
                        .icon(Icon::RotateCw)
                        .keywords(["reload", "refresh"])
                        .action(Box::new(ReloadCurrentNote)),
                )
                .item(
                    CommandItem::new()
                        .label(
                            t!(
                                "workspace.note_menu.upload_icon",
                                locale = language.locale()
                            )
                            .into_owned(),
                        )
                        .icon(Icon::ArrowUp)
                        .keywords(["upload", "icon", "image"])
                        .action(Box::new(ChooseCurrentNoteIcon))
                        .disabled(self.pending_note_action.is_some()),
                )
                .item(
                    CommandItem::new()
                        .label(
                            t!("workspace.note_menu.archive", locale = language.locale())
                                .into_owned(),
                        )
                        .icon(Icon::Inbox)
                        .keywords(["archive"])
                        .action(Box::new(ArchiveCurrentNote))
                        .disabled(self.pending_note_action.is_some()),
                );
            command = command.separator().group(current_note);
        }

        let note_items = visible_note_nodes(&self.library_tree.nodes)
            .into_iter()
            .map(note_command_item);
        command
            .separator()
            .group(
                CommandGroup::new()
                    .label(
                        t!(
                            "workspace.command_palette.notes",
                            locale = language.locale()
                        )
                        .into_owned(),
                    )
                    .items(note_items),
            )
            .placeholder(
                t!(
                    "workspace.command_palette.placeholder",
                    locale = language.locale()
                )
                .into_owned(),
            )
            .empty(move |_, _, cx| {
                gpui_kit::div()
                    .w_full()
                    .py_6()
                    .text_center()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(
                        t!(
                            "workspace.command_palette.no_matches",
                            locale = language.locale()
                        )
                        .into_owned(),
                    )
            })
            .bordered(true)
            .rounded(cx.theme().radius_tokens().xl)
            .pt_1()
            .on_cancel(move |window, cx| {
                let _ = locus_for_cancel.update(cx, |this, cx| {
                    this.begin_closing_command_palette(window, cx);
                });
            })
            .on_confirm(move |_, window, cx| {
                let _ = locus.update(cx, |this, cx| {
                    this.begin_closing_command_palette(window, cx);
                });
            })
    }

    fn begin_closing_command_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.command_palette_state != CommandPaletteState::Visible {
            return;
        }

        if cx.reduce_motion() {
            self.finish_closing_command_palette(window, cx);
            return;
        }

        self.command_palette_state = CommandPaletteState::Hiding;
        self.command_palette_close_generation =
            self.command_palette_close_generation.wrapping_add(1);
        let generation = self.command_palette_close_generation;
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            cx.background_executor()
                .timer(COMMAND_PALETTE_TRANSITION)
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                if this.command_palette_close_generation != generation {
                    return;
                }
                this.finish_closing_command_palette(window, cx);
            });
        })
        .detach();
    }

    fn finish_closing_command_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.command_palette_state = CommandPaletteState::Hidden;
        let palette_focus = self.command_palette.read(cx).focus_handle(cx);
        if palette_focus.is_focused(window) {
            if let Some(focus) = self
                .command_palette_previous_focus
                .take()
                .and_then(|focus| focus.upgrade())
            {
                focus.focus(window, cx);
            } else {
                self.root_focus.focus(window, cx);
            }
        } else {
            self.command_palette_previous_focus = None;
        }
        cx.notify();
    }

    pub(super) fn render_command_palette_overlay(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let hiding = match self.command_palette_state {
            CommandPaletteState::Hidden => return None,
            CommandPaletteState::Visible => false,
            CommandPaletteState::Hiding => true,
        };
        let mask_transition_id = if hiding {
            "command-palette-mask-hide"
        } else {
            "command-palette-mask-show"
        };
        let panel_transition_id = if hiding {
            "command-palette-panel-hide"
        } else {
            "command-palette-panel-show"
        };
        let animation = Animation::new(COMMAND_PALETTE_TRANSITION).with_easing(ease_in_out);
        let viewport = window.viewport_size();
        let panel_x = viewport.width / 2.0 - px(COMMAND_PALETTE_WIDTH / 2.0);
        let panel_y = viewport.height * COMMAND_PALETTE_VERTICAL_POSITION;
        let palette = self.command_palette.clone();
        let overlay_focus = self.command_palette_overlay_focus.clone();
        let command = self.build_command_palette(&palette, cx.weak_entity(), cx);
        let theme = cx.theme();

        let mask = div()
            .absolute()
            .inset_0()
            .bg(theme.overlay)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(Self::command_palette_mask_down),
            )
            .with_animation(mask_transition_id, animation.clone(), move |mask, delta| {
                mask.opacity(if hiding { 1.0 - delta } else { delta })
            });

        let panel = div()
            .absolute()
            .left(panel_x)
            .top(panel_y)
            .w(px(COMMAND_PALETTE_WIDTH))
            .rounded(theme.radius_tokens().xl)
            .shadow(theme.shadow_tokens().lg)
            .occlude()
            .child(command)
            .with_animation(panel_transition_id, animation, move |panel, delta| {
                let opacity = if hiding { 1.0 - delta } else { delta };
                let offset = if hiding {
                    -COMMAND_PALETTE_MOTION * delta
                } else {
                    -COMMAND_PALETTE_MOTION * (1.0 - delta)
                };
                panel.opacity(opacity).top(panel_y + px(offset))
            });

        Some(
            div()
                .id("command-palette-overlay")
                .absolute()
                .inset_0()
                .role(Role::Dialog)
                .focus_trap("command-palette", &overlay_focus)
                .child(mask)
                .child(panel)
                .into_any_element(),
        )
    }

    fn command_palette_mask_down(
        &mut self,
        _: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        self.begin_closing_command_palette(window, cx);
    }

    fn current_note_node(&self) -> Option<&LibraryNode> {
        self.selected_library_node
            .as_deref()
            .and_then(|path| self.library_tree.node(path))
            .filter(|node| node.document_path.is_some())
    }

    pub(super) fn open_note_action(
        &mut self,
        action: &OpenNote,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_library_reference(&action.path, cx);
    }

    pub(super) fn copy_current_note_reference_action(
        &mut self,
        _: &CopyCurrentNoteReference,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(path) = self.current_note_node().map(|node| node.path.clone()) else {
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(format!(
            "{LIBRARY_REFERENCE_SCHEME}{path}"
        )));
        self.copied_library_node = Some(path);
        self.note_copy_generation = self.note_copy_generation.wrapping_add(1);
        let generation = self.note_copy_generation;
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(super::COPY_FEEDBACK_DURATION)
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.note_copy_generation == generation {
                    this.copied_library_node = None;
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }

    pub(super) fn edit_current_note_action(
        &mut self,
        _: &EditCurrentNote,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(document_path) = self
            .current_note_node()
            .and_then(|node| node.document_path.clone())
        else {
            return;
        };
        let preferences = self.settings.read(cx).preferences();
        if let Err(error) = locus_ui::platform::open_in_editor(&document_path, preferences.editor) {
            eprintln!("{error}");
            self.show_file_action_error("workspace.note.edit_error", window, cx);
        }
    }

    pub(super) fn open_current_note_directory_action(
        &mut self,
        _: &OpenCurrentNoteDirectory,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(directory) = self.current_note_node().map(|node| node.directory.clone()) else {
            return;
        };
        if let Err(error) = locus_ui::platform::open_directory(&directory) {
            eprintln!("{error}");
            self.show_file_action_error("workspace.note.open_directory_error", window, cx);
        }
    }

    pub(super) fn choose_current_note_icon_action(
        &mut self,
        _: &ChooseCurrentNoteIcon,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(document_path) = self
            .current_note_node()
            .and_then(|node| node.document_path.clone())
        {
            self.choose_note_icon(document_path, cx);
        }
    }

    pub(super) fn archive_current_note_action(
        &mut self,
        _: &ArchiveCurrentNote,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(document_path) = self
            .current_note_node()
            .and_then(|node| node.document_path.clone())
        {
            self.archive_note_at_path(document_path, cx);
        }
    }

    fn show_file_action_error(&self, title_key: &str, window: &mut Window, cx: &mut Context<Self>) {
        let language = self.settings.read(cx).preferences().appearance.language;
        let title = t!(title_key, locale = language.locale()).into_owned();
        let detail = t!(
            "common.file_action_error_detail",
            locale = language.locale()
        )
        .into_owned();
        let confirmation = t!("common.ok", locale = language.locale()).into_owned();
        drop(window.prompt(
            gpui_kit::PromptLevel::Critical,
            &title,
            Some(&detail),
            &[confirmation.as_str()],
            cx,
        ));
    }
}

fn note_command_item(node: &LibraryNode) -> CommandItem {
    let label: SharedString = format!("{} · {}", node.title, node.path).into();
    let title: SharedString = node.title.clone().into();
    let path: SharedString = node.path.clone().into();
    let directory: SharedString = node.directory.to_string_lossy().into_owned().into();
    let icon = node.icon.clone();
    let icon_image = node.icon_image.clone();
    let row_label = label.clone();
    CommandItem::new()
        .label(label)
        .keywords([title, path, directory])
        .child(move |_, cx| {
            h_flex()
                .flex_1()
                .min_w_0()
                .gap_2()
                .child(command_note_icon(icon_image.as_ref(), icon.as_deref(), cx))
                .child(div().min_w_0().truncate().child(row_label.clone()))
        })
        .action(Box::new(OpenNote {
            path: node.path.clone(),
        }))
}

fn command_note_icon(
    image: Option<&DocumentIcon>,
    emoji: Option<&str>,
    cx: &gpui_kit::App,
) -> AnyElement {
    if let Some(image) = image {
        let fallback = emoji.map(str::to_owned);
        let muted_foreground = cx.theme().muted_foreground;
        return document_icon_image(image)
            .size(px(16.))
            .flex_none()
            .rounded(px(4.))
            .object_fit(ObjectFit::Cover)
            .with_fallback(move || {
                fallback.clone().map_or_else(
                    || {
                        gpui_kit::component::Icon::new(Icon::FileText)
                            .size(px(16.))
                            .text_color(muted_foreground)
                            .into_any_element()
                    },
                    command_note_emoji,
                )
            })
            .into_any_element();
    }

    emoji.map_or_else(
        || {
            gpui_kit::component::Icon::new(Icon::FileText)
                .size(px(16.))
                .text_color(cx.theme().muted_foreground)
                .into_any_element()
        },
        command_note_emoji,
    )
}

fn command_note_emoji(emoji: impl Into<SharedString>) -> AnyElement {
    div()
        .size(px(16.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(14.))
        .line_height(px(16.))
        .child(emoji.into())
        .into_any_element()
}

fn visible_note_nodes(nodes: &[LibraryNode]) -> Vec<&LibraryNode> {
    fn collect<'a>(nodes: &'a [LibraryNode], notes: &mut Vec<&'a LibraryNode>) {
        for node in nodes {
            if node.document_path.is_some() {
                notes.push(node);
            }
            collect(&node.children, notes);
        }
    }

    let mut notes = Vec::new();
    collect(nodes, &mut notes);
    notes
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn node(path: &str, document: bool, children: Vec<LibraryNode>) -> LibraryNode {
        LibraryNode {
            path: path.to_owned(),
            directory: PathBuf::from(path),
            document_path: document.then(|| PathBuf::from(path).join("index.md")),
            title: path.to_owned(),
            icon: None,
            icon_image: None,
            summary: None,
            additional_metadata: Vec::new(),
            attachments: Vec::new(),
            change: None,
            descendant_change: None,
            children,
        }
    }

    #[test]
    fn note_search_includes_nested_notes_but_not_organizing_directories() {
        let nodes = vec![node(
            "group",
            false,
            vec![node("group/one", true, Vec::new())],
        )];

        assert_eq!(
            visible_note_nodes(&nodes)
                .into_iter()
                .map(|node| node.path.as_str())
                .collect::<Vec<_>>(),
            ["group/one"]
        );
    }
}
