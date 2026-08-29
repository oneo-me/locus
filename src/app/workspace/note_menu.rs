use super::Locus;
use crate::document_library::{LibraryNode, archive_note, update_note_icon};
use crate::theme::Theme;
use crate::ui::{ActivationExt, Icon, icon, toolbar_icon_button};
use gpui::{
    Anchor, App, Context, KeyDownEvent, MouseButton, PathPromptOptions, PromptLevel, Role, Window,
    anchored, deferred, div, prelude::*, px,
};

impl Locus {
    pub(super) fn render_note_menu(
        &self,
        node: &LibraryNode,
        theme: Theme,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let language = self.language(cx);
        let open = self.note_menu_open;
        let pending = self.note_action_pending;
        let trigger = toolbar_icon_button(
            "note-menu",
            if pending {
                language.text("正在更新笔记", "Updating note")
            } else {
                language.text("笔记菜单", "Note menu")
            },
            &self.note_menu_focus,
            if pending {
                Icon::Loader
            } else {
                Icon::MoreHorizontal
            },
            theme,
        )
        .when(open, |button| button.bg(theme.control_selected))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_activation(cx, move |this, window, cx| {
            if this.note_action_pending {
                return;
            }
            this.note_menu_open = !open;
            if this.note_menu_open {
                window.focus(&this.note_menu_upload_focus, cx);
            } else {
                window.focus(&this.note_menu_focus, cx);
            }
            cx.notify();
        });

        let document_path = node
            .document_path
            .clone()
            .expect("note has a document path");
        let upload_document_path = document_path.clone();
        let upload = div()
            .id("note-menu-upload-icon")
            .role(Role::Button)
            .aria_label(language.text("上传图标", "Upload icon"))
            .track_focus(&self.note_menu_upload_focus)
            .tab_index(0)
            .tab_stop(true)
            .h(px(32.0))
            .px(px(9.0))
            .rounded(px(6.0))
            .flex()
            .items_center()
            .gap(px(9.0))
            .cursor_default()
            .text_sm()
            .text_color(theme.text_secondary)
            .focus_visible(|style| style.border_1().border_color(theme.focus))
            .hover(|style| style.bg(theme.control_hover))
            .active(|style| style.bg(theme.control_active))
            .child(icon(Icon::Upload, 14.0, theme.text_tertiary))
            .child(language.text("上传图标", "Upload icon"))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_activation(cx, move |this, window, cx| {
                this.choose_note_icon(upload_document_path.clone(), window, cx)
            });
        let archive = div()
            .id("note-menu-archive")
            .role(Role::Button)
            .aria_label(language.text("归档笔记", "Archive note"))
            .track_focus(&self.note_menu_archive_focus)
            .tab_index(0)
            .tab_stop(true)
            .h(px(32.0))
            .px(px(9.0))
            .rounded(px(6.0))
            .flex()
            .items_center()
            .gap(px(9.0))
            .cursor_default()
            .text_sm()
            .text_color(theme.text_secondary)
            .focus_visible(|style| style.border_1().border_color(theme.focus))
            .hover(|style| style.bg(theme.control_hover))
            .active(|style| style.bg(theme.control_active))
            .child(icon(Icon::Archive, 14.0, theme.text_tertiary))
            .child(language.text("归档笔记", "Archive note"))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_activation(cx, move |this, window, cx| {
                this.archive_note_from_menu(document_path.clone(), window, cx)
            });
        let menu = div()
            .id("note-menu-popup")
            .key_context("NoteMenu")
            .w(px(176.0))
            .p(px(4.0))
            .rounded(px(9.0))
            .border_1()
            .border_color(theme.border_strong)
            .bg(theme.raised)
            .shadow_lg()
            .occlude()
            .flex()
            .flex_col()
            .text_color(theme.text)
            .child(upload)
            .child(div().h(px(1.0)).my(px(4.0)).bg(theme.border))
            .child(archive)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    this.close_note_menu(window, cx);
                    cx.stop_propagation();
                }
            }))
            .on_mouse_down_out(cx.listener(|this, _, window, cx| this.close_note_menu(window, cx)));

        div()
            .relative()
            .size(px(26.0))
            .flex_none()
            .child(trigger)
            .when(open, |control| {
                control.child(
                    div().absolute().bottom_0().right_0().size_0().child(
                        deferred(
                            anchored()
                                .anchor(Anchor::TopRight)
                                .snap_to_window_with_margin(px(8.0))
                                .child(menu),
                        )
                        .with_priority(2),
                    ),
                )
            })
    }

    fn close_note_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.note_menu_open {
            self.note_menu_open = false;
            window.focus(&self.note_menu_focus, cx);
            cx.notify();
        }
    }

    fn archive_note_from_menu(
        &mut self,
        document_path: std::path::PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.note_action_pending {
            return;
        }
        self.note_menu_open = false;
        self.note_action_pending = true;
        window.focus(&self.note_menu_focus, cx);
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
                        language.text("无法归档笔记", "Could not archive note"),
                        &error,
                        language.text("好", "OK"),
                        window,
                        cx,
                    ),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn choose_note_icon(
        &mut self,
        document_path: std::path::PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.note_action_pending {
            return;
        }
        self.note_menu_open = false;
        window.focus(&self.note_menu_focus, cx);
        cx.notify();
        let language = self.language(cx);
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(language.text("选择图标图片", "Choose Icon Image").into()),
        });
        cx.spawn(async move |this, cx| {
            let selected = match receiver.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                Ok(Ok(None)) | Err(_) => return,
                Ok(Err(error)) => {
                    let _ = this.update_in(cx, |_, window, cx| {
                        show_action_error(
                            language.text("无法选择图标", "Could not choose icon"),
                            &error.to_string(),
                            language.text("好", "OK"),
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
                        language.text("无法更新图标", "Could not update icon"),
                        &error,
                        language.text("好", "OK"),
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
    title: &'static str,
    error: &str,
    confirmation: &'static str,
    window: &mut Window,
    cx: &mut App,
) {
    drop(window.prompt(
        PromptLevel::Critical,
        title,
        Some(error),
        &[confirmation],
        cx,
    ));
}
