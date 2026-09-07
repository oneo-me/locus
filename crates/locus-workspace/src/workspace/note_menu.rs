use super::Locus;
use gpui_kit::component::{
    Disableable as _,
    menu::{DropdownMenu as _, PopupMenu, PopupMenuItem},
};
use gpui_kit::{
    Anchor, AnyElement, App, Context, PathPromptOptions, PromptLevel, Window, prelude::*,
};
use locus_library::{LibraryNode, archive_note, update_note_icon};
use locus_ui::{Icon, toolbar_icon_button};
use rust_i18n::t;

impl Locus {
    pub(super) fn render_note_menu(
        &self,
        node: &LibraryNode,
        id: String,
        theme: &locus_preferences::theme::Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let document_path = node
            .document_path
            .clone()
            .expect("note menus require a document");
        let language = self.language(cx);
        let pending = self.pending_note_action.as_deref() == Some(document_path.as_path());
        let disabled = self.pending_note_action.is_some();
        let trigger_label = if pending {
            t!("workspace.note_menu.updating", locale = language.locale()).into_owned()
        } else {
            t!("workspace.note_menu.title", locale = language.locale()).into_owned()
        };
        let menu = self.note_popup_menu(node, cx);
        toolbar_icon_button(id, trigger_label.clone(), Icon::Ellipsis, theme)
            .loading(pending)
            .disabled(disabled)
            .dropdown_menu_with_anchor(Anchor::TopRight, menu)
            .into_any_element()
    }

    pub(super) fn note_popup_menu(
        &self,
        node: &LibraryNode,
        cx: &mut Context<Self>,
    ) -> impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + use<> {
        let document_path = node
            .document_path
            .clone()
            .expect("note menus require a document");
        let language = self.language(cx);
        let upload_label = t!(
            "workspace.note_menu.upload_icon",
            locale = language.locale()
        )
        .into_owned();
        let archive_label =
            t!("workspace.note_menu.archive", locale = language.locale()).into_owned();
        let disabled = self.pending_note_action.is_some();
        let locus = cx.weak_entity();
        move |menu, _, _| {
            let choose_locus = locus.clone();
            let choose_path = document_path.clone();
            let archive_locus = locus.clone();
            let archive_path = document_path.clone();
            menu.item(
                PopupMenuItem::new(upload_label.clone())
                    .icon(gpui_kit::component::Icon::new(Icon::ArrowUp))
                    .disabled(disabled)
                    .on_click(move |_, _, cx| {
                        let _ = choose_locus.update(cx, |this, cx| {
                            this.choose_note_icon(choose_path.clone(), cx);
                        });
                    }),
            )
            .separator()
            .item(
                PopupMenuItem::new(archive_label.clone())
                    .icon(gpui_kit::component::Icon::new(Icon::Inbox))
                    .disabled(disabled)
                    .on_click(move |_, _, cx| {
                        let _ = archive_locus.update(cx, |this, cx| {
                            this.archive_note_at_path(archive_path.clone(), cx);
                        });
                    }),
            )
        }
    }

    pub(super) fn archive_note_at_path(
        &mut self,
        document_path: std::path::PathBuf,
        cx: &mut Context<Self>,
    ) {
        if self.pending_note_action.is_some() {
            return;
        }
        self.pending_note_action = Some(document_path.clone());
        cx.notify();
        let language = self.language(cx);
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { archive_note(&document_path) })
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.pending_note_action = None;
                match result {
                    Ok(()) => this.reload_library_projection(cx),
                    Err(error) => {
                        eprintln!("{error}");
                        show_action_error(
                            t!(
                                "workspace.note_menu.archive_error",
                                locale = language.locale()
                            )
                            .into_owned(),
                            t!(
                                "common.file_action_error_detail",
                                locale = language.locale()
                            )
                            .into_owned(),
                            t!("common.ok", locale = language.locale()).into_owned(),
                            window,
                            cx,
                        )
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn choose_note_icon(
        &mut self,
        document_path: std::path::PathBuf,
        cx: &mut Context<Self>,
    ) {
        if self.pending_note_action.is_some() {
            return;
        }
        cx.notify();
        let language = self.language(cx);
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(
                t!(
                    "workspace.note_menu.choose_icon",
                    locale = language.locale()
                )
                .into_owned()
                .into(),
            ),
        });
        cx.spawn(async move |this, cx| {
            let selected = match receiver.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                Ok(Ok(None)) | Err(_) => return,
                Ok(Err(error)) => {
                    eprintln!("{error}");
                    let _ = this.update_in(cx, |_, window, cx| {
                        show_action_error(
                            t!(
                                "workspace.note_menu.choose_icon_error",
                                locale = language.locale()
                            )
                            .into_owned(),
                            t!(
                                "common.file_action_error_detail",
                                locale = language.locale()
                            )
                            .into_owned(),
                            t!("common.ok", locale = language.locale()).into_owned(),
                            window,
                            cx,
                        )
                    });
                    return;
                }
            };
            let Some(source_path) = selected else {
                return;
            };
            let started = this
                .update(cx, |this, cx| {
                    if this.pending_note_action.is_some() {
                        return false;
                    }
                    this.pending_note_action = Some(document_path.clone());
                    cx.notify();
                    true
                })
                .unwrap_or(false);
            if !started {
                return;
            }
            let result = cx
                .background_spawn(async move { update_note_icon(&document_path, &source_path) })
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.pending_note_action = None;
                match result {
                    Ok(()) => this.reload_library_projection(cx),
                    Err(error) => {
                        eprintln!("{error}");
                        show_action_error(
                            t!(
                                "workspace.note_menu.update_icon_error",
                                locale = language.locale()
                            )
                            .into_owned(),
                            t!(
                                "common.file_action_error_detail",
                                locale = language.locale()
                            )
                            .into_owned(),
                            t!("common.ok", locale = language.locale()).into_owned(),
                            window,
                            cx,
                        )
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
}

fn show_action_error(
    title: String,
    detail: String,
    confirmation: String,
    window: &mut Window,
    cx: &mut App,
) {
    drop(window.prompt(
        PromptLevel::Critical,
        &title,
        Some(&detail),
        &[confirmation.as_str()],
        cx,
    ));
}
