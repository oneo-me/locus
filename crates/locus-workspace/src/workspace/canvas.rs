use super::*;
use gpui_kit::Focusable as _;
use gpui_kit::component::WindowExt as _;
use locus_canvas::{CanvasEvent, CanvasView};

pub(super) struct NoteCanvas {
    pub(super) path: String,
    pub(super) view: Entity<CanvasView>,
    _subscription: Subscription,
}

impl Locus {
    pub(super) fn download_note_canvas(
        &mut self,
        path: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.canvas_download_pending {
            return;
        }
        let Some(canvas) = self
            .note_canvas
            .as_ref()
            .filter(|canvas| canvas.path == path)
        else {
            return;
        };
        let Some(node) = self.library_tree.node(path) else {
            return;
        };
        let title = node.title.clone();
        let Some(directory) = self
            .settings
            .read(cx)
            .preferences()
            .resolved_export_directory()
        else {
            window.push_notification(
                gpui_kit::component::notification::Notification::error(
                    t!(
                        "workspace.canvas.directory_required",
                        locale = self.language(cx).locale()
                    )
                    .into_owned(),
                ),
                cx,
            );
            return;
        };
        let task = canvas.view.read(cx).prepare_download(cx);
        self.canvas_download_pending = true;
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let result = async {
                let download = task.await?;
                cx.background_spawn(async move { download.save(&directory, &title) })
                    .await
            }
            .await;
            let _ = this.update_in(cx, |this, window, cx| {
                use gpui_kit::component::notification::Notification;
                this.canvas_download_pending = false;
                let language = this.language(cx);
                match result {
                    Ok(path) => {
                        window.push_notification(
                            Notification::success(format!(
                                "{}\n{}",
                                t!("workspace.canvas.downloaded", locale = language.locale()),
                                path.display()
                            )),
                            cx,
                        );
                    }
                    Err(error) => {
                        window.push_notification(
                            Notification::error(format!(
                                "{}\n{error}",
                                t!(
                                    "workspace.canvas.download_error",
                                    locale = language.locale()
                                )
                            )),
                            cx,
                        );
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn reload_focused_canvas(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.app_shell.is_settings_open() {
            return false;
        }
        if self
            .note_canvas
            .as_ref()
            .is_some_and(|canvas| canvas.view.read(cx).focus_handle(cx).is_focused(window))
        {
            self.reload_note_canvas(window, cx);
            return true;
        }
        false
    }

    pub(super) fn reload_note_canvas(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self
            .note_canvas
            .as_ref()
            .is_some_and(|canvas| canvas.view.read(cx).has_error())
        {
            self.release_note_canvas(cx);
            self.apply_note_canvas_selection(window, cx);
        } else if let Some(canvas) = &self.note_canvas {
            canvas.view.update(cx, |view, cx| view.reload(cx));
        }
    }

    pub(super) fn release_note_canvas(&mut self, cx: &mut Context<Self>) {
        let language = self.language(cx);
        if let Some(canvas) = self.note_canvas.take() {
            canvas
                .view
                .update(cx, |view, cx| view.synchronize(false, language, cx));
        }
    }

    pub(super) fn schedule_note_canvas_selection(&self, cx: &mut Context<Self>) {
        let view = cx.weak_entity();
        let window = self.window_handle;
        let selected = self.selected_library_node.clone();
        let library = self.current_library_path(cx).map(Path::to_path_buf);
        cx.defer(move |cx| {
            let _ = window.update(cx, |_, window, cx| {
                let _ = view.update(cx, |this, cx| {
                    if this.selected_library_node == selected
                        && this.current_library_path(cx) == library.as_deref()
                    {
                        this.apply_note_canvas_selection(window, cx);
                    }
                });
            });
        });
    }

    pub(super) fn apply_note_canvas_selection(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let node = self
            .selected_library_node
            .as_deref()
            .and_then(|path| self.library_tree.node(path))
            .filter(|node| node.document_path.is_some() && node.canvas_path.is_some())
            .cloned();
        let owner = node.as_ref().map(|node| node.path.as_str());
        if self.note_canvas.as_ref().map(|canvas| canvas.path.as_str()) == owner {
            return;
        }
        if self
            .note_canvas
            .as_ref()
            .is_some_and(|canvas| canvas.view.read(cx).focus_handle(cx).is_focused(window))
        {
            window.focus(&self.root_focus, cx);
        }
        self.release_note_canvas(cx);
        if let Some(node) = node
            && let Some(root) = self.current_library_path(cx).map(Path::to_path_buf)
        {
            let language = self.language(cx);
            let view = cx.new(|cx| CanvasView::new(&root, &node.directory, language, window, cx));
            let subscription = cx.subscribe_in(&view, window, |this, view, event, window, cx| {
                if this.app_shell.is_settings_open()
                    || !this
                        .note_canvas
                        .as_ref()
                        .is_some_and(|canvas| canvas.view == *view)
                {
                    return;
                }
                match event {
                    CanvasEvent::CloseWindow => window.dispatch_action(Box::new(CloseWindow), cx),
                    CanvasEvent::OpenTerminal => this.new_terminal_action(&NewTerminal, window, cx),
                    CanvasEvent::CommandPalette => {
                        this.open_command_palette_action(&crate::OpenCommandPalette, window, cx);
                    }
                    CanvasEvent::Settings => window.dispatch_action(Box::new(OpenSettings), cx),
                    CanvasEvent::OpenUrl(url) => cx.open_url(url),
                }
            });
            self.note_canvas = Some(NoteCanvas {
                path: node.path,
                view,
                _subscription: subscription,
            });
        }
        cx.notify();
    }

    pub(super) fn synchronize_canvases(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let visible = !self.app_shell.is_settings_open()
            && self.command_palette_state == super::command_palette::CommandPaletteState::Hidden
            && !window.has_active_dialog(cx)
            && !window.has_active_sheet(cx);
        let language = self.language(cx);
        if let Some(canvas) = &self.note_canvas {
            let visible = visible && self.selected_library_node.as_deref() == Some(&canvas.path);
            canvas
                .view
                .update(cx, |view, cx| view.synchronize(visible, language, cx));
        }
    }
}

pub(super) fn canvas_page(
    titlebar: AnyElement,
    metadata: Option<AnyElement>,
    canvas: Option<AnyElement>,
    theme: &Theme,
) -> Div {
    div()
        .relative()
        .size_full()
        .min_h_0()
        .min_w_0()
        .flex()
        .flex_col()
        .overflow_hidden()
        .child(div().h(px(super::TITLEBAR_HEIGHT)).flex_none())
        .when_some(metadata, |page, metadata| {
            page.child(
                div()
                    .id("canvas-metadata")
                    .debug_selector(|| "canvas-metadata".to_owned())
                    .w_full()
                    .max_h_64()
                    .flex_none()
                    .overflow_y_scroll()
                    .bg(theme.background)
                    .border_b_1()
                    .border_color(theme.border)
                    .child(metadata),
            )
        })
        .child(
            div()
                .min_h_0()
                .min_w_0()
                .flex_1()
                .overflow_hidden()
                .pl(px(1.0))
                .debug_selector(|| "canvas-region".to_owned())
                .children(canvas),
        )
        .child(titlebar)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::TestAppContext;

    struct NotePagePreview {
        height: f32,
        canvas: bool,
        metadata_height: Option<f32>,
    }

    impl Render for NotePagePreview {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div().w(px(700.0)).h(px(self.height)).child(canvas_page(
                div()
                    .absolute()
                    .top_0()
                    .h(px(super::super::TITLEBAR_HEIGHT))
                    .into_any_element(),
                self.metadata_height
                    .map(|height| div().h(px(height)).into_any_element()),
                self.canvas.then(|| {
                    div()
                        .size_full()
                        .debug_selector(|| "canvas-surface".to_owned())
                        .into_any_element()
                }),
                Theme::global(cx),
            ))
        }
    }

    #[gpui_kit::test]
    fn canvas_fills_page_below_titlebar_even_while_loading(cx: &mut TestAppContext) {
        cx.update(gpui_kit::component::init);
        let (view, cx) = cx.add_window_view(|_, _| NotePagePreview {
            height: 900.0,
            canvas: true,
            metadata_height: None,
        });
        for height in [900.0, 820.0, 1100.0] {
            view.update(cx, |view, cx| {
                view.height = height;
                cx.notify();
            });
            cx.update(|window, cx| window.draw(cx).clear(cx));
            let canvas = cx.debug_bounds("canvas-region").unwrap();
            assert!(
                (f32::from(canvas.size.height) - (height - super::super::TITLEBAR_HEIGHT)).abs()
                    <= 1.0
            );
            assert_eq!(canvas.top(), px(super::super::TITLEBAR_HEIGHT));
            assert_eq!(f32::from(canvas.bottom()), height);
            let surface = cx.debug_bounds("canvas-surface").unwrap();
            assert_eq!(surface.left() - canvas.left(), px(1.0));
            assert_eq!(surface.right(), canvas.right());
        }
        for metadata_height in [Some(80.0), Some(180.0), Some(900.0), None] {
            view.update(cx, |view, cx| {
                view.metadata_height = metadata_height;
                cx.notify();
            });
            cx.update(|window, cx| window.draw(cx).clear(cx));
            let canvas = cx.debug_bounds("canvas-region").unwrap();
            if metadata_height.is_some() {
                let metadata = cx.debug_bounds("canvas-metadata").unwrap();
                assert_eq!(metadata.top(), px(super::super::TITLEBAR_HEIGHT));
                assert_eq!(metadata.bottom(), canvas.top());
                assert!(metadata.size.height <= px(256.0));
                assert!(metadata.size.height > px(0.0));
            } else {
                assert!(cx.debug_bounds("canvas-metadata").is_none());
                assert_eq!(canvas.top(), px(super::super::TITLEBAR_HEIGHT));
            }
            assert!(canvas.size.height > px(0.0));
            assert_eq!(f32::from(canvas.bottom()), 1100.0);
        }
        view.update(cx, |view, cx| {
            view.canvas = false;
            cx.notify();
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(
            f32::from(cx.debug_bounds("canvas-region").unwrap().size.height),
            1100.0 - super::super::TITLEBAR_HEIGHT
        );
        assert!(cx.debug_bounds("canvas-surface").is_none());
    }
}
