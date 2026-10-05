use std::{
    fs, io,
    path::{Path, PathBuf},
};

use super::*;

impl Settings {
    fn skills_base(&self) -> Option<PathBuf> {
        match self.preferences.skills_location {
            SkillsLocation::Library => self.preferences.library_path.clone(),
            SkillsLocation::Global => dirs::home_dir(),
        }
    }

    pub(super) fn install_skills(&mut self, cx: &mut Context<Self>) {
        if self.skills_installing {
            return;
        }
        let Some(base) = self.skills_base() else {
            return;
        };
        let previous_base = match self.preferences.skills_location {
            SkillsLocation::Library => dirs::home_dir(),
            SkillsLocation::Global => self.preferences.library_path.clone(),
        };
        self.skills_installing = true;
        self.skills_result = None;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    bundled_skill_directory()
                        .and_then(|source| relocate(&base, previous_base.as_deref(), &source))
                        .map_err(|error| error.to_string())
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.skills_installing = false;
                this.skills_result = Some(result);
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn render_skills(&self, cx: &mut Context<Self>) -> gpui_kit::Div {
        let language = self.preferences.appearance.language;
        let theme = Theme::global(cx);
        let message = if self.skills_installing {
            Some(t!("settings.skills.installing", locale = language.locale()).into_owned())
        } else {
            self.skills_result
                .as_ref()
                .map(|result| match result {
                    Ok(path) => t!(
                        "settings.skills.installed",
                        locale = language.locale(),
                        path = path.display().to_string()
                    )
                    .into_owned(),
                    Err(error) => t!(
                        "settings.skills.failed",
                        locale = language.locale(),
                        error = error
                    )
                    .into_owned(),
                })
                .or_else(|| {
                    self.skills_base().is_none().then(|| {
                        t!(
                            "settings.skills.library_required",
                            locale = language.locale()
                        )
                        .into_owned()
                    })
                })
        };
        let message_color = if matches!(self.skills_result, Some(Err(_))) {
            theme.danger
        } else {
            theme.muted_foreground
        };
        div()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .child(SettingsItem::new(
                t!("settings.skills.location", locale = language.locale()).into_owned(),
                t!("settings.skills.description", locale = language.locale()).into_owned(),
                self.render_dropdown(Dropdown::SkillsLocation, cx),
            ))
            .when_some(message, |view, message| {
                view.child(
                    div()
                        .px_5()
                        .pb_3()
                        .text_xs()
                        .text_color(message_color)
                        .child(message),
                )
            })
    }
}

// Development and release installs both use the running application's resources.
fn bundled_skill_directory() -> io::Result<PathBuf> {
    skill_directory_for_executable(&std::env::current_exe()?)
}

fn skill_directory_for_executable(executable: &Path) -> io::Result<PathBuf> {
    let directory = executable
        .parent()
        .ok_or_else(|| io::Error::other("Missing executable directory"))?;
    #[cfg(target_os = "macos")]
    if directory.file_name().is_some_and(|name| name == "MacOS") {
        return directory.join("../Resources/skills/locus").canonicalize();
    }
    directory.join("resources/skills/locus").canonicalize()
}

// Install first so a destination conflict never removes the working old link.
fn relocate(base: &Path, previous_base: Option<&Path>, source: &Path) -> io::Result<PathBuf> {
    let target = install(base, source)?;
    if let Some(previous_base) = previous_base {
        uninstall(previous_base, &target).map_err(|error| {
            io::Error::other(format!(
                "Installed at {}; could not remove the old link: {error}",
                target.display()
            ))
        })?;
    }
    Ok(target)
}

fn uninstall(base: &Path, installed: &Path) -> io::Result<()> {
    let base = match base.canonicalize() {
        Ok(base) => base,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    let target = base.join(".agents/skills/locus");
    if target == installed {
        return Ok(());
    }
    // Never follow redirected parent directories during cleanup.
    for relative in [".agents", ".agents/skills"] {
        match fs::symlink_metadata(base.join(relative)) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Ok(_) => {
                return Err(io::Error::other(
                    "Old skill parent is not a regular directory",
                ));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        }
    }
    match fs::symlink_metadata(&target) {
        Ok(metadata) if metadata.file_type().is_symlink() => remove_directory_symlink(&target),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(unix)]
fn remove_directory_symlink(path: &Path) -> io::Result<()> {
    fs::remove_file(path)
}

#[cfg(windows)]
fn remove_directory_symlink(path: &Path) -> io::Result<()> {
    fs::remove_dir(path)
}

// Replacing a link never modifies its destination. A real directory is never removed.
fn install(base: &Path, source: &Path) -> io::Result<PathBuf> {
    let base = base.canonicalize()?;
    let source = source.canonicalize()?;
    if !source.join("SKILL.md").is_file() {
        return Err(io::Error::other(format!(
            "Missing SKILL.md: {}",
            source.display()
        )));
    }
    for relative in [".agents", ".agents/skills"] {
        ensure_directory(&base.join(relative))?;
    }
    let parent = base.join(".agents/skills");
    let target = parent.join("locus");
    let previous = match fs::symlink_metadata(&target) {
        Ok(metadata) if metadata.file_type().is_symlink() => true,
        Ok(_) => {
            return Err(io::Error::other(format!(
                "Move the existing directory or file and retry: {}",
                target.display()
            )));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => false,
        Err(error) => return Err(error),
    };
    let staging = tempfile::tempdir_in(&parent)?;
    let link = staging.path().join("locus");
    directory_symlink(&source, &link)?;
    let backup = staging.path().join("previous");
    if previous {
        fs::rename(&target, &backup)?;
    }
    if let Err(error) = fs::rename(&link, &target) {
        if previous && let Err(restore) = fs::rename(&backup, &target) {
            let backup_directory = staging.keep();
            return Err(io::Error::other(format!(
                "{error}; could not restore previous link: {restore}; retained at {}",
                backup_directory.join("previous").display()
            )));
        }
        return Err(error);
    }
    Ok(target)
}

#[cfg(unix)]
fn directory_symlink(source: &Path, target: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(source, target)
}

#[cfg(windows)]
fn directory_symlink(source: &Path, target: &Path) -> io::Result<()> {
    std::os::windows::fs::symlink_dir(source, target)
}

fn ensure_directory(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Ok(()),
        Ok(_) => Err(io::Error::other(format!(
            "Not a regular directory: {}",
            path.display()
        ))),
        Err(error) if error.kind() == io::ErrorKind::NotFound => fs::create_dir(path),
        Err(error) => Err(error),
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn skill(root: &Path, name: &str) -> PathBuf {
        let path = root.join(name);
        fs::create_dir(&path).unwrap();
        fs::write(path.join("SKILL.md"), name).unwrap();
        path
    }

    #[test]
    fn resolves_packaged_skills_without_source_fallback() {
        let root = tempfile::tempdir().unwrap();
        for name in ["Locus", "Locus Dev"] {
            let directory = root.path().join(name);
            let resources = directory.join("resources/skills/locus");
            fs::create_dir_all(&resources).unwrap();
            let executable = directory.join("locus.exe");
            assert_eq!(
                skill_directory_for_executable(&executable).unwrap(),
                resources.canonicalize().unwrap()
            );
            fs::remove_dir_all(&resources).unwrap();
            assert_eq!(
                skill_directory_for_executable(&executable)
                    .unwrap_err()
                    .kind(),
                io::ErrorKind::NotFound
            );
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_development_and_release_use_only_bundle_resources() {
        let root = tempfile::tempdir().unwrap();
        for name in ["Locus", "Locus Dev"] {
            let contents = root.path().join(format!("{name}.app/Contents"));
            let executable = contents.join("MacOS").join(name);
            fs::create_dir_all(executable.parent().unwrap()).unwrap();
            let resources = contents.join("Resources/skills/locus");
            fs::create_dir_all(&resources).unwrap();
            assert_eq!(
                skill_directory_for_executable(&executable).unwrap(),
                resources.canonicalize().unwrap()
            );
            fs::remove_dir_all(&resources).unwrap();
            assert_eq!(
                skill_directory_for_executable(&executable)
                    .unwrap_err()
                    .kind(),
                io::ErrorKind::NotFound
            );
        }
    }

    #[test]
    fn relocates_in_both_directions_and_preserves_resources_and_neighbors() {
        let library = tempfile::tempdir().unwrap();
        let global = tempfile::tempdir().unwrap();
        let resources = tempfile::tempdir().unwrap();
        let source = skill(resources.path(), "source");
        let old = install(library.path(), &source).unwrap();
        let sibling = skill(old.parent().unwrap(), "other");
        let new = relocate(global.path(), Some(library.path()), &source).unwrap();
        assert!(fs::symlink_metadata(&old).is_err());
        assert!(sibling.join("SKILL.md").is_file());
        assert!(source.join("SKILL.md").is_file());
        relocate(library.path(), Some(global.path()), &source).unwrap();
        assert!(fs::symlink_metadata(new).is_err());
        assert!(old.join("SKILL.md").is_file());
        relocate(library.path(), Some(library.path()), &source).unwrap();
        assert!(old.join("SKILL.md").is_file());
    }

    #[test]
    fn relocation_conflicts_keep_old_links_and_cleanup_preserves_real_directories() {
        let old_base = tempfile::tempdir().unwrap();
        let new_base = tempfile::tempdir().unwrap();
        let resources = tempfile::tempdir().unwrap();
        let source = skill(resources.path(), "source");
        let old = install(old_base.path(), &source).unwrap();
        let new = install(new_base.path(), &source).unwrap();
        fs::remove_file(&new).unwrap();
        fs::create_dir(&new).unwrap();
        assert!(relocate(new_base.path(), Some(old_base.path()), &source).is_err());
        assert!(old.join("SKILL.md").is_file());
        relocate(old_base.path(), Some(new_base.path()), &source).unwrap();
        assert!(new.is_dir());
    }

    #[test]
    fn cleanup_rejects_redirected_parents_and_removes_dangling_links() {
        let old_base = tempfile::tempdir().unwrap();
        let new_base = tempfile::tempdir().unwrap();
        let resources = tempfile::tempdir().unwrap();
        let source = skill(resources.path(), "source");
        let old = install(old_base.path(), &source).unwrap();
        fs::remove_file(&old).unwrap();
        directory_symlink(&resources.path().join("missing"), &old).unwrap();
        let new = relocate(new_base.path(), Some(old_base.path()), &source).unwrap();
        assert!(fs::symlink_metadata(&old).is_err());
        let redirected = tempfile::tempdir().unwrap();
        directory_symlink(
            &old_base.path().join(".agents"),
            &redirected.path().join(".agents"),
        )
        .unwrap();
        assert!(uninstall(redirected.path(), &new).is_err());
    }

    #[test]
    fn links_resources_and_updates_links_without_modifying_old_sources_or_other_skills() {
        let base = tempfile::tempdir().unwrap();
        let resources = tempfile::tempdir().unwrap();
        let first = skill(resources.path(), "first");
        let target = install(base.path(), &first).unwrap();
        assert!(
            fs::symlink_metadata(&target)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(
            fs::read_link(&target).unwrap(),
            first.canonicalize().unwrap()
        );
        fs::write(first.join("SKILL.md"), "updated in bundle").unwrap();
        assert_eq!(
            fs::read_to_string(target.join("SKILL.md")).unwrap(),
            "updated in bundle"
        );
        let sibling = skill(target.parent().unwrap(), "other-skill");
        let second = skill(resources.path(), "second");
        install(base.path(), &second).unwrap();
        assert_eq!(
            fs::read_link(&target).unwrap(),
            second.canonicalize().unwrap()
        );
        assert_eq!(
            fs::read_to_string(first.join("SKILL.md")).unwrap(),
            "updated in bundle"
        );
        assert_eq!(
            fs::read_to_string(sibling.join("SKILL.md")).unwrap(),
            "other-skill"
        );
    }

    #[test]
    fn preserves_real_installations_and_rejects_symlinked_parent_directories() {
        let base = tempfile::tempdir().unwrap();
        let resources = tempfile::tempdir().unwrap();
        let source = skill(resources.path(), "source");
        let outside = tempfile::tempdir().unwrap();
        directory_symlink(outside.path(), &base.path().join(".agents")).unwrap();
        assert!(install(base.path(), &source).is_err());
        assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
        fs::remove_file(base.path().join(".agents")).unwrap();
        let target = install(base.path(), &source).unwrap();
        fs::remove_file(&target).unwrap();
        fs::create_dir(&target).unwrap();
        fs::write(target.join("SKILL.md"), "user version").unwrap();
        assert!(install(base.path(), &source).is_err());
        assert_eq!(
            fs::read_to_string(target.join("SKILL.md")).unwrap(),
            "user version"
        );
    }

    #[test]
    fn repairs_dangling_links_but_missing_resources_leave_existing_links_unchanged() {
        let base = tempfile::tempdir().unwrap();
        let resources = tempfile::tempdir().unwrap();
        let source = skill(resources.path(), "source");
        let target = install(base.path(), &source).unwrap();
        let original_link = fs::read_link(&target).unwrap();
        fs::remove_dir_all(&source).unwrap();
        let second = skill(resources.path(), "second");
        assert!(install(base.path(), &source).is_err());
        assert_eq!(fs::read_link(&target).unwrap(), original_link);
        install(base.path(), &second).unwrap();
        assert_eq!(
            fs::read_to_string(target.join("SKILL.md")).unwrap(),
            "second"
        );
    }
}
