use crate::{
    cli::Target,
    config::{Project, Resource},
};
use anyhow::{Context, Result, bail, ensure};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

const MACOS_ICON: &str = "icon.icon";
const MACOS_MINIMUM_VERSION: &str = "13.0";

pub fn output_path(project: &Project, target: &Target) -> PathBuf {
    let name = project.config.name.clone();
    project
        .root
        .join(".build")
        .join(&target.triple)
        .join(if target.os == "macos" {
            format!("{name}.app")
        } else {
            name
        })
}

pub fn preflight(project: &Project, target: &Target) -> Result<()> {
    if target.os == "macos" {
        ensure!(
            project.root.join(MACOS_ICON).is_dir(),
            "missing macOS icon directory {}",
            project.root.join(MACOS_ICON).display()
        );
        crate::captured(Command::new("xcrun").args(["--find", "actool"]))?;
    } else {
        ensure!(
            project.root.join("icon.ico").is_file(),
            "missing Windows icon {}",
            project.root.join("icon.ico").display()
        );
        crate::captured(
            Command::new("dumpbin")
                .arg("/headers")
                .arg(std::env::current_exe()?),
        )
        .context(
            "Windows packaging requires dumpbin in the Visual Studio C++ developer environment",
        )?;
    }
    Ok(())
}

pub fn create(
    project: &Project,
    target: &Target,
    executable: &Path,
    identity: Option<&str>,
) -> Result<PathBuf> {
    let output = output_path(project, target);
    let parent = output.parent().context("bundle parent directory")?;
    fs::create_dir_all(parent)?;
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(parent.join(".lock"))?;
    lock.try_lock()
        .context("another packer is publishing to this output directory")?;
    let staging = tempfile::Builder::new()
        .prefix(".packing-")
        .tempdir_in(parent)?;
    let bundle = staging
        .path()
        .join(output.file_name().context("output name")?);
    fs::create_dir(&bundle)?;
    if target.os == "macos" {
        macos(project, executable, identity, &bundle, staging.path())?;
    } else {
        fs::copy(
            executable,
            bundle.join(format!("{}.exe", project.config.bin)),
        )?;
        copy_platform_resources(project, "windows", &bundle.join("resources"))?;
        crate::windows::copy_dependencies(project, target, executable, &bundle)?;
    }
    publish(&bundle, &output, staging)?;
    Ok(output)
}

fn macos(
    project: &Project,
    executable: &Path,
    identity: Option<&str>,
    bundle: &Path,
    staging: &Path,
) -> Result<()> {
    let name = project.config.name.clone();
    let identifier = project.config.identifier.clone();
    let contents = bundle.join("Contents");
    let resources = contents.join("Resources");
    fs::create_dir_all(contents.join("MacOS"))?;
    fs::create_dir(&resources)?;
    fs::copy(executable, contents.join("MacOS").join(&name))?;
    let partial = staging.join("icon.plist");
    let icon = project.root.join(MACOS_ICON);
    let compiled_icons = staging.join("compiled-icons");
    fs::create_dir(&compiled_icons)?;
    let log = fs::File::create(staging.join("actool.log"))?;
    let result = crate::checked(
        Command::new("xcrun")
            .arg("actool")
            .arg("--compile")
            .arg(&compiled_icons)
            .args([
                "--platform",
                "macosx",
                "--minimum-deployment-target",
                MACOS_MINIMUM_VERSION,
                "--app-icon",
                "icon",
                "--output-partial-info-plist",
            ])
            .arg(&partial)
            .arg(icon)
            .stdout(Stdio::from(log)),
    );
    if let Err(error) = result {
        eprintln!(
            "{}",
            fs::read_to_string(staging.join("actool.log")).unwrap_or_default()
        );
        return Err(error);
    }
    for entry in fs::read_dir(&compiled_icons)? {
        copy_resources(
            &compiled_icons,
            &[Resource::Path(entry?.path())],
            &resources,
        )?;
    }
    copy_platform_resources(project, "macos", &resources)?;
    let mut info = plist::Value::from_file(&partial)?
        .into_dictionary()
        .context("actool produced invalid metadata")?;
    info.extend(macos_info(project));
    plist::Value::Dictionary(info).to_file_xml(contents.join("Info.plist"))?;
    crate::checked(Command::new("xattr").arg("-cr").arg(bundle))?;
    let identity = identity
        .map(str::to_owned)
        .or_else(|| std::env::var("ONEO_PACKER_CODESIGN_IDENTITY").ok())
        .unwrap_or_else(|| "-".into());
    ensure!(!identity.is_empty(), "codesign identity must not be empty");
    let mut sign = Command::new("codesign");
    sign.args(["--force", "--identifier", &identifier]);
    if identity != "-" {
        sign.args(["--options", "runtime", "--timestamp"]);
    }
    crate::checked(sign.arg("--sign").arg(identity).arg(bundle))?;
    crate::checked(
        Command::new("codesign")
            .args(["--verify", "--deep", "--strict"])
            .arg(bundle),
    )?;
    Ok(())
}

fn macos_info(project: &Project) -> plist::Dictionary {
    let mut info = plist::Dictionary::new();
    let name = project.config.name.clone();
    let identifier = project.config.identifier.clone();
    for (key, value) in [
        ("CFBundleDisplayName", name.as_str()),
        ("CFBundleExecutable", name.as_str()),
        ("CFBundleName", name.as_str()),
        ("CFBundleIdentifier", identifier.as_str()),
        ("CFBundleShortVersionString", project.version.as_str()),
        ("CFBundleVersion", project.version.as_str()),
        ("CFBundleInfoDictionaryVersion", "6.0"),
        ("CFBundlePackageType", "APPL"),
        ("LSMinimumSystemVersion", MACOS_MINIMUM_VERSION),
        ("NSPrincipalClass", "NSApplication"),
    ] {
        info.insert(key.into(), plist::Value::String(value.into()));
    }
    info.insert(
        "NSHighResolutionCapable".into(),
        plist::Value::Boolean(true),
    );
    info
}

fn publish(bundle: &Path, output: &Path, staging: tempfile::TempDir) -> Result<()> {
    let previous = staging.path().join("previous");
    let existed = match fs::symlink_metadata(output) {
        Ok(metadata) => {
            ensure!(
                metadata.is_dir() && !metadata.is_symlink(),
                "refusing to replace non-directory {}",
                output.display()
            );
            true
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => return Err(error.into()),
    };
    if existed {
        fs::rename(output, &previous)
            .context("move previous bundle; close applications using this directory")?;
    }
    if let Err(error) = fs::rename(bundle, output) {
        if existed && let Err(restore) = fs::rename(&previous, output) {
            let saved = staging.keep();
            bail!(
                "publish failed: {error}; restore failed: {restore}; previous bundle preserved in {}",
                saved.join("previous").display()
            );
        }
        return Err(error).context("publish bundle");
    }
    Ok(())
}

fn copy_platform_resources(project: &Project, os: &str, destination: &Path) -> Result<()> {
    copy_resources(&project.root, &project.config.resources, destination)?;
    let resources = if os == "macos" {
        &project.config.macos.resources
    } else {
        &project.config.windows.resources
    };
    copy_resources(&project.root, resources, destination)
}

pub fn copy_resources(root: &Path, paths: &[Resource], destination: &Path) -> Result<()> {
    for path in paths {
        let (source, name) = match path {
            Resource::Path(path) => {
                let name = path.file_name().context("resource must have a filename")?;
                (
                    root.join(path),
                    name.to_str().context("resource filename must be UTF-8")?,
                )
            }
            Resource::Mapped {
                source,
                destination,
            } => (root.join(source), destination.as_str()),
        };
        crate::config::component(name)?;
        fs::create_dir_all(destination)?;
        copy_tree(&source, &destination.join(name))?;
    }
    Ok(())
}

fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    let metadata =
        fs::symlink_metadata(source).with_context(|| format!("resource {}", source.display()))?;
    ensure!(
        !metadata.is_symlink(),
        "resource symlinks are unsupported: {}",
        source.display()
    );
    ensure!(
        !destination.try_exists()?,
        "resource destination already exists: {}",
        destination.display()
    );
    if metadata.is_dir() {
        fs::create_dir(destination)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            crate::config::component(
                entry
                    .file_name()
                    .to_str()
                    .context("resource filename must be UTF-8")?,
            )?;
            copy_tree(&entry.path(), &destination.join(entry.file_name()))?;
        }
    } else {
        ensure!(
            metadata.is_file(),
            "resource is not a regular file: {}",
            source.display()
        );
        fs::copy(source, destination)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generates_release_macos_metadata_from_project() {
        let project = Project {
            root: "/no/plist/required".into(),
            config: toml::from_str(
                "package='app'\nbin='app'\nname='Test & App'\nidentifier='me.test.app'",
            )
            .unwrap(),
            version: "1.2.3".into(),
            package_id: "app".into(),
        };
        let info = macos_info(&project);
        let name = "Test & App";
        let identifier = "me.test.app";
        assert_eq!(info["CFBundleExecutable"].as_string(), Some(name));
        assert_eq!(info["CFBundleIdentifier"].as_string(), Some(identifier));
        assert_eq!(
            info["CFBundleShortVersionString"].as_string(),
            Some("1.2.3")
        );
        assert_eq!(info["LSMinimumSystemVersion"].as_string(), Some("13.0"));
        let value = plist::Value::Dictionary(info);
        let mut xml = Vec::new();
        value.to_writer_xml(&mut xml).unwrap();
        assert_eq!(
            plist::Value::from_reader(std::io::Cursor::new(xml)).unwrap(),
            value
        );
    }
    #[test]
    fn replaces_complete_bundle_without_retaining_stale_files() {
        let root = tempfile::tempdir().unwrap();
        let output = root.path().join("App");
        fs::create_dir(&output).unwrap();
        fs::write(output.join("stale"), "old").unwrap();
        let staging = tempfile::tempdir_in(root.path()).unwrap();
        let next = staging.path().join("App");
        fs::create_dir(&next).unwrap();
        fs::write(next.join("app.exe"), "new").unwrap();
        publish(&next, &output, staging).unwrap();
        assert!(!output.join("stale").exists());
        assert_eq!(fs::read_to_string(output.join("app.exe")).unwrap(), "new");
    }
    #[test]
    fn failed_publication_restores_previous_bundle() {
        let root = tempfile::tempdir().unwrap();
        let output = root.path().join("App");
        fs::create_dir(&output).unwrap();
        fs::write(output.join("app.exe"), "old").unwrap();
        let staging = tempfile::tempdir_in(root.path()).unwrap();
        assert!(publish(&staging.path().join("missing"), &output, staging).is_err());
        assert_eq!(fs::read_to_string(output.join("app.exe")).unwrap(), "old");
    }
    #[test]
    fn copies_external_template_directories_for_both_platforms() {
        let root = tempfile::tempdir().unwrap();
        let template = root.path().join("templates/手机主题/桌面");
        fs::create_dir_all(&template).unwrap();
        fs::write(template.join("壁纸.svg"), "<svg/>").unwrap();
        let config: crate::config::Config = toml::from_str(
            r#"
package = "app"
bin = "app"
name = "App"
identifier = "me.test.app"
resources = ["templates"]
"#,
        )
        .unwrap();
        for output in ["App.app/Contents/Resources", "App/resources"] {
            let destination = root.path().join(output);
            copy_resources(root.path(), &config.resources, &destination).unwrap();
            assert_eq!(
                fs::read_to_string(destination.join("templates/手机主题/桌面/壁纸.svg")).unwrap(),
                "<svg/>"
            );
        }
    }

    #[test]
    fn locus_bundle_contains_the_builtin_documentation() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let config: crate::config::Config =
            toml::from_str(&fs::read_to_string(root.join("packer.toml")).unwrap()).unwrap();
        let output = tempfile::tempdir().unwrap();
        for platform in ["Locus.app/Contents/Resources", "Locus/resources"] {
            let destination = output.path().join(platform);
            copy_resources(&root, &config.resources, &destination).unwrap();
            for name in ["attachments.md", "canvas.md"] {
                assert_eq!(
                    fs::read(destination.join("docs").join(name)).unwrap(),
                    fs::read(root.join("resources/docs").join(name)).unwrap()
                );
            }
        }
    }

    #[test]
    fn bundles_only_resources_for_the_selected_platform() {
        let root = tempfile::tempdir().unwrap();
        for (path, content) in [
            ("templates/theme.xml", "template"),
            ("tools/macos/aapt2", "mac"),
            ("tools/windows/aapt2.exe", "windows"),
        ] {
            let path = root.path().join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, content).unwrap();
        }
        let project = Project {
            root: root.path().into(),
            config: toml::from_str(
                r#"
package = "app"
bin = "app"
name = "App"
identifier = "me.test.app"
resources = ["templates"]
[macos]
resources = [{ source = "tools/macos", destination = "tools" }]
[windows]
resources = [{ source = "tools/windows", destination = "tools" }]
"#,
            )
            .unwrap(),
            version: "1.0.0".into(),
            package_id: "app".into(),
        };
        for (os, other, binary) in [
            ("macos", "windows", "aapt2"),
            ("windows", "macos", "aapt2.exe"),
        ] {
            let output = root.path().join("output").join(os);
            copy_platform_resources(&project, os, &output).unwrap();
            assert!(output.join("templates/theme.xml").is_file());
            assert!(output.join("tools").join(binary).is_file());
            let other_binary = if os == "macos" { "aapt2.exe" } else { "aapt2" };
            assert!(!output.join("tools").join(other_binary).exists());
            assert!(!output.join(os).exists());
            assert!(!output.join(other).exists());
            assert_eq!(fs::read_dir(&output).unwrap().count(), 2);
        }
    }

    #[test]
    fn mapped_resources_reject_escaping_names_and_collisions() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("source"), "data").unwrap();
        let output = root.path().join("output");
        for destination in ["../escape", "nested/tools", "NUL"] {
            let resource = Resource::Mapped {
                source: "source".into(),
                destination: destination.into(),
            };
            assert!(copy_resources(root.path(), &[resource], &output).is_err());
        }
        let resources = [
            Resource::Mapped {
                source: "source".into(),
                destination: "tools".into(),
            },
            Resource::Mapped {
                source: "source".into(),
                destination: "tools".into(),
            },
        ];
        assert!(copy_resources(root.path(), &resources, &output).is_err());
        assert_eq!(fs::read_to_string(output.join("tools")).unwrap(), "data");
    }

    #[test]
    fn resource_collision_fails_instead_of_overwriting() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("icon"), "image").unwrap();
        let out = root.path().join("out");
        assert!(
            copy_resources(
                root.path(),
                &[Resource::Path("icon".into()), Resource::Path("icon".into())],
                &out
            )
            .is_err()
        );
        assert_eq!(fs::read_to_string(out.join("icon")).unwrap(), "image");
    }
}
