use std::fs;

use serde::{Deserialize, Serialize};

use crate::config::{
    DEFAULT_FPS_COUNTER_VISIBLE, DEFAULT_LEFT_SIDEBAR_VISIBLE, DEFAULT_LEFT_SIDEBAR_WIDTH,
    DEFAULT_RIGHT_SIDEBAR_VISIBLE, DEFAULT_RIGHT_SIDEBAR_WIDTH, INTERFACE_STATE_FILE,
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
}

impl Default for InterfaceState {
    fn default() -> Self {
        Self {
            left_sidebar_width: DEFAULT_LEFT_SIDEBAR_WIDTH,
            right_sidebar_width: DEFAULT_RIGHT_SIDEBAR_WIDTH,
            left_sidebar_visible: DEFAULT_LEFT_SIDEBAR_VISIBLE,
            right_sidebar_visible: DEFAULT_RIGHT_SIDEBAR_VISIBLE,
            fps_counter_visible: DEFAULT_FPS_COUNTER_VISIBLE,
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
            ..InterfaceState::default()
        };

        state.normalize();

        assert_eq!(state.left_sidebar_width, DEFAULT_LEFT_SIDEBAR_WIDTH);
        assert_eq!(state.right_sidebar_width, RIGHT_SIDEBAR_MAX_WIDTH);
    }
}
