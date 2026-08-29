use std::path::PathBuf;
use std::time::Duration;

use gpui::{
    Animation, AnimationExt, AnyElement, FocusHandle, KeyDownEvent, MouseButton, ObjectFit,
    SharedString, StyledImage, Transformation, Window, div, img, percentage, prelude::*, px,
};

use super::Locus;
use crate::theme::Theme;
use crate::ui::{Icon, icon};

pub(super) struct NoteImagePreview {
    path: PathBuf,
    label: SharedString,
    focus: FocusHandle,
    previous_focus: Option<FocusHandle>,
    generation: u64,
}

impl Locus {
    pub(super) fn open_note_image_preview(
        &mut self,
        path: PathBuf,
        label: String,
        window: &mut Window,
        cx: &mut gpui::Context<Self>,
    ) {
        self.note_image_preview_generation = self.note_image_preview_generation.wrapping_add(1);
        let generation = self.note_image_preview_generation;
        let focus = cx.focus_handle();
        self.note_image_preview = Some(NoteImagePreview {
            path,
            label: label.into(),
            focus: focus.clone(),
            previous_focus: window.focused(cx),
            generation,
        });
        let weak = cx.weak_entity();
        window.on_next_frame(move |window, cx| {
            let should_focus = weak
                .update(cx, |this, _| {
                    this.note_image_preview
                        .as_ref()
                        .is_some_and(|preview| preview.generation == generation)
                })
                .unwrap_or(false);
            if should_focus {
                window.focus(&focus, cx);
            }
        });
        cx.notify();
    }

    fn close_note_image_preview(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) {
        let Some(preview) = self.note_image_preview.take() else {
            return;
        };
        self.note_image_preview_generation = self.note_image_preview_generation.wrapping_add(1);
        if let Some(previous_focus) = preview.previous_focus {
            window.focus(&previous_focus, cx);
        } else {
            window.focus(&self.root_focus, cx);
        }
        cx.notify();
    }

    pub(super) fn render_note_image_preview(
        &self,
        cx: &mut gpui::Context<Self>,
    ) -> Option<AnyElement> {
        let preview = self.note_image_preview.as_ref()?;
        let theme = Theme::current(cx);
        let path = preview.path.clone();
        let label = preview.label.clone();
        let focus = preview.focus.clone();
        let generation = preview.generation;

        let loading = move || {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    icon(Icon::Loader, 22.0, gpui::white().opacity(0.75)).with_animation(
                        SharedString::from(format!("preview-loading-{generation}")),
                        Animation::new(Duration::from_millis(900))
                            .repeat_synced()
                            .with_max_fps(30.0),
                        |icon, phase| {
                            icon.with_transformation(Transformation::rotate(percentage(phase)))
                        },
                    ),
                )
                .into_any_element()
        };
        let fallback = || {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(icon(Icon::ImageOff, 24.0, gpui::white().opacity(0.75)))
                .into_any_element()
        };
        let image = img(path)
            .size_full()
            .object_fit(ObjectFit::Contain)
            .with_loading(loading)
            .with_fallback(fallback);
        let close = div()
            .id("close-note-image-preview")
            .tab_index(0)
            .absolute()
            .top(px(14.0))
            .right(px(14.0))
            .size(px(32.0))
            .rounded_full()
            .flex()
            .items_center()
            .justify_center()
            .cursor_default()
            .bg(gpui::black().opacity(0.48))
            .focus_visible(|style| style.border_1().border_color(theme.focus))
            .hover(|style| style.bg(gpui::black().opacity(0.66)))
            .child(icon(Icon::X, 14.0, gpui::white()))
            .on_click(cx.listener(|this, _, window, cx| {
                this.close_note_image_preview(window, cx);
                cx.stop_propagation();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if !event.keystroke.modifiers.modified()
                    && matches!(event.keystroke.key.as_str(), "enter" | "space")
                {
                    this.close_note_image_preview(window, cx);
                    cx.stop_propagation();
                }
            }));
        let content = div()
            .relative()
            .size_full()
            .min_h_0()
            .p(px(36.0))
            .flex()
            .flex_col()
            .items_center()
            .gap(px(12.0))
            .child(div().w_full().min_h_0().flex_1().child(image))
            .child(
                div()
                    .max_w(px(560.0))
                    .px(px(11.0))
                    .py(px(5.0))
                    .rounded_full()
                    .bg(gpui::black().opacity(0.48))
                    .text_size(px(12.5))
                    .text_color(gpui::white().opacity(0.9))
                    .truncate()
                    .child(label),
            )
            .child(close);
        let layer = div()
            .id(SharedString::from(format!(
                "note-image-preview-{generation}"
            )))
            .absolute()
            .inset_0()
            .occlude()
            .track_focus(&focus)
            .tab_index(0)
            .bg(gpui::black().opacity(0.82))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if !event.keystroke.modifiers.modified() && event.keystroke.key == "escape" {
                    this.close_note_image_preview(window, cx);
                    cx.stop_propagation();
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    this.close_note_image_preview(window, cx);
                }),
            )
            .child(content)
            .with_animation(
                SharedString::from(format!("note-image-preview-enter-{generation}")),
                Animation::new(Duration::from_millis(140)).with_easing(gpui::ease_out_quint()),
                |element, phase| element.opacity(phase),
            );
        Some(gpui::deferred(layer).with_priority(5).into_any_element())
    }
}
