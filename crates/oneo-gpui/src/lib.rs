#[cfg(feature = "shell")]
pub mod app;
#[cfg(feature = "shell")]
mod button;
pub mod config;
#[cfg(feature = "shell")]
mod fps;
#[cfg(feature = "shell")]
mod settings;
#[cfg(feature = "shell")]
pub mod window;
#[cfg(feature = "shell")]
mod workspace;

#[cfg(feature = "shell")]
pub use app::{
    About, AppShell, AppShellMenuLabels, AppShellMenus, CloseWindow, OpenSettings, Quit,
    ToggleFpsCounter, ToggleFullScreen, ToggleLeftPanel, ToggleRightPanel, set_app_shell_menus,
};
#[cfg(feature = "shell")]
pub use button::toolbar_icon_button;
pub use config::SettingsStore;
#[cfg(feature = "shell")]
pub use fps::FpsCounter;
#[cfg(feature = "shell")]
pub use settings::{
    SettingsShell, settings_back_button, settings_navigation_group, settings_navigation_row,
    settings_sidebar_header, sidebar_selection_background,
};
#[cfg(feature = "shell")]
pub use window::{
    MACOS_TRAFFIC_LIGHT_CLEARANCE, ShellWindow, TITLEBAR_CONTROL_INSET, TITLEBAR_CONTROL_SIZE,
    TITLEBAR_HEIGHT, titlebar_content_left_padding, titlebar_drag_region,
    titlebar_left_control_offset,
};
#[cfg(feature = "shell")]
pub use workspace::{
    AppShellMode, AppShellState, PanelFrame, PanelSide, PanelSize, WorkspaceShell,
    WorkspaceShellConfig, WorkspaceShellLayout, WorkspaceShellSnapshot, WorkspaceShellState,
    panel_resize_handle, panel_toggle_button,
};
