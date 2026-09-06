mod button;
mod fps;
mod menu;
mod platform;
mod settings;
mod titlebar;
mod workspace;

pub const TITLEBAR_HEIGHT: f32 = 48.0;
pub const TITLEBAR_CONTROL_INSET: f32 = 11.0;
pub const TITLEBAR_CONTROL_SIZE: f32 = 26.0;
pub const MACOS_TRAFFIC_LIGHT_CLEARANCE: f32 = 86.0;

pub use button::toolbar_icon_button;

pub fn titlebar_left_control_offset(fullscreen: bool) -> f32 {
    if cfg!(target_os = "macos") && !fullscreen {
        MACOS_TRAFFIC_LIGHT_CLEARANCE
    } else {
        TITLEBAR_CONTROL_INSET
    }
}

pub fn titlebar_content_left_padding(
    control_clearance: f32,
    rendered_left_width: f32,
    content_inset: f32,
) -> f32 {
    (control_clearance - rendered_left_width).max(content_inset)
}

pub use fps::FpsCounter;
pub use menu::{
    About, AppShellMenuLabels, AppShellMenus, CloseWindow, OpenSettings, Quit, ToggleFpsCounter,
    ToggleFullScreen, ToggleLeftPanel, ToggleRightPanel, bind_app_shell_keys, set_app_shell_menus,
};
pub use platform::{
    close_main_window, configure_main_window_close_behavior, configure_sidebar_material,
    init_reduce_motion,
};
pub use settings::{
    SettingsShell, settings_back_button, settings_navigation_group, settings_navigation_row,
    settings_sidebar_header, sidebar_selection_background,
};
pub use titlebar::titlebar_drag_region;
pub use workspace::{
    AppShellMode, AppShellState, PanelFrame, PanelSide, PanelSize, WorkspaceShell,
    WorkspaceShellConfig, WorkspaceShellLayout, WorkspaceShellSnapshot, WorkspaceShellState,
    panel_resize_handle, panel_toggle_button,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_title_padding_moves_continuously_with_the_left_panel() {
        assert_eq!(titlebar_content_left_padding(124.0, 252.0, 20.0), 20.0);
        assert_eq!(titlebar_content_left_padding(124.0, 50.0, 20.0), 74.0);
        assert_eq!(titlebar_content_left_padding(124.0, 0.0, 20.0), 124.0);
    }
}
