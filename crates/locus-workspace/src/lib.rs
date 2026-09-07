rust_i18n::i18n!("locales", fallback = "en");

mod commands;
mod sidebar_order;
mod state;
mod workspace;

use commands::*;
pub use commands::{bind_keys, set_app_menus};
pub use state::InterfaceState;
pub use workspace::Locus;

use oneo_gpui::{
    OpenSettings, ToggleFpsCounter, ToggleFullScreen, ToggleLeftPanel, ToggleRightPanel,
};

#[cfg(test)]
mod localization_tests;
