#![cfg(target_os = "macos")]

mod native;
mod view;

pub use view::WebView;
pub use wry::{NewWindowResponse, PageLoadEvent, WebViewBuilder};
