use std::path::PathBuf;

pub fn data_directory() -> Result<PathBuf, String> {
    dirs::home_dir()
        .map(|home| home.join(".oneo").join("locus_native"))
        .ok_or_else(|| "无法确定用户主目录".to_string())
}
