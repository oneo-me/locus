mod settings;
mod workspace;

pub use settings::{SettingsShell, settings_back_button};
pub use workspace::{
    PanelFrame, PanelSide, PanelSize, PanelSlide, WorkspaceShell, WorkspaceShellConfig,
    fitted_panel_widths, panel_toggle_button, slide_width,
};
