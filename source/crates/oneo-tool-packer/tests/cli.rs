#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Output},
};

struct Fixture {
    root: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::Builder::new()
            .prefix("packer project with spaces ")
            .tempdir()
            .unwrap();
        let path = root.path();
        fs::create_dir(path.join("tools")).unwrap();
        fs::create_dir_all(path.join("windows/System32")).unwrap();
        fs::write(path.join("windows/System32/KERNEL32.dll"), "system").unwrap();
        fs::write(path.join("host"), "host: x86_64-pc-windows-msvc\n").unwrap();
        fs::write(
            path.join("Cargo.toml"),
            "[package]\nname='app'\nversion='1.2.3'\nedition='2024'\n",
        )
        .unwrap();
        fs::write(path.join("packer.toml"), "package='app'\nbin='app'\nname='Test App'\nidentifier='me.test.app'\n[windows]\nresources=['readme.txt']\n").unwrap();
        fs::write(path.join("readme.txt"), "packaged resource").unwrap();
        fs::write(path.join("icon.ico"), "fixture icon").unwrap();
        script(
            &path.join("tools/rc"),
            r#"#!/bin/sh
cmp icon.ico "$PACKER_FIXTURE_ROOT/icon.ico" || exit 8
printf 'compiled icon resource' > icon.res
cp icon.rc "$PACKER_FIXTURE_ROOT/compiled-icon.rc"
"#,
        );
        let mut pe = vec![0; 70];
        pe[..2].copy_from_slice(b"MZ");
        pe[60..64].copy_from_slice(&64u32.to_le_bytes());
        pe[64..68].copy_from_slice(b"PE\0\0");
        pe[68..70].copy_from_slice(&0x8664u16.to_le_bytes());
        fs::write(path.join("app.exe"), &pe).unwrap();
        fs::write(path.join("helper.dll"), pe).unwrap();
        fs::write(path.join("metadata.json"), serde_json::to_vec(&serde_json::json!({
            "packages": [{"id":"app-id", "name":"app", "version":"1.2.3", "targets":[{"name":"app", "kind":["bin"]}]}],
            "workspace_members":["app-id"], "target_directory":path.join("custom target")
        })).unwrap()).unwrap();
        fs::write(path.join("artifact.json"), serde_json::to_vec(&serde_json::json!({
            "reason":"compiler-artifact", "package_id":"app-id", "target":{"name":"app"}, "executable":path.join("app.exe")
        })).unwrap()).unwrap();
        script(
            &path.join("tools/cargo"),
            r#"#!/bin/sh
case "$1" in
 metadata) cat "$PACKER_FIXTURE_ROOT/metadata.json" ;;
 rustc)
  printf '%s\n' "$@" > "$PACKER_FIXTURE_ROOT/build-args"
  test ! -e "$PACKER_FIXTURE_ROOT/fail-build" || exit 9
  cat "$PACKER_FIXTURE_ROOT/artifact.json"
  ;;
 *) exit 8 ;;
esac
"#,
        );
        script(
            &path.join("tools/rustc"),
            "#!/bin/sh\ncat \"$PACKER_FIXTURE_ROOT/host\"\n",
        );
        script(
            &path.join("tools/dumpbin"),
            r#"#!/bin/sh
test "$1" != /help || exit 0
if test "$3" = "$PACKER_FIXTURE_ROOT/app.exe"; then
 printf '  KERNEL32.dll\n  helper.dll\n'
else
 printf '  KERNEL32.dll\n'
fi
"#,
        );
        Self { root }
    }

    fn run(&self, args: &[&str]) -> Output {
        let mut paths = vec![self.root.path().join("tools")];
        paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
        Command::new(env!("CARGO_BIN_EXE_oneo-tool-packer"))
            .args(["--project"])
            .arg(self.root.path())
            .args(args)
            .env("PATH", std::env::join_paths(paths).unwrap())
            .env("PACKER_FIXTURE_ROOT", self.root.path())
            .env("SystemRoot", self.root.path().join("windows"))
            .env_remove("VCToolsRedistDir")
            .output()
            .unwrap()
    }
}

fn script(path: &Path, source: &str) {
    fs::write(path, source).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn windows_cli_builds_portable_directory_under_dist() {
    let fixture = Fixture::new();
    let result = fixture.run(&[]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let output = fixture.root.path().join("dist/Test App");
    assert!(output.join("app.exe").is_file());
    assert!(output.join("helper.dll").is_file());
    assert!(!output.join("KERNEL32.dll").exists());
    assert_eq!(
        fs::read_to_string(output.join("resources/readme.txt")).unwrap(),
        "packaged resource"
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains(output.to_str().unwrap()));
    let args = fs::read_to_string(fixture.root.path().join("build-args")).unwrap();
    assert!(args.contains("--target\nx86_64-pc-windows-msvc\n"));
    assert!(!args.contains("no_exported_symbols"));
    assert!(args.lines().any(|arg| arg == "--release"));
    assert!(
        args.lines()
            .any(|arg| arg.starts_with("link-arg=") && arg.ends_with("icon.res"))
    );
    assert_eq!(
        fs::read_to_string(fixture.root.path().join("compiled-icon.rc")).unwrap(),
        "1 ICON \"icon.ico\"\n"
    );
    assert!(!fixture.root.path().join("custom target").exists());
    fs::write(output.join("stale"), "old").unwrap();
    fs::write(fixture.root.path().join("fail-build"), "").unwrap();
    assert!(!fixture.run(&[]).status.success());
    assert!(output.join("stale").exists());
}

#[test]
fn missing_runtime_keeps_previous_output_and_returns_failure() {
    let fixture = Fixture::new();
    assert!(fixture.run(&[]).status.success());
    fs::remove_file(fixture.root.path().join("helper.dll")).unwrap();
    let result = fixture.run(&[]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("missing runtime helper.dll"));
    assert!(
        fixture
            .root
            .path()
            .join("dist/Test App/helper.dll")
            .is_file()
    );
}

#[test]
fn cross_system_target_can_be_previewed_but_fails_before_building() {
    let fixture = Fixture::new();
    fs::write(
        fixture.root.path().join("host"),
        "host: aarch64-apple-darwin\n",
    )
    .unwrap();
    let result = fixture.run(&["--target", "windows", "--dry-run"]);
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("aarch64-pc-windows-msvc"));
    assert!(!fixture.root.path().join("build-args").exists());
    let result = fixture.run(&["--target", "windows"]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("requires a windows build host"));
    assert!(!fixture.root.path().join("build-args").exists());
}

#[test]
fn missing_icon_and_resource_compiler_failure_stop_before_cargo_build() {
    let fixture = Fixture::new();
    fs::remove_file(fixture.root.path().join("icon.ico")).unwrap();
    let result = fixture.run(&[]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("missing Windows icon"));
    assert!(!fixture.root.path().join("build-args").exists());
    fs::write(fixture.root.path().join("icon.ico"), "fixture icon").unwrap();
    script(&fixture.root.path().join("tools/rc"), "#!/bin/sh\nexit 7\n");
    let result = fixture.run(&[]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("compile icon.ico"));
    assert!(!fixture.root.path().join("build-args").exists());
}
