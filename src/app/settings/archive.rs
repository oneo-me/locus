use gpui::{AnyElement, ObjectFit, StyledImage};
use gpui_component::{Disableable as _, Sizable as _, button::Button};

use super::*;
use crate::document_library::restore_archived_note;
use crate::ui::document_icon_image;

impl Settings {
    fn restore_archive(&mut self, note: ArchivedNote, cx: &mut Context<Self>) {
        if self.restoring_archive.is_some() {
            return;
        }
        let document_path = note.document_path.clone();
        self.restoring_archive = Some(document_path.clone());
        self.archive_action_error = None;
        cx.notify();

        cx.spawn(async move |this, cx| {
            let target = document_path.clone();
            let result = cx
                .background_spawn(async move { restore_archived_note(&target) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.restoring_archive.as_ref() != Some(&document_path) {
                    return;
                }
                this.restoring_archive = None;
                match result {
                    Ok(()) => {
                        if let Some(index) = this
                            .archived_notes
                            .iter()
                            .position(|note| note.document_path == document_path)
                        {
                            this.archived_notes.remove(index);
                        }
                        cx.emit(SettingsEvent::ArchiveRestored);
                    }
                    Err(error) => this.archive_action_error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn render_archive(&self, cx: &mut Context<Self>) -> gpui::Div {
        let theme = Theme::global(cx).clone();
        let language = self.preferences.language;
        if self.preferences.library_path.is_none() {
            return archive_message(
                language.text("settings.archive.library_required"),
                theme.muted_foreground,
            );
        }
        if let Some(error) = &self.archive_load_error {
            return archive_message(
                format!("{}：{error}", language.text("settings.archive.load_error")),
                theme.danger,
            );
        }
        if self.archived_notes.is_empty() {
            return archive_message(
                language.text("settings.archive.empty"),
                theme.muted_foreground,
            );
        }

        let mut list = div()
            .w_full()
            .self_stretch()
            .min_w_0()
            .flex()
            .flex_col()
            .items_stretch();
        for (index, note) in self.archived_notes.iter().cloned().enumerate() {
            if index > 0 {
                list = list.child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border));
            }
            list = list.child(self.render_archive_row(note, cx));
        }
        list.when_some(self.archive_action_error.as_ref(), |list, error| {
            list.child(
                div()
                    .mx(px(20.0))
                    .mb(px(13.0))
                    .rounded(px(8.0))
                    .border_1()
                    .border_color(theme.danger)
                    .px(px(12.0))
                    .py(px(9.0))
                    .text_sm()
                    .text_color(theme.danger)
                    .child(format!(
                        "{}：{error}",
                        language.text("settings.archive.restore_error")
                    )),
            )
        })
    }

    fn render_archive_row(&self, note: ArchivedNote, cx: &mut Context<Self>) -> gpui::Div {
        let theme = Theme::global(cx).clone();
        let language = self.preferences.language;
        let pending = self.restoring_archive.as_ref() == Some(&note.document_path);
        let disabled = self.restoring_archive.is_some();
        let button_label = if pending {
            language.text("settings.archive.restoring")
        } else {
            language.text("settings.archive.restore")
        };
        let action_note = note.clone();
        let button = Button::new(format!("restore-archive-{}", note.path))
            .small()
            .label(button_label)
            .loading(pending)
            .disabled(disabled)
            .on_click(
                cx.listener(move |this, _, _, cx| this.restore_archive(action_note.clone(), cx)),
            );

        div()
            .w_full()
            .self_stretch()
            .min_w_0()
            .min_h(px(68.0))
            .px(px(20.0))
            .py(px(13.0))
            .flex()
            .items_center()
            .gap(px(24.0))
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(archive_note_icon(&note, &theme))
                            .child(
                                div()
                                    .min_w_0()
                                    .truncate()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(note.title),
                            ),
                    )
                    .child(
                        div()
                            .mt(px(4.0))
                            .truncate()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(note.path),
                    ),
            )
            .child(button)
    }
}

fn archive_note_icon(note: &ArchivedNote, theme: &Theme) -> AnyElement {
    if let Some(image) = &note.icon_image {
        let fallback = note.icon.clone();
        let muted_foreground = theme.muted_foreground;
        return document_icon_image(image)
            .size(px(16.0))
            .flex_none()
            .rounded(px(3.0))
            .object_fit(ObjectFit::Cover)
            .with_fallback(move || {
                fallback
                    .clone()
                    .map(emoji_icon)
                    .unwrap_or_else(|| icon(Icon::File, 15.0, muted_foreground).into_any_element())
            })
            .into_any_element();
    }
    note.icon
        .clone()
        .map(emoji_icon)
        .unwrap_or_else(|| icon(Icon::File, 15.0, theme.muted_foreground).into_any_element())
}

fn emoji_icon(value: String) -> AnyElement {
    div()
        .size(px(16.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(13.0))
        .line_height(px(16.0))
        .child(value)
        .into_any_element()
}

fn archive_message(message: impl Into<gpui::SharedString>, color: gpui::Hsla) -> gpui::Div {
    div()
        .w_full()
        .self_stretch()
        .min_w_0()
        .min_h(px(68.0))
        .px(px(20.0))
        .py(px(13.0))
        .flex()
        .items_center()
        .text_sm()
        .text_color(color)
        .child(message.into())
}
