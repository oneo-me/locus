use std::path::Path;

use gpui::{
    AnyElement, App, KeyDownEvent, MouseButton, PromptLevel, SharedString, Window, div, prelude::*,
    px,
};

use super::extension_syntax;
use super::thumbnail;
use crate::{Icon, icon};
use locus_library::Attachment;
use locus_preferences::localization::Language;
use locus_preferences::theme::Theme;

const GALLERY_ROW_HEIGHT: f32 = 180.0;

pub fn render_gallery(
    source: &str,
    attachments: &[Attachment],
    theme: Theme,
    language: Language,
) -> AnyElement {
    let items = match extension_syntax::gallery_items(source, attachments, language) {
        Ok(items) => items,
        Err(error) => {
            return extension_error(language.text("markdown.gallery.render_error"), error, theme);
        }
    };
    let children = items
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            let size = item
                .attachment
                .image_size
                .expect("validated gallery image size");
            let ratio = size.width as f32 / size.height as f32;
            let attachment = item.attachment.clone();
            div()
                .id(SharedString::from(format!(
                    "gallery-item-{index}-{}",
                    attachment.name
                )))
                .h(px(GALLERY_ROW_HEIGHT))
                .min_w(px(120.0))
                .max_w_full()
                .flex_basis(px((ratio * GALLERY_ROW_HEIGHT).clamp(120.0, 360.0)))
                .flex_grow(ratio.clamp(0.6, 2.0))
                .flex_shrink_1()
                .overflow_hidden()
                .rounded(px(10.0))
                .bg(theme.secondary)
                .child(thumbnail::gallery_thumbnail(&attachment, index, &theme))
                .into_any_element()
        })
        .collect::<Vec<_>>();

    div()
        .w_full()
        .min_w_0()
        .flex()
        .flex_wrap()
        .gap(px(8.0))
        .children(children)
        .child(div().h_0().flex_basis(px(0.0)).flex_grow(999_999.0))
        .into_any_element()
}

pub fn render_attachments(
    source: &str,
    attachments: &[Attachment],
    theme: Theme,
    language: Language,
) -> AnyElement {
    let items = match extension_syntax::attachment_items(source, attachments, language) {
        Ok(items) => items,
        Err(error) => {
            return extension_error(
                language.text("markdown.attachments.render_error"),
                error,
                theme,
            );
        }
    };
    let last = items.len().saturating_sub(1);
    let rows = items
        .into_iter()
        .enumerate()
        .map(|(index, attachment)| {
            let path = attachment.path.clone();
            let keyboard_path = path.clone();
            div()
                .id(SharedString::from(format!(
                    "attachment-{index}-{}",
                    attachment.name
                )))
                .tab_index(0)
                .w_full()
                .min_w_0()
                .px(px(12.0))
                .py(px(10.0))
                .flex()
                .items_center()
                .gap(px(9.0))
                .cursor_default()
                .when(index < last, |row| {
                    row.border_b_1().border_color(theme.border)
                })
                .focus_visible(|style| style.bg(theme.accent))
                .hover(|style| style.bg(theme.accent))
                .active(|style| style.bg(theme.accent))
                .child(icon(Icon::File, 15.0, theme.muted_foreground))
                .child(
                    attachment_name(attachment.name)
                        .text_size(px(14.0))
                        .line_height(px(20.0)),
                )
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(move |_, window, cx| {
                    open_attachment(&path, language, window, cx);
                })
                .on_key_down(move |event: &KeyDownEvent, window, cx| {
                    if !event.keystroke.modifiers.modified()
                        && matches!(event.keystroke.key.as_str(), "enter" | "space")
                    {
                        open_attachment(&keyboard_path, language, window, cx);
                        cx.stop_propagation();
                    }
                })
                .into_any_element()
        })
        .collect::<Vec<_>>();

    div()
        .w_full()
        .min_w_0()
        .overflow_hidden()
        .rounded(px(8.0))
        .border_1()
        .border_color(theme.input)
        .bg(theme.background)
        .flex()
        .flex_col()
        .children(rows)
        .into_any_element()
}

pub fn render_thumbnail_image(attachment: &Attachment, theme: Theme) -> AnyElement {
    div()
        .id(SharedString::from(format!(
            "inline-image-{}",
            attachment.name
        )))
        .max_w_full()
        .overflow_hidden()
        .rounded(px(10.0))
        .child(thumbnail::inline_thumbnail(attachment, &theme))
        .into_any_element()
}

fn attachment_name(name: String) -> gpui::Div {
    let (start, end) = split_filename(&name);
    if end.is_empty() {
        return div().min_w_0().flex_1().truncate().child(start);
    }
    div()
        .min_w_0()
        .flex_1()
        .flex()
        .child(div().min_w(px(28.0)).truncate().child(start))
        .child(div().flex_none().child(end))
}

fn split_filename(value: &str) -> (String, String) {
    let characters = value.chars().collect::<Vec<_>>();
    if characters.len() <= 20 {
        return (value.to_owned(), String::new());
    }
    let tail_length = 12.min(8.max((characters.len() as f32 * 0.35).floor() as usize));
    (
        characters[..characters.len() - tail_length]
            .iter()
            .collect(),
        characters[characters.len() - tail_length..]
            .iter()
            .collect(),
    )
}

fn open_attachment(path: &Path, language: Language, window: &mut Window, cx: &mut App) {
    if let Err(error) = crate::platform::open_path(path) {
        eprintln!("{error}");
        let title = language.text("markdown.attachments.open_error");
        let detail = language.text("common.file_action_error_detail");
        let confirmation = language.text("common.ok");
        drop(window.prompt(
            PromptLevel::Critical,
            &title,
            Some(&detail),
            &[confirmation.as_str()],
            cx,
        ));
    }
}

fn extension_error(title: String, message: String, theme: Theme) -> AnyElement {
    div()
        .w_full()
        .min_w_0()
        .rounded(px(8.0))
        .border_1()
        .border_color(theme.danger.opacity(0.35))
        .bg(theme.danger.opacity(0.08))
        .px(px(12.0))
        .py(px(10.0))
        .flex()
        .flex_col()
        .gap(px(3.0))
        .text_size(px(13.0))
        .line_height(px(19.0))
        .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child(title))
        .child(div().text_color(theme.muted_foreground).child(message))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_attachment_names_preserve_the_extension_and_tail() {
        assert_eq!(
            split_filename("a-very-long-report-name.pdf"),
            ("a-very-long-report".to_owned(), "-name.pdf".to_owned())
        );
        assert_eq!(
            split_filename("report.pdf"),
            ("report.pdf".to_owned(), String::new())
        );
    }
}
