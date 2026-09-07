use crate::localization::Language;
use crate::theme::ThemePreference;

pub const PROJECT_NAME: &str = "locus";
pub const INTERFACE_STATE_FILE: &str = "state.toml";
pub const SIDEBAR_ORDER_FILE: &str = "sidebar-order.toml";
pub const THUMBNAIL_CACHE_DIRECTORY: &str = "thumbnails";
pub const LINK_ICON_CACHE_DIRECTORY: &str = "link-icons";

pub const DEFAULT_LIBRARY_PATH: Option<&str> = None;
pub const DEFAULT_THEME: ThemePreference = ThemePreference::System;
pub const DEFAULT_LANGUAGE: Language = Language::Chinese;
pub const DEFAULT_UI_FONT_FAMILY: &str = ".SystemUIFont";
pub const DEFAULT_UI_FONT_SIZE: f32 = 16.0;
pub const DEFAULT_CODE_FONT_FAMILY: &str = "Menlo";
pub const DEFAULT_CODE_FONT_SIZE: f32 = 14.0;
pub const DEFAULT_TERMINAL_FONT_FAMILY: &str = "Menlo";
pub const DEFAULT_TERMINAL_FONT_SIZE: f32 = 14.0;
pub const DEFAULT_TERMINAL_SHELL: &str = "";
pub const DEFAULT_TERMINAL_COMMAND: &str = "";
pub const DEFAULT_MCP_ENABLED: bool = false;
pub const DEFAULT_MCP_LOCAL_NETWORK: bool = false;
pub const DEFAULT_MCP_PORT: u16 = 2301;
pub const DEFAULT_SHOW_GRAPH_ATTACHMENTS: bool = true;

pub const DEFAULT_LEFT_SIDEBAR_VISIBLE: bool = true;
pub const DEFAULT_RIGHT_SIDEBAR_VISIBLE: bool = true;
pub const DEFAULT_FPS_COUNTER_VISIBLE: bool = false;
pub const DEFAULT_LEFT_SIDEBAR_WIDTH: f32 = 252.0;
pub const DEFAULT_RIGHT_SIDEBAR_WIDTH: f32 = 520.0;

pub const WINDOW_WIDTH: f32 = 980.0;
pub const WINDOW_HEIGHT: f32 = 820.0;
pub const WINDOW_MIN_WIDTH: f32 = 980.0;
pub const WINDOW_MIN_HEIGHT: f32 = 820.0;
pub const WINDOW_TITLEBAR_HEIGHT: f32 = 48.0;
pub const WINDOW_TRAFFIC_LIGHT_CLEARANCE: f32 = 86.0;
