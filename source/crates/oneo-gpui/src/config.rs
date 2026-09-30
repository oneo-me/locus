use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, ensure};
use serde::{Serialize, de::DeserializeOwned};

/// Reads and atomically replaces a typed TOML file without retaining application state.
#[derive(Clone, Debug)]
pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    /// Resolves ~/.oneo/<name>/settings.toml; name must be one directory component.
    pub fn for_project(name: &str) -> Result<Self> {
        let home = dirs::home_dir().context("Cannot determine the home directory")?;
        Self::in_home(&home, name)
    }

    fn in_home(home: &Path, name: &str) -> Result<Self> {
        let mut components = Path::new(name).components();
        ensure!(
            !name.is_empty()
                && !name.contains(['/', '\\'])
                && matches!(components.next(), Some(Component::Normal(_)))
                && components.next().is_none(),
            "Project name must be a single directory name"
        );
        Self::at_path(home.join(".oneo").join(name).join("settings.toml"))
    }

    /// Uses an absolute file path verbatim, without expanding shell syntax such as `~`.
    pub fn at_path(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        ensure!(
            path.is_absolute() && path.file_name().is_some(),
            "Settings path must be an absolute file path"
        );
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Missing files return T::default(); parse and IO failures remain errors.
    /// Missing fields are controlled by T's Serde schema, for example #[serde(default)].
    pub fn load<T: DeserializeOwned + Default>(&self) -> Result<T> {
        Ok(self.read()?.unwrap_or_default())
    }

    pub fn read<T: DeserializeOwned>(&self) -> Result<Option<T>> {
        let text = match fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("Cannot read settings {}", self.path.display()));
            }
        };
        toml::from_str(&text)
            .map(Some)
            .with_context(|| format!("Cannot parse settings {}", self.path.display()))
    }

    /// Replaces the complete document; callers serialize concurrent updates (last writer wins).
    /// Comments and unknown fields are not retained by typed serialization.
    pub fn save<T: Serialize>(&self, value: &T) -> Result<()> {
        self.write(value, true)
    }

    /// Creates a document without replacing an existing file, including concurrent creations.
    pub fn create<T: Serialize>(&self, value: &T) -> Result<()> {
        self.write(value, false)
    }

    fn write<T: Serialize>(&self, value: &T, replace: bool) -> Result<()> {
        let text = toml::to_string_pretty(value).context("Cannot serialize settings")?;
        let directory = self.path.parent().context("Settings path has no parent")?;
        fs::create_dir_all(directory)
            .with_context(|| format!("Cannot create settings directory {}", directory.display()))?;
        let mut file = tempfile::NamedTempFile::new_in(directory)
            .context("Cannot create settings temporary file")?;
        file.write_all(text.as_bytes())
            .context("Cannot write settings")?;
        file.as_file().sync_all().context("Cannot flush settings")?;
        if replace {
            file.persist(&self.path)
        } else {
            file.persist_noclobber(&self.path)
        }
        .with_context(|| format!("Cannot write settings {}", self.path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
    #[serde(default)]
    struct Preferences {
        name: String,
        enabled: bool,
        library: Option<PathBuf>,
    }

    #[test]
    fn project_and_custom_paths() {
        let home = tempfile::tempdir().unwrap();
        assert_eq!(
            SettingsStore::in_home(home.path(), "my-app")
                .unwrap()
                .path(),
            home.path().join(".oneo/my-app/settings.toml")
        );
        for name in ["", ".", "..", "../app", "/app", "app/", "app\\child"] {
            assert!(SettingsStore::in_home(home.path(), name).is_err(), "{name}");
        }
        let path = home.path().join("custom/preferences.toml");
        assert_eq!(SettingsStore::at_path(&path).unwrap().path(), path);
        assert!(SettingsStore::at_path("relative.toml").is_err());
    }

    #[test]
    fn create_does_not_replace_an_existing_document() {
        let directory = tempfile::tempdir().unwrap();
        let store = SettingsStore::at_path(directory.path().join("info.toml")).unwrap();
        assert_eq!(store.read::<Preferences>().unwrap(), None);
        let original = Preferences {
            name: "original".into(),
            ..Preferences::default()
        };
        store.create(&original).unwrap();
        assert!(store.create(&Preferences::default()).is_err());
        assert_eq!(store.load::<Preferences>().unwrap(), original);
    }

    #[test]
    fn serialization_failure_preserves_the_existing_file() {
        let directory = tempfile::tempdir().unwrap();
        let store = SettingsStore::at_path(directory.path().join("settings.toml")).unwrap();
        store
            .save(&Preferences {
                enabled: true,
                ..Preferences::default()
            })
            .unwrap();
        let before = fs::read(store.path()).unwrap();
        assert!(store.save(&42).is_err());
        assert_eq!(fs::read(store.path()).unwrap(), before);
    }

    #[test]
    fn missing_partial_invalid_and_replaced_settings() {
        let directory = tempfile::tempdir().unwrap();
        let store = SettingsStore::at_path(directory.path().join("nested/settings.toml")).unwrap();
        assert_eq!(store.load::<Preferences>().unwrap(), Preferences::default());
        assert!(!store.path().exists());
        let value = Preferences {
            name: "中文".into(),
            enabled: true,
            library: Some(PathBuf::from("/tmp/笔记")),
        };
        store.save(&value).unwrap();
        assert_eq!(store.load::<Preferences>().unwrap(), value);
        store.save(&Preferences::default()).unwrap();
        assert_eq!(store.load::<Preferences>().unwrap(), Preferences::default());
        fs::write(store.path(), "enabled = true").unwrap();
        assert!(store.load::<Preferences>().unwrap().enabled);
        fs::write(store.path(), "enabled = [").unwrap();
        assert!(store.load::<Preferences>().is_err());
        assert_eq!(fs::read_to_string(store.path()).unwrap(), "enabled = [");
        fs::remove_file(store.path()).unwrap();
        fs::create_dir(store.path()).unwrap();
        assert!(store.load::<Preferences>().is_err());
        assert!(store.save(&value).is_err());
    }
}
