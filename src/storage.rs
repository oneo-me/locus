use std::path::PathBuf;

use crate::config::DATA_DIRECTORY;

pub fn data_directory() -> Result<PathBuf, String> {
    dirs::home_dir()
        .map(|home| home.join(DATA_DIRECTORY))
        .ok_or_else(|| "无法确定用户主目录".to_string())
}
