#[cfg(feature = "shell")]
rust_i18n::i18n!("locales", fallback = "en");

pub mod appearance;
#[cfg(feature = "shell")]
pub use appearance::AppearanceControls;
pub use appearance::{
    AppearanceChange, AppearancePreferences, FONT_SIZES, Language, ThemePreference,
};

#[cfg(feature = "shell")]
pub mod app;
#[cfg(feature = "shell")]
mod button;
pub mod config;
#[cfg(feature = "shell")]
mod settings;
#[cfg(feature = "shell")]
pub mod window;
#[cfg(feature = "shell")]
mod workspace;
#[cfg(feature = "shell")]
mod workspace_panel;
#[cfg(feature = "shell")]
pub use workspace_panel::{
    DEFAULT_LEFT_SIDEBAR_WIDTH, DEFAULT_RIGHT_SIDEBAR_WIDTH, LEFT_SIDEBAR_MIN_WIDTH,
    RIGHT_SIDEBAR_MIN_WIDTH, WorkspacePanel, WorkspaceSidebar,
};

#[cfg(feature = "shell")]
pub use app::{
    About, AppShell, AppShellMenuLabels, AppShellMenus, CloseSettings, CloseWindow, OpenSettings,
    Quit, ToggleFullScreen, ToggleLeftPanel, ToggleRightPanel, set_app_shell_menus,
};
#[cfg(feature = "shell")]
pub use button::{toolbar_dropdown_menu, toolbar_icon_button};
pub use config::SettingsStore;
#[cfg(feature = "shell")]
pub use settings::{
    SettingsGroup, SettingsItem, SettingsShell, settings_back_button, settings_button,
    settings_navigation_group, settings_navigation_row, sidebar_selection_background,
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

#[cfg(all(test, feature = "shell"))]
mod localization_tests;
