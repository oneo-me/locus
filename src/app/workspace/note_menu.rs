use super::Locus;
use crate::document_library::{LibraryNode, archive_note, update_note_icon};
use crate::ui::{Icon, toolbar_icon_button};
use gpui::{Anchor, App, Context, PathPromptOptions, PromptLevel, Window, prelude::*};
use gpui_component::{
    Disableable as _,
    menu::{DropdownMenu as _, PopupMenuItem},
};

impl Locus {
    pub(super) fn render_note_menu(
        &self,
        node: &LibraryNode,
        theme: &crate::theme::Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let language = self.language(cx);
        let pending = self.note_action_pending;
        let trigger_label = if pending {
            language.text("workspace.note_menu.updating")
        } else {
            language.text("workspace.note_menu.title")
        };
        let upload_label = language.text("workspace.note_menu.upload_icon");
        let archive_label = language.text("workspace.note_menu.archive");
        let document_path = node
            .document_path
            .clone()
            .expect("note has a document path");
        let upload_document_path = document_path.clone();
        let locus = cx.entity();
        toolbar_icon_button(
            format!("note-menu-{}", language.locale()),
            trigger_label.clone(),
            Icon::Ellipsis,
            theme,
        )
        .loading(pending)
        .disabled(pending)
        .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, _| {
            let upload_locus = locus.clone();
            let archive_locus = locus.clone();
            let upload_document_path = upload_document_path.clone();
            let document_path = document_path.clone();
            menu.item(
                PopupMenuItem::new(upload_label.clone())
                    .icon(gpui_component::Icon::new(Icon::ArrowUp))
                    .on_click(move |_, _, cx| {
                        upload_locus.update(cx, |this, cx| {
                            this.choose_note_icon(upload_document_path.clone(), cx)
                        });
                    }),
            )
            .separator()
            .item(
                PopupMenuItem::new(archive_label.clone())
                    .icon(gpui_component::Icon::new(Icon::Inbox))
                    .on_click(move |_, _, cx| {
                        archive_locus.update(cx, |this, cx| {
                            this.archive_note_from_menu(document_path.clone(), cx)
                        });
                    }),
            )
        })
    }

    fn archive_note_from_menu(
        &mut self,
        document_path: std::path::PathBuf,
        cx: &mut Context<Self>,
    ) {
        if self.note_action_pending {
            return;
        }
        self.note_action_pending = true;
        cx.notify();
        let language = self.language(cx);
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { archive_note(&document_path) })
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.note_action_pending = false;
                match result {
                    Ok(()) => this.reload_library_projection(cx),
                    Err(error) => show_action_error(
                        language.text("workspace.note_menu.archive_error"),
                        &error,
                        language.text("common.ok"),
                        window,
                        cx,
                    ),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn choose_note_icon(&mut self, document_path: std::path::PathBuf, cx: &mut Context<Self>) {
        if self.note_action_pending {
            return;
        }
        cx.notify();
        let language = self.language(cx);
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(language.text("workspace.note_menu.choose_icon").into()),
        });
        cx.spawn(async move |this, cx| {
            let selected = match receiver.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                Ok(Ok(None)) | Err(_) => return,
                Ok(Err(error)) => {
                    let _ = this.update_in(cx, |_, window, cx| {
                        show_action_error(
                            language.text("workspace.note_menu.choose_icon_error"),
                            &error.to_string(),
                            language.text("common.ok"),
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
            if this
                .update(cx, |this, cx| {
                    this.note_action_pending = true;
                    cx.notify();
                })
                .is_err()
            {
                return;
            }
            let result = cx
                .background_spawn(async move { update_note_icon(&document_path, &source_path) })
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.note_action_pending = false;
                match result {
                    Ok(()) => this.reload_library_projection(cx),
                    Err(error) => show_action_error(
                        language.text("workspace.note_menu.update_icon_error"),
                        &error,
                        language.text("common.ok"),
                        window,
                        cx,
                    ),
                }
                cx.notify();
            });
        })
        .detach();
    }
}

fn show_action_error(
    title: String,
    error: &str,
    confirmation: String,
    window: &mut Window,
    cx: &mut App,
) {
    drop(window.prompt(
        PromptLevel::Critical,
        &title,
        Some(error),
        &[confirmation.as_str()],
        cx,
    ));
}
