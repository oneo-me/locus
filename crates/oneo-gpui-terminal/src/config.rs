use gpui::SharedString;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TerminalColorScheme {
    #[default]
    Auto,
    Light,
    Dark,
}

impl TerminalColorScheme {
    pub const ALL: [Self; 3] = [Self::Auto, Self::Light, Self::Dark];
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TerminalCursorStyle {
    #[default]
    Block,
    Underline,
    Bar,
}

impl TerminalCursorStyle {
    pub const ALL: [Self; 3] = [Self::Block, Self::Underline, Self::Bar];
}

#[derive(Clone)]
pub struct TerminalLabels {
    pub accessibility_label: SharedString,
    pub could_not_start: SharedString,
    pub starting: SharedString,
    pub process_exited: SharedString,
}

#[derive(Clone)]
pub struct TerminalConfiguration {
    pub shell: String,
    pub startup_command: String,
    pub color_scheme: TerminalColorScheme,
    pub cursor_style: TerminalCursorStyle,
    pub font_family: SharedString,
    pub font_size: f32,
    pub labels: TerminalLabels,
}
