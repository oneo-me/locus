mod platform;
mod titlebar;

pub use titlebar::{
    MACOS_TRAFFIC_LIGHT_CLEARANCE, TITLEBAR_CONTROL_INSET, TITLEBAR_CONTROL_SIZE, TITLEBAR_HEIGHT,
    titlebar_content_left_padding, titlebar_drag_region, titlebar_left_control_offset,
};

pub(crate) use platform::{
    close_main_window, configure_main_window_close_behavior, configure_sidebar_material,
    init_reduce_motion,
};

use anyhow::{Result, anyhow};
use gpui_kit::component::{Root, Theme};
use gpui_kit::{
    App, AppContext, Bounds, Entity, Pixels, Render, SharedString, Size, Styled as _,
    TitlebarOptions, Window, WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowOptions,
    point, px, size,
};

const MIN_VISIBLE_TITLEBAR_WIDTH: f32 = 120.0;
const MIN_VISIBLE_TITLEBAR_HEIGHT: f32 = 24.0;

/// Creates a shell window with one component Root and native titlebar/close behavior.
/// Defaults to 980 × 820 logical pixels, with a 640 × 480 minimum.
pub struct ShellWindow {
    title: SharedString,
    size: Size<Pixels>,
    min_size: Size<Pixels>,
    saved_bounds: Option<Bounds<Pixels>>,
}

impl ShellWindow {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            size: size(px(980.0), px(820.0)),
            min_size: size(px(640.0), px(480.0)),
            saved_bounds: None,
        }
    }

    pub fn min_size(&self) -> Size<Pixels> {
        self.min_size
    }

    pub fn with_size(mut self, size: Size<Pixels>) -> Self {
        self.size = size;
        self
    }

    pub fn with_min_size(mut self, size: Size<Pixels>) -> Self {
        self.min_size = size;
        self
    }

    pub fn with_saved_bounds(mut self, bounds: Option<Bounds<Pixels>>) -> Self {
        self.saved_bounds = bounds;
        self
    }

    /// Builds product content inside Root; the caller owns content state and persistence.
    pub fn open<V: Render + 'static>(
        self,
        cx: &mut App,
        build: impl FnOnce(&mut Window, &mut App) -> Entity<V>,
    ) -> Result<WindowHandle<Root>> {
        let bounds = self
            .saved_bounds
            .filter(|bounds| {
                cx.displays()
                    .iter()
                    .any(|display| window_titlebar_is_reachable(*bounds, display.visible_bounds()))
            })
            .unwrap_or_else(|| Bounds::centered(None, self.size, cx));
        let mut material_error = None;
        let handle = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(self.min_size),
                titlebar: Some(TitlebarOptions {
                    title: Some(self.title),
                    appears_transparent: true,
                    traffic_light_position: Some(point(px(16.0), px(17.0))),
                }),
                app_owns_titlebar_drag: cfg!(target_os = "macos"),
                window_background: if cfg!(target_os = "macos") {
                    WindowBackgroundAppearance::Blurred
                } else {
                    WindowBackgroundAppearance::Opaque
                },
                ..Default::default()
            },
            |window, cx| {
                configure_main_window_close_behavior(window, cx);
                material_error = configure_sidebar_material(window).err();
                let view = build(window, cx);
                cx.new(|cx| {
                    Root::new(view, window, cx)
                        .bordered(false)
                        .bg(Theme::global(cx).transparent)
                })
            },
        )?;
        if let Some(error) = material_error {
            handle.update(cx, |_, window, _| window.remove_window())?;
            return Err(anyhow!(error));
        }
        Ok(handle)
    }
}

fn window_titlebar_is_reachable(
    window_bounds: Bounds<Pixels>,
    visible_bounds: Bounds<Pixels>,
) -> bool {
    let titlebar = Bounds::new(
        window_bounds.origin,
        size(window_bounds.size.width, px(TITLEBAR_HEIGHT)),
    );
    let visible_titlebar = titlebar.intersect(&visible_bounds);
    f32::from(visible_titlebar.size.width) >= MIN_VISIBLE_TITLEBAR_WIDTH
        && f32::from(visible_titlebar.size.height) >= MIN_VISIBLE_TITLEBAR_HEIGHT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_window_requires_a_reachable_titlebar() {
        let display = Bounds::new(point(px(0.0), px(0.0)), size(px(1440.0), px(900.0)));
        let reachable = Bounds::new(point(px(1320.0), px(-24.0)), size(px(1080.0), px(920.0)));
        let unreachable = Bounds::new(point(px(1321.0), px(-25.0)), size(px(1080.0), px(920.0)));

        assert!(window_titlebar_is_reachable(reachable, display));
        assert!(!window_titlebar_is_reachable(unreachable, display));
    }
}
