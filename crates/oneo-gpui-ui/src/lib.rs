pub mod app_shell;

mod activation;
mod button;
mod icon;
mod platform;
mod scrollbar;
mod theme;

pub use activation::ActivationExt;
pub use button::toolbar_icon_button;
pub use icon::{Icon, icon};
pub use platform::{
    close_main_window, configure_main_window_close_behavior, configure_sidebar_material,
    init_reduce_motion,
};
pub use scrollbar::{ScrollbarState, vertical_scrollbar};
pub use theme::{Theme, set_theme};
