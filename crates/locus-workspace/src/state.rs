use oneo_gpui::SettingsStore;

use gpui_kit::{Bounds, Pixels, px, size};
use serde::{Deserialize, Serialize};

use locus_preferences::config::{
    DEFAULT_FPS_COUNTER_VISIBLE, DEFAULT_LEFT_SIDEBAR_VISIBLE, DEFAULT_LEFT_SIDEBAR_WIDTH,
    DEFAULT_RIGHT_SIDEBAR_VISIBLE, DEFAULT_RIGHT_SIDEBAR_WIDTH, INTERFACE_STATE_FILE,
    WINDOW_MIN_HEIGHT, WINDOW_MIN_WIDTH,
};
use locus_preferences::storage::data_directory;

pub const LEFT_SIDEBAR_MIN_WIDTH: f32 = DEFAULT_LEFT_SIDEBAR_WIDTH;
pub const LEFT_SIDEBAR_MAX_WIDTH: f32 = 420.0;
pub const RIGHT_SIDEBAR_MIN_WIDTH: f32 = DEFAULT_RIGHT_SIDEBAR_WIDTH;
pub const RIGHT_SIDEBAR_MAX_WIDTH: f32 = 1000.0;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
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
        match state_store()
            .and_then(|store| store.load::<Self>().map_err(|error| format!("{error:#}")))
        {
            Ok(mut state) => {
                state.normalize();
                (state, None)
            }
            Err(error) => (Self::default(), Some(format!("无法读取界面状态：{error}"))),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        state_store()?
            .save(self)
            .map_err(|error| format!("{error:#}"))
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

fn state_store() -> Result<SettingsStore, String> {
    let path = data_directory()?.join(INTERFACE_STATE_FILE);
    SettingsStore::at_path(path).map_err(|error| format!("{error:#}"))
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
                gpui_kit::point(px(120.0), px(80.0)),
                size(px(1280.0), px(960.0)),
            )),
        };

        let directory = tempfile::tempdir().unwrap();
        let store = SettingsStore::at_path(directory.path().join("state.toml")).unwrap();
        store.save(&state).unwrap();
        let restored: InterfaceState = store.load().unwrap();

        assert_eq!(restored, state);
    }

    #[test]
    fn invalid_widths_are_normalized() {
        let mut state = InterfaceState {
            left_sidebar_width: f32::NAN,
            right_sidebar_width: 1200.0,
            selected_library_node: Some("../outside".to_owned()),
            window_bounds: Some(Bounds::new(
                gpui_kit::point(px(f32::NAN), px(40.0)),
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
    fn left_sidebar_width_uses_the_default_as_its_minimum() {
        let mut state = InterfaceState {
            left_sidebar_width: 180.0,
            ..InterfaceState::default()
        };

        state.normalize();

        assert_eq!(LEFT_SIDEBAR_MIN_WIDTH, DEFAULT_LEFT_SIDEBAR_WIDTH);
        assert_eq!(state.left_sidebar_width, DEFAULT_LEFT_SIDEBAR_WIDTH);
    }

    #[test]
    fn undersized_window_bounds_are_normalized() {
        let mut state = InterfaceState {
            window_bounds: Some(Bounds::new(
                gpui_kit::point(px(120.0), px(80.0)),
                size(px(800.0), px(600.0)),
            )),
            ..InterfaceState::default()
        };

        state.normalize();

        let bounds = state.window_bounds.unwrap();
        assert_eq!(f32::from(bounds.size.width), WINDOW_MIN_WIDTH);
        assert_eq!(f32::from(bounds.size.height), WINDOW_MIN_HEIGHT);
    }

    #[test]
    fn locus_right_sidebar_defaults_and_restored_widths_respect_520() {
        assert_eq!(InterfaceState::default().right_sidebar_width, 520.0);
        let mut state = InterfaceState {
            right_sidebar_width: 320.0,
            ..InterfaceState::default()
        };
        state.normalize();
        assert_eq!(state.right_sidebar_width, 520.0);
        assert_eq!(WINDOW_MIN_WIDTH, 980.0);
        assert_eq!(oneo_gpui::DEFAULT_RIGHT_SIDEBAR_WIDTH, 320.0);
        assert_eq!(oneo_gpui::RIGHT_SIDEBAR_MIN_WIDTH, 320.0);
    }
}
