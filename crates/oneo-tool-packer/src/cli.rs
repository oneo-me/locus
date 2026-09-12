use anyhow::{Result, bail, ensure};
use std::{ffi::OsString, path::PathBuf};

const HELP: &str = "oneo-tool-packer [--project DIR] [--target TARGET] [--dry-run]

Build the release application configured in packer.toml.
  --project DIR               Project directory (default: current directory)
  --target TARGET             macos, windows, or a supported Rust target triple
                              Default: rustc host, not Cargo build.target
  --codesign-identity ID       macOS identity; alternatively ONEO_PACKER_CODESIGN_IDENTITY
                              Default: ad-hoc signing
  --dry-run                   Print build command and destination without building
  -h, --help                  Show this help
  -V, --version               Show version

Targets: aarch64-apple-darwin, x86_64-apple-darwin,
         x86_64-pc-windows-msvc, aarch64-pc-windows-msvc.
macOS bundles require macOS and Xcode. Windows directories require Windows
and Visual Studio C++ tools with Windows SDK (environment loaded automatically).
No installer is generated.";

#[derive(Debug, Default)]
pub struct Args {
    pub project: PathBuf,
    pub target: Option<String>,
    pub codesign_identity: Option<String>,
    pub dry_run: bool,
}

impl Args {
    pub fn parse(arguments: impl IntoIterator<Item = OsString>) -> Result<Option<Self>> {
        let mut args = Self {
            project: PathBuf::from("."),
            ..Self::default()
        };
        let mut arguments = arguments.into_iter();
        while let Some(argument) = arguments.next() {
            match argument.to_str() {
                Some("--dry-run") => args.dry_run = true,
                Some("--project") => {
                    args.project = arguments
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("--project requires a directory"))?
                        .into()
                }
                Some("--target") => args.target = Some(value(&mut arguments, "--target")?),
                Some("--codesign-identity") => {
                    args.codesign_identity = Some(value(&mut arguments, "--codesign-identity")?)
                }
                Some("--help" | "-h") => {
                    println!("{HELP}");
                    return Ok(None);
                }
                Some("--version" | "-V") => {
                    println!("oneo-tool-packer {}", env!("CARGO_PKG_VERSION"));
                    return Ok(None);
                }
                _ => bail!("unknown argument {argument:?}; use --help"),
            }
        }
        Ok(Some(args))
    }
}

fn value(arguments: &mut impl Iterator<Item = OsString>, flag: &str) -> Result<String> {
    let value = arguments
        .next()
        .ok_or_else(|| anyhow::anyhow!("{flag} requires a value"))?;
    let value = value
        .into_string()
        .map_err(|_| anyhow::anyhow!("{flag} requires UTF-8 text"))?;
    ensure!(
        !value.is_empty() && !value.starts_with("--"),
        "{flag} requires a value"
    );
    Ok(value)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub triple: String,
    pub os: &'static str,
    pub arch: &'static str,
}

impl Target {
    pub fn resolve(requested: Option<&str>, host: &str) -> Result<Self> {
        let arch = host.split('-').next().unwrap_or("");
        let triple = match requested {
            Some("macos") => format!("{arch}-apple-darwin"),
            Some("windows") => format!("{arch}-pc-windows-msvc"),
            Some(triple) => triple.into(),
            None => host.into(),
        };
        let (os, arch) = match triple.as_str() {
            "aarch64-apple-darwin" => ("macos", "aarch64"),
            "x86_64-apple-darwin" => ("macos", "x86_64"),
            "aarch64-pc-windows-msvc" => ("windows", "aarch64"),
            "x86_64-pc-windows-msvc" => ("windows", "x86_64"),
            _ => bail!("unsupported target {triple}; use --help for supported targets"),
        };
        Ok(Self { triple, os, arch })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn target_alias_preserves_host_architecture() {
        assert_eq!(
            Target::resolve(Some("windows"), "aarch64-apple-darwin")
                .unwrap()
                .triple,
            "aarch64-pc-windows-msvc"
        );
        assert!(Target::resolve(Some("x86_64-pc-windows-gnu"), "aarch64-apple-darwin").is_err());
    }
    #[test]
    fn parses_paths_with_spaces_and_rejects_missing_values() {
        let args = Args::parse(["--project", "/tmp/my project"].map(Into::into))
            .unwrap()
            .unwrap();
        assert_eq!(args.project, PathBuf::from("/tmp/my project"));
        assert!(Args::parse(["--release"].map(Into::into)).is_err());
        assert!(Args::parse(["--target", "--release"].map(Into::into)).is_err());
    }
}
