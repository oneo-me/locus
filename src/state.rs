use std::fs;

use gpui::{Bounds, Pixels, px, size};
use serde::{Deserialize, Serialize};

use crate::config::{
    DEFAULT_FPS_COUNTER_VISIBLE, DEFAULT_LEFT_SIDEBAR_VISIBLE, DEFAULT_LEFT_SIDEBAR_WIDTH,
    DEFAULT_RIGHT_SIDEBAR_VISIBLE, DEFAULT_RIGHT_SIDEBAR_WIDTH, INTERFACE_STATE_FILE,
    WINDOW_MIN_HEIGHT, WINDOW_MIN_WIDTH,
};
use crate::storage::data_directory;

pub const LEFT_SIDEBAR_MIN_WIDTH: f32 = 180.0;
pub const LEFT_SIDEBAR_MAX_WIDTH: f32 = 420.0;
pub const RIGHT_SIDEBAR_MIN_WIDTH: f32 = 280.0;
pub const RIGHT_SIDEBAR_MAX_WIDTH: f32 = 1000.0;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InterfaceState {
    pub left_sidebar_width: f32,
    pub right_sidebar_width: f32,
    pub left_sidebar_visible: bool,
    pub right_sidebar_visible: bool,
    pub fps_counter_visible: bool,
    pub selected_library_node: Option<String>,
    pub window_bounds: Option<Bounds<Pixels>>,
}

impl Default for InterfaceState {
    fn default() -> Self {
        Self {
            left_sidebar_width: DEFAULT_LEFT_SIDEBAR_WIDTH,
            right_sidebar_width: DEFAULT_RIGHT_SIDEBAR_WIDTH,
            left_sidebar_visible: DEFAULT_LEFT_SIDEBAR_VISIBLE,
            right_sidebar_visible: DEFAULT_RIGHT_SIDEBAR_VISIBLE,
            fps_counter_visible: DEFAULT_FPS_COUNTER_VISIBLE,
            selected_library_node: None,
            window_bounds: None,
        }
    }
}

impl InterfaceState {
    pub fn load() -> (Self, Option<String>) {
        let path = match state_path() {
            Ok(path) => path,
            Err(error) => return (Self::default(), Some(error)),
        };
        if !path.exists() {
            return (Self::default(), None);
        }

        match fs::read(&path)
            .map_err(|error| error.to_string())
            .and_then(|bytes| {
                serde_json::from_slice::<Self>(&bytes).map_err(|error| error.to_string())
            }) {
            Ok(mut state) => {
                state.normalize();
                (state, None)
            }
            Err(error) => (Self::default(), Some(format!("无法读取界面状态：{error}"))),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path = state_path()?;
        let directory = path
            .parent()
            .ok_or_else(|| "界面状态路径无效".to_string())?;
        fs::create_dir_all(directory).map_err(|error| error.to_string())?;
        let data = serde_json::to_vec_pretty(self).map_err(|error| error.to_string())?;
        fs::write(path, data).map_err(|error| error.to_string())
    }

    fn normalize(&mut self) {
        if !self.left_sidebar_width.is_finite() {
            self.left_sidebar_width = DEFAULT_LEFT_SIDEBAR_WIDTH;
        }
        if !self.right_sidebar_width.is_finite() {
            self.right_sidebar_width = DEFAULT_RIGHT_SIDEBAR_WIDTH;
        }
        self.left_sidebar_width = self
            .left_sidebar_width
            .clamp(LEFT_SIDEBAR_MIN_WIDTH, LEFT_SIDEBAR_MAX_WIDTH);
        self.right_sidebar_width = self
            .right_sidebar_width
            .clamp(RIGHT_SIDEBAR_MIN_WIDTH, RIGHT_SIDEBAR_MAX_WIDTH);
        self.window_bounds = self.window_bounds.take().and_then(|mut bounds| {
            let values = [
                f32::from(bounds.origin.x),
                f32::from(bounds.origin.y),
                f32::from(bounds.size.width),
                f32::from(bounds.size.height),
            ];
            values.iter().all(|value| value.is_finite()).then(|| {
                bounds.size = size(
                    px(f32::from(bounds.size.width).max(WINDOW_MIN_WIDTH)),
                    px(f32::from(bounds.size.height).max(WINDOW_MIN_HEIGHT)),
                );
                bounds
            })
        });
        self.selected_library_node = self.selected_library_node.take().and_then(|path| {
            let path = path.trim();
            let valid = !path.is_empty()
                && !path.starts_with('/')
                && !path.contains('\\')
                && path
                    .split('/')
                    .all(|segment| !segment.is_empty() && segment != "." && segment != "..");
            valid.then(|| path.to_owned())
        });
    }
}

fn state_path() -> Result<std::path::PathBuf, String> {
    data_directory().map(|directory| directory.join(INTERFACE_STATE_FILE))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interface_state_round_trip() {
        let state = InterfaceState {
            left_sidebar_width: 320.0,
            right_sidebar_width: 640.0,
            left_sidebar_visible: false,
            right_sidebar_visible: true,
            fps_counter_visible: true,
            selected_library_node: Some("projects/locus".to_owned()),
            window_bounds: Some(Bounds::new(
                gpui::point(px(120.0), px(80.0)),
                size(px(1280.0), px(960.0)),
            )),
        };

        let json = serde_json::to_string(&state).unwrap();
        let restored: InterfaceState = serde_json::from_str(&json).unwrap();

        assert_eq!(restored, state);
    }

    #[test]
    fn invalid_widths_are_normalized() {
        let mut state = InterfaceState {
            left_sidebar_width: f32::NAN,
            right_sidebar_width: 1200.0,
            selected_library_node: Some("../outside".to_owned()),
            window_bounds: Some(Bounds::new(
                gpui::point(px(f32::NAN), px(40.0)),
                size(px(800.0), px(600.0)),
            )),
            ..InterfaceState::default()
        };

        state.normalize();

        assert_eq!(state.left_sidebar_width, DEFAULT_LEFT_SIDEBAR_WIDTH);
        assert_eq!(state.right_sidebar_width, RIGHT_SIDEBAR_MAX_WIDTH);
        assert_eq!(state.selected_library_node, None);
        assert_eq!(state.window_bounds, None);
    }

    #[test]
    fn undersized_window_bounds_are_normalized() {
        let mut state = InterfaceState {
            window_bounds: Some(Bounds::new(
                gpui::point(px(120.0), px(80.0)),
                size(px(800.0), px(600.0)),
            )),
            ..InterfaceState::default()
        };

        state.normalize();

        let bounds = state.window_bounds.unwrap();
        assert_eq!(f32::from(bounds.size.width), WINDOW_MIN_WIDTH);
        assert_eq!(f32::from(bounds.size.height), WINDOW_MIN_HEIGHT);
    }
}
