use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Config {
    pub package: String,
    pub bin: String,
    pub name: String,
    pub identifier: String,
    #[serde(default)]
    pub resources: Vec<Resource>,
    #[serde(default)]
    pub windows: Windows,
    #[serde(default)]
    pub macos: Macos,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Macos {
    #[serde(default)]
    pub resources: Vec<Resource>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Windows {
    #[serde(default)]
    pub resources: Vec<Resource>,
    #[serde(default)]
    pub runtime_directories: Vec<PathBuf>,
    #[serde(default)]
    pub runtime_files: Vec<PathBuf>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Resource {
    Path(PathBuf),
    Mapped {
        source: PathBuf,
        destination: String,
    },
}

#[derive(Debug)]
pub struct Project {
    pub root: PathBuf,
    pub config: Config,
    pub version: String,
    pub package_id: String,
}

impl Project {
    pub fn load(root: &Path) -> Result<Self> {
        // Keep ordinary Windows paths when possible: dependency build scripts
        // may append `/file`, which is invalid after a verbatim `\\?\` prefix.
        let root = dunce::canonicalize(root)
            .with_context(|| format!("project directory {}", root.display()))?;
        let path = root.join("packer.toml");
        let config: Config = toml::from_str(
            &fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?,
        )
        .with_context(|| format!("parse {}", path.display()))?;
        for value in [&config.name, &config.bin, &config.package] {
            component(value)?;
        }
        ensure!(
            !config.identifier.is_empty()
                && config
                    .identifier
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-'),
            "invalid bundle identifier"
        );
        let metadata = crate::cargo::metadata(&root)?;
        let package = metadata
            .packages
            .iter()
            .find(|p| p.name == config.package && metadata.workspace_members.contains(&p.id))
            .with_context(|| format!("{} is not a workspace package", config.package))?;
        ensure!(
            package
                .targets
                .iter()
                .any(|t| t.name == config.bin && t.kind.iter().any(|k| k == "bin")),
            "package {} has no binary {}",
            config.package,
            config.bin
        );
        Ok(Self {
            root,
            config,
            version: package.version.clone(),
            package_id: package.id.clone(),
        })
    }
}

pub fn component(value: &str) -> Result<()> {
    ensure!(
        !value.is_empty()
            && value != "."
            && value != ".."
            && !value.ends_with([' ', '.'])
            && !value
                .chars()
                .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c)),
        "invalid file name {value:?}"
    );
    let stem = value.split('.').next().unwrap_or("").to_ascii_uppercase();
    ensure!(
        !matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            && !(stem.len() == 4
                && (stem.starts_with("COM") || stem.starts_with("LPT"))
                && matches!(stem.as_bytes()[3], b'1'..=b'9')),
        "reserved Windows file name {value:?}"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    #[test]
    fn loaded_project_supports_dependency_paths_with_forward_slashes() {
        let directory = tempfile::Builder::new()
            .prefix("packer 项目 ")
            .tempdir()
            .unwrap();
        let root = directory.path();
        std::fs::write(root.join("Cargo.toml"), "[package]\nname='app'\nversion='0.1.0'\nedition='2024'\n[workspace]\n[[bin]]\nname='app'\npath='main.rs'\n").unwrap();
        std::fs::write(root.join("main.rs"), "fn main() {}\n").unwrap();
        std::fs::write(
            root.join("packer.toml"),
            "package='app'\nbin='app'\nname='App'\nidentifier='me.app'\n",
        )
        .unwrap();
        let project = super::Project::load(&root.canonicalize().unwrap()).unwrap();
        // Mirror dependency build scripts that form filenames with format!.
        let manifest = format!("{}/Cargo.toml", project.root.display());
        assert_eq!(
            std::fs::read(manifest).unwrap(),
            std::fs::read(root.join("Cargo.toml")).unwrap()
        );
    }

    use super::*;
    #[test]
    fn output_names_cannot_escape_or_use_windows_reserved_names() {
        for name in ["../escape", "a/b", "a\\b", "NUL", "COM1.txt", "Locus."] {
            assert!(component(name).is_err(), "{name}");
        }
        assert!(component("Opus Index").is_ok());
    }
}
