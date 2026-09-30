use std::time::{Duration, Instant};

use gpui_kit::component::{IconName, Theme};
use gpui_kit::{
    AnyElement, Div, Hsla, Stateful, Window, div, ease_out_quint, prelude::*, px, transparent_black,
};

use crate::toolbar_icon_button;

const PANEL_SLIDE_DURATION: Duration = Duration::from_millis(200);

#[derive(Clone, Copy, Debug)]
pub struct PanelSize {
    pub min: f32,
    pub max: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct WorkspaceShellConfig {
    pub content_min_width: f32,
    pub left: PanelSize,
}

#[derive(Clone, Copy, Debug)]
pub struct PanelFrame {
    pub left_content: f32,
    pub left: f32,
    pub left_sliding: bool,
}

impl PanelFrame {
    pub fn sliding(self) -> bool {
        self.left_sliding
    }
}

#[derive(Clone, Copy)]
struct PanelSlide {
    from: f32,
    started: Instant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppShellMode {
    Workspace,
    Settings,
}

#[derive(Clone, Copy, Debug)]
pub struct WorkspaceShellSnapshot {
    pub left_visible: bool,
    pub left_width: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct WorkspaceShellLayout {
    pub frame: PanelFrame,
    pub left_transition_finished: bool,
}

#[derive(Clone, Copy, Debug)]
struct PanelResizeDrag {
    start_mouse_x: f32,
    start_width: f32,
}

pub struct AppShellState {
    mode: AppShellMode,
    workspace: WorkspaceShellState,
}

pub struct WorkspaceShellState {
    config: WorkspaceShellConfig,
    left_visible: bool,
    left_width: f32,
    rendered_left_width: f32,
    left_slide: Option<PanelSlide>,
    resize_drag: Option<PanelResizeDrag>,
}

impl AppShellState {
    pub fn new(workspace: WorkspaceShellState) -> Self {
        Self {
            mode: AppShellMode::Workspace,
            workspace,
        }
    }

    pub fn mode(&self) -> AppShellMode {
        self.mode
    }

    pub fn is_settings_open(&self) -> bool {
        self.mode == AppShellMode::Settings
    }

    pub fn open_settings(&mut self) {
        self.mode = AppShellMode::Settings;
    }

    pub fn close_settings(&mut self) {
        self.mode = AppShellMode::Workspace;
    }

    pub fn workspace(&self) -> &WorkspaceShellState {
        &self.workspace
    }

    pub fn workspace_mut(&mut self) -> &mut WorkspaceShellState {
        &mut self.workspace
    }
}

impl Default for WorkspaceShellConfig {
    fn default() -> Self {
        Self {
            content_min_width: 360.0,
            left: PanelSize {
                min: crate::LEFT_SIDEBAR_MIN_WIDTH,
                max: 420.0,
            },
        }
    }
}

impl Default for WorkspaceShellState {
    fn default() -> Self {
        Self::new(
            WorkspaceShellConfig::default(),
            true,
            crate::DEFAULT_LEFT_SIDEBAR_WIDTH,
        )
    }
}

impl Default for AppShellState {
    fn default() -> Self {
        Self::new(WorkspaceShellState::default())
    }
}

impl WorkspaceShellState {
    pub fn new(config: WorkspaceShellConfig, left_visible: bool, left_width: f32) -> Self {
        let left_width = left_width.clamp(config.left.min, config.left.max);
        Self {
            config,
            left_visible,
            left_width,
            rendered_left_width: if left_visible { left_width } else { 0.0 },
            left_slide: None,
            resize_drag: None,
        }
    }

    pub fn snapshot(&self) -> WorkspaceShellSnapshot {
        WorkspaceShellSnapshot {
            left_visible: self.left_visible,
            left_width: self.left_width,
        }
    }

    pub fn visible(&self) -> bool {
        self.left_visible
    }

    pub fn rendered_width(&self) -> f32 {
        self.rendered_left_width
    }

    pub fn transitioning(&self) -> bool {
        self.left_slide.is_some()
    }

    pub fn resize_active(&self) -> bool {
        self.resize_drag.is_some()
    }

    pub fn panel_content_width(&self, viewport_width: f32) -> f32 {
        if !self.left_visible && self.left_slide.is_none() {
            return 0.0;
        }
        self.left_width
            .min((viewport_width - self.config.content_min_width).max(0.0))
    }

    pub fn frame(&self, window: &Window) -> PanelFrame {
        let left_content = self.panel_content_width(f32::from(window.viewport_size().width));
        PanelFrame {
            left_content,
            left: sampled_slide_width(
                self.left_slide,
                if self.left_visible { left_content } else { 0.0 },
            ),
            left_sliding: self.left_slide.is_some(),
        }
    }

    pub fn toggle(&mut self, reduce_motion: bool) {
        self.left_visible = !self.left_visible;
        self.left_slide = (!reduce_motion).then(|| PanelSlide::new(self.rendered_left_width));
    }

    pub fn layout(&mut self, window: &mut Window) -> WorkspaceShellLayout {
        let was_transitioning = self.left_slide.is_some();
        let left_content = self.panel_content_width(f32::from(window.viewport_size().width));
        let left = slide_width(
            &mut self.left_slide,
            if self.left_visible { left_content } else { 0.0 },
        );
        self.rendered_left_width = left;
        let frame = PanelFrame {
            left_content,
            left,
            left_sliding: self.left_slide.is_some(),
        };
        if frame.sliding() {
            window.request_animation_frame();
        }
        WorkspaceShellLayout {
            frame,
            left_transition_finished: was_transitioning && self.left_slide.is_none(),
        }
    }

    pub fn begin_resize(&mut self, pointer_x: f32) {
        self.left_slide = None;
        self.resize_drag = Some(PanelResizeDrag {
            start_mouse_x: pointer_x,
            start_width: self.rendered_left_width,
        });
    }

    pub fn resize(&mut self, pointer_x: f32, viewport_width: f32) -> bool {
        let Some(drag) = self.resize_drag else {
            return false;
        };
        let maximum = self
            .config
            .left
            .max
            .min(viewport_width - self.config.content_min_width)
            .max(self.config.left.min);
        self.left_width = (drag.start_width + pointer_x - drag.start_mouse_x)
            .clamp(self.config.left.min, maximum);
        true
    }

    pub fn finish_resize(&mut self) -> bool {
        self.resize_drag.take().is_some()
    }

    pub fn reset_width(&mut self, width: f32) {
        self.resize_drag = None;
        self.left_width = width.clamp(self.config.left.min, self.config.left.max);
        self.left_slide = None;
    }
}

impl PanelSlide {
    fn new(from: f32) -> Self {
        Self {
            from,
            started: Instant::now(),
        }
    }

    fn width_toward(self, target: f32) -> Option<f32> {
        width_at(self.from, target, self.started.elapsed())
    }
}

fn slide_width(slide: &mut Option<PanelSlide>, target: f32) -> f32 {
    match slide.and_then(|slide| slide.width_toward(target)) {
        Some(width) => width,
        None => {
            *slide = None;
            target
        }
    }
}

fn sampled_slide_width(slide: Option<PanelSlide>, target: f32) -> f32 {
    slide
        .and_then(|slide| slide.width_toward(target))
        .unwrap_or(target)
}

pub fn panel_toggle_button(
    id: &'static str,
    label: impl Into<gpui_kit::SharedString>,
    theme: &Theme,
) -> gpui_kit::component::button::Button {
    toolbar_icon_button(id, label, IconName::PanelLeft, theme)
}

pub fn panel_resize_handle(id: &'static str, active: bool, theme: &Theme) -> Stateful<Div> {
    let handle = div().right(px(-5.0)).w(px(10.0));
    let edge = div()
        .left(px(5.0))
        .w(px(1.0))
        .absolute()
        .top_0()
        .h_full()
        .bg(if active {
            theme.drag_border
        } else {
            transparent_black()
        });
    handle
        .id(id)
        .absolute()
        .top_0()
        .bottom_0()
        .cursor_col_resize()
        .child(edge)
}

pub struct WorkspaceShell {
    frame: PanelFrame,
    left: AnyElement,
    content: AnyElement,
    left_toggle: AnyElement,
    left_toggle_offset: f32,
    control_inset: f32,
    sidebar_divider_color: Option<Hsla>,
}

impl WorkspaceShell {
    pub fn new(
        frame: PanelFrame,
        left: AnyElement,
        content: AnyElement,
        window: &Window,
        cx: &gpui_kit::App,
    ) -> Self {
        let theme = gpui_kit::component::Theme::global(cx);
        let left_toggle = panel_toggle_button(
            "toggle-left-sidebar",
            rust_i18n::t!("shell.toggle_left", locale = &gpui_kit::component::locale())
                .into_owned(),
            theme,
        )
        .on_click(|_, window, cx| window.dispatch_action(Box::new(crate::ToggleLeftPanel), cx))
        .into_any_element();
        Self {
            frame,
            left,
            content,
            left_toggle,
            left_toggle_offset: crate::titlebar_left_control_offset(window.is_fullscreen()),
            control_inset: crate::TITLEBAR_CONTROL_INSET,
            sidebar_divider_color: Some(theme.sidebar_border),
        }
    }

    pub fn left_toggle_offset(mut self, offset: f32) -> Self {
        self.left_toggle_offset = offset;
        self
    }

    pub fn control_inset(mut self, inset: f32) -> Self {
        self.control_inset = inset;
        self
    }

    pub fn sidebar_divider_color(mut self, color: Hsla) -> Self {
        self.sidebar_divider_color = Some(color);
        self
    }

    pub fn into_element(self) -> Div {
        let frame = self.frame;
        let sidebar_divider_color = self.sidebar_divider_color;
        div()
            .size_full()
            .relative()
            .flex()
            .when(frame.left > 0.0, |root| {
                root.child(
                    div()
                        .h_full()
                        .flex_none()
                        .w(px(frame.left))
                        .when(frame.left_sliding, |panel| panel.overflow_hidden())
                        .child(
                            div()
                                .w(px(frame.left_content))
                                .h_full()
                                .flex_none()
                                .child(self.left),
                        ),
                )
            })
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_w(px(0.0))
                    .h_full()
                    .child(self.content)
                    .when(sidebar_divider_visible(frame.left), |content| {
                        content.when_some(sidebar_divider_color, |content, color| {
                            content.child(
                                div()
                                    .absolute()
                                    .top_0()
                                    .left_0()
                                    .bottom_0()
                                    .w(px(1.0))
                                    .bg(color),
                            )
                        })
                    }),
            )
            .child(
                div()
                    .absolute()
                    .debug_selector(|| "workspace-left-toggle".to_owned())
                    .top(px(self.control_inset))
                    .left(px(self.left_toggle_offset))
                    .child(self.left_toggle),
            )
    }
}

fn sidebar_divider_visible(panel_width: f32) -> bool {
    panel_width >= 1.0
}

fn width_at(from: f32, target: f32, elapsed: Duration) -> Option<f32> {
    let progress = elapsed.as_secs_f32() / PANEL_SLIDE_DURATION.as_secs_f32();
    (progress < 1.0).then(|| from + (target - from) * ease_out_quint()(progress.max(0.0)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sidebar_preserves_content_width_and_hidden_sidebar_consumes_none() {
        let mut state = WorkspaceShellState::default();
        assert_eq!(
            state.panel_content_width(980.0),
            crate::DEFAULT_LEFT_SIDEBAR_WIDTH
        );
        assert_eq!(state.panel_content_width(480.0), 120.0);
        state.toggle(true);
        assert_eq!(state.panel_content_width(980.0), 0.0);
        state.toggle(true);
        assert_eq!(
            state.snapshot().left_width,
            crate::DEFAULT_LEFT_SIDEBAR_WIDTH
        );
    }

    #[test]
    fn resize_clamps_and_reset_restores_width() {
        let mut state = WorkspaceShellState::default();
        state.begin_resize(100.0);
        assert!(state.resize(1000.0, 660.0));
        assert_eq!(state.snapshot().left_width, 300.0);
        assert!(state.finish_resize());
        assert!(!state.finish_resize());
        state.reset_width(crate::DEFAULT_LEFT_SIDEBAR_WIDTH);
        assert_eq!(
            state.snapshot().left_width,
            crate::DEFAULT_LEFT_SIDEBAR_WIDTH
        );
    }

    #[test]
    fn panel_slide_eases_out_and_retires() {
        assert_eq!(width_at(0.0, 260.0, Duration::ZERO), Some(0.0));
        assert!(width_at(0.0, 260.0, PANEL_SLIDE_DURATION / 2).unwrap() > 130.0);
        assert_eq!(width_at(0.0, 260.0, PANEL_SLIDE_DURATION), None);
    }

    #[gpui_kit::test]
    fn workspace_content_reaches_window_edge_and_sidebar_reopens_after_settings(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        struct Harness {
            shell: AppShellState,
        }
        impl gpui_kit::Render for Harness {
            fn render(
                &mut self,
                window: &mut Window,
                cx: &mut gpui_kit::Context<Self>,
            ) -> impl IntoElement {
                WorkspaceShell::new(
                    self.shell.workspace_mut().layout(window).frame,
                    div().into_any_element(),
                    div()
                        .size_full()
                        .debug_selector(|| "content".to_owned())
                        .into_any_element(),
                    window,
                    cx,
                )
                .into_element()
            }
        }
        cx.update(gpui_kit::component::init);
        let (view, cx) = cx.add_window_view(|_, _| Harness {
            shell: AppShellState::default(),
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("workspace-left-toggle").is_some());
        let bounds = cx.debug_bounds("content").unwrap();
        cx.update(|window, _| assert_eq!(bounds.right(), window.viewport_size().width));
        view.update_in(cx, |view, _, _| {
            view.shell.workspace_mut().toggle(true);
            view.shell.open_settings();
            view.shell.close_settings();
            assert!(!view.shell.workspace().visible());
            view.shell.workspace_mut().toggle(true);
            assert!(view.shell.workspace().visible());
        });
    }
}
