use std::path::PathBuf;

use oneo_gpui::SettingsStore;

use crate::config::PROJECT_NAME;

pub fn settings_store() -> Result<SettingsStore, String> {
    SettingsStore::for_project(PROJECT_NAME).map_err(|error| format!("{error:#}"))
}

pub fn data_directory() -> Result<PathBuf, String> {
    settings_store().map(|store| {
        store
            .path()
            .parent()
            .expect("absolute settings path has a parent")
            .to_path_buf()
    })
}
