use super::Locus;
use crate::document_library::{archive_note, update_note_icon};
use crate::ui::{Icon, toolbar_icon_button};
use gpui::{Anchor, App, Context, PathPromptOptions, PromptLevel, Window, prelude::*};
use gpui_component::{
    Disableable as _,
    menu::{DropdownMenu as _, PopupMenuItem},
};

impl Locus {
    pub(super) fn render_note_menu(
        &self,
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
        toolbar_icon_button(
            format!("note-menu-{}", language.locale()),
            trigger_label.clone(),
            Icon::Ellipsis,
            theme,
        )
        .loading(pending)
        .disabled(pending)
        .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, _| {
            menu.item(
                PopupMenuItem::new(upload_label.clone())
                    .icon(gpui_component::Icon::new(Icon::ArrowUp))
                    .on_click(move |_, window, cx| {
                        window.dispatch_action(Box::new(crate::ChooseCurrentNoteIcon), cx);
                    }),
            )
            .separator()
            .item(
                PopupMenuItem::new(archive_label.clone())
                    .icon(gpui_component::Icon::new(Icon::Inbox))
                    .on_click(move |_, window, cx| {
                        window.dispatch_action(Box::new(crate::ArchiveCurrentNote), cx);
                    }),
            )
        })
    }

    pub(super) fn archive_note_at_path(
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
                    Err(error) => {
                        eprintln!("{error}");
                        show_action_error(
                            language.text("workspace.note_menu.archive_error"),
                            language.text("common.file_action_error_detail"),
                            language.text("common.ok"),
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
                    eprintln!("{error}");
                    let _ = this.update_in(cx, |_, window, cx| {
                        show_action_error(
                            language.text("workspace.note_menu.choose_icon_error"),
                            language.text("common.file_action_error_detail"),
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
                    Err(error) => {
                        eprintln!("{error}");
                        show_action_error(
                            language.text("workspace.note_menu.update_icon_error"),
                            language.text("common.file_action_error_detail"),
                            language.text("common.ok"),
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
