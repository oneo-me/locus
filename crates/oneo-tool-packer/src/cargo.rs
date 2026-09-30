use crate::{cli::Target, config::Project};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::{
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[derive(Deserialize)]
pub struct Metadata {
    pub packages: Vec<Package>,
    pub workspace_members: Vec<String>,
}
#[derive(Deserialize)]
pub struct Package {
    pub id: String,
    pub name: String,
    pub version: String,
    pub targets: Vec<CargoTarget>,
}
#[derive(Deserialize)]
pub struct CargoTarget {
    pub name: String,
    pub kind: Vec<String>,
}

pub fn metadata(root: &Path) -> Result<Metadata> {
    let output = crate::captured(
        Command::new("cargo")
            .current_dir(root)
            .args(["metadata", "--no-deps", "--format-version", "1"])
            .arg("--manifest-path")
            .arg(root.join("Cargo.toml")),
    )?;
    serde_json::from_slice(&output).context("parse Cargo metadata")
}

pub fn host(root: &Path) -> Result<String> {
    let output = crate::captured(Command::new("rustc").current_dir(root).arg("-vV"))?;
    String::from_utf8(output)?
        .lines()
        .find_map(|line| line.strip_prefix("host: ").map(str::to_owned))
        .context("rustc did not report its host")
}

pub fn build_command(project: &Project, target: &Target, icon_resource: Option<&Path>) -> Command {
    let mut command = Command::new("cargo");
    command
        .current_dir(&project.root)
        .args([
            "rustc",
            "--locked",
            "--message-format=json-render-diagnostics",
            "--manifest-path",
        ])
        .arg(project.root.join("Cargo.toml"))
        .args([
            "--package",
            &project.config.package,
            "--bin",
            &project.config.bin,
            "--target",
            &target.triple,
        ]);
    command.arg("--release");
    if target.os == "macos" {
        command.args(["--", "-C", "link-arg=-Wl,-no_exported_symbols"]);
    } else {
        // Apply to the full target dependency graph, including native build
        // scripts, rather than only the final binary's rustc arguments.
        command.arg("--config").arg(format!(
            "target.{}.rustflags=[\"-C\",\"target-feature=+crt-static\"]",
            target.triple
        ));
        let mut argument = std::ffi::OsString::from("link-arg=");
        argument.push(icon_resource.unwrap_or(Path::new("<temporary-icon.res>")));
        command.args(["--", "-C"]).arg(argument);
    }
    command
}

pub fn build(mut command: Command, project: &Project) -> Result<PathBuf> {
    eprintln!("Building {}", project.config.package);
    let mut child = command
        .stdout(Stdio::piped())
        .spawn()
        .context("start Cargo build")?;
    let reader = BufReader::new(child.stdout.take().context("read Cargo output")?);
    let executable = match read_artifacts(reader, project) {
        Ok(executable) => executable,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
    };
    let status = child.wait()?;
    ensure!(status.success(), "Cargo build failed ({status})");
    executable.context("Cargo did not report the selected executable")
}

fn read_artifacts(reader: impl BufRead, project: &Project) -> Result<Option<PathBuf>> {
    let mut executable = None;
    for line in reader.lines() {
        let line = line?;
        let message: serde_json::Value =
            serde_json::from_str(&line).context("parse Cargo build output")?;
        if message["reason"] == "compiler-artifact"
            && message["package_id"] == project.package_id
            && message["target"]["name"] == project.config.bin
            && let Some(path) = message["executable"].as_str()
        {
            executable = Some(PathBuf::from(path));
        }
    }
    Ok(executable)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn project() -> Project {
        Project {
            root: "/tmp/project with spaces".into(),
            config: toml::from_str("package='app'\nbin='app'\nname='App'\nidentifier='me.app'")
                .unwrap(),
            version: "0.1.0".into(),
            package_id: "path+file:///tmp/app#0.1.0".into(),
        }
    }
    #[test]
    fn always_builds_release_and_limits_export_flag_to_macos_main_binary() {
        for triple in ["aarch64-apple-darwin", "x86_64-pc-windows-msvc"] {
            let args: Vec<_> = build_command(
                &project(),
                &Target::resolve(Some(triple), "").unwrap(),
                None,
            )
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
            assert!(args.iter().any(|a| a == "--release"));
            assert_eq!(
                args.iter().any(|a| a.contains("no_exported_symbols")),
                triple.ends_with("darwin")
            );
            assert!(args.windows(2).any(|a| a == ["--bin", "app"]));
        }
    }
    #[test]
    fn windows_static_crt_applies_to_dependencies_before_rustc_separator() {
        for triple in [
            "x86_64-pc-windows-msvc",
            "aarch64-pc-windows-msvc",
            "aarch64-apple-darwin",
        ] {
            let command = build_command(
                &project(),
                &Target::resolve(Some(triple), "").unwrap(),
                None,
            );
            let args: Vec<_> = command
                .get_args()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect();
            let separator = args.iter().position(|arg| arg == "--").unwrap();
            let config =
                format!("target.{triple}.rustflags=[\"-C\",\"target-feature=+crt-static\"]");
            assert_eq!(
                args[..separator]
                    .windows(2)
                    .any(|pair| pair == ["--config", config.as_str()]),
                triple.ends_with("msvc")
            );
            assert!(
                !args[separator..]
                    .iter()
                    .any(|arg| arg.contains("crt-static"))
            );
        }
    }

    #[test]
    fn selects_only_the_requested_package_artifact() {
        let input = b"{\"reason\":\"compiler-artifact\",\"package_id\":\"other\",\"target\":{\"name\":\"app\"},\"executable\":\"/wrong\"}\n{\"reason\":\"compiler-artifact\",\"package_id\":\"path+file:///tmp/app#0.1.0\",\"target\":{\"name\":\"app\"},\"executable\":\"/custom target/app\"}\n";
        assert_eq!(
            read_artifacts(&input[..], &project()).unwrap().unwrap(),
            PathBuf::from("/custom target/app")
        );
    }

    #[test]
    fn builds_with_cargo_resolved_target_directories() {
        for source in ["global", "project", "environment"] {
            let directory = tempfile::Builder::new()
                .prefix("packer shared target ")
                .tempdir()
                .unwrap();
            let root = directory.path().join("project");
            let cargo_home = directory.path().join(".cargo");
            std::fs::create_dir_all(&root).unwrap();
            std::fs::create_dir_all(&cargo_home).unwrap();
            let root = dunce::canonicalize(root).unwrap();
            std::fs::write(
                root.join("Cargo.toml"),
                "[package]\nname='app'\nversion='0.1.0'\nedition='2024'\n[workspace]\n[[bin]]\nname='app'\npath='main.rs'\n",
            )
            .unwrap();
            std::fs::write(root.join("main.rs"), "fn main() {}\n").unwrap();
            let target_directory = match source {
                "global" => {
                    std::fs::write(
                        cargo_home.join("config.toml"),
                        "[build]\ntarget-dir='.cargo/shared target'\n",
                    )
                    .unwrap();
                    cargo_home.join("shared target")
                }
                "project" => {
                    std::fs::create_dir(root.join(".cargo")).unwrap();
                    std::fs::write(
                        root.join(".cargo/config.toml"),
                        "[build]\ntarget-dir='../shared target'\n",
                    )
                    .unwrap();
                    directory.path().join("shared target")
                }
                _ => directory.path().join("environment target"),
            };
            let configure = |command: &mut Command| {
                command
                    .env("CARGO_HOME", &cargo_home)
                    .env_remove("CARGO_TARGET_DIR")
                    .env_remove("CARGO_BUILD_TARGET_DIR");
                if source == "environment" {
                    command.env("CARGO_TARGET_DIR", &target_directory);
                }
            };
            let mut metadata_command = Command::new("cargo");
            metadata_command.current_dir(&root).args([
                "metadata",
                "--offline",
                "--no-deps",
                "--format-version",
                "1",
            ]);
            configure(&mut metadata_command);
            let metadata: Metadata =
                serde_json::from_slice(&crate::captured(&mut metadata_command).unwrap()).unwrap();
            let mut project = project();
            project.root = root.clone();
            project.package_id = metadata.packages[0].id.clone();
            let mut lock = Command::new("cargo");
            lock.current_dir(&root)
                .args(["generate-lockfile", "--offline"]);
            configure(&mut lock);
            crate::captured(&mut lock).unwrap();
            let target = Target::resolve(None, &host(&root).unwrap()).unwrap();
            let mut command = build_command(&project, &target, None);
            // This test exercises Cargo's path resolution without platform SDKs.
            // Platform link flags are covered by the command tests above.
            if target.os == "windows" {
                command = Command::new("cargo");
                command.current_dir(&root).args([
                    "rustc",
                    "--locked",
                    "--release",
                    "--message-format=json-render-diagnostics",
                    "--target",
                    &target.triple,
                ]);
            }
            command.env("CARGO_NET_OFFLINE", "true");
            configure(&mut command);
            let executable = build(command, &project).unwrap();
            let expected = target_directory.join(&target.triple).join("release").join(
                if target.os == "windows" {
                    "app.exe"
                } else {
                    "app"
                },
            );
            assert_eq!(
                dunce::canonicalize(&executable).unwrap(),
                dunce::canonicalize(&expected).unwrap(),
                "{source} target-dir"
            );
            assert!(!root.join("target").exists(), "{source} target-dir");
            assert_eq!(
                crate::bundle::output_path(&project, &target),
                root.join(".build")
                    .join(&target.triple)
                    .join(if target.os == "macos" {
                        "App.app"
                    } else {
                        "App"
                    })
            );
        }
    }
}
