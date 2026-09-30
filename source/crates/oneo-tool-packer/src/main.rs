mod bundle;
mod cargo;
mod cli;
mod config;
mod windows;
#[cfg(windows)]
mod windows_env;

use anyhow::{Context, Result, ensure};
use std::process::Command;

fn main() {
    if let Err(error) = run() {
        eprintln!("oneo-tool-packer: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let Some(args) = cli::Args::parse(std::env::args_os().skip(1))? else {
        return Ok(());
    };
    let project = config::Project::load(&args.project)?;
    let host = cargo::host(&project.root)?;
    let target = cli::Target::resolve(args.target.as_deref(), &host)?;
    if args.dry_run {
        let command = cargo::build_command(&project, &target, None);
        if target.os == "windows" {
            println!(
                "Icon: {} -> temporary icon.res (Windows SDK rc)",
                project.root.join("icon.ico").display()
            );
        }
        println!("Project: {}", project.root.display());
        println!("Target: {}", target.triple);
        println!("Build: {command:?}");
        println!(
            "Output: {}",
            bundle::output_path(&project, &target).display()
        );
        return Ok(());
    }
    ensure!(
        target.os == cli::Target::resolve(None, &host)?.os,
        "target {} requires a {} build host and its SDK; current host is {host}",
        target.triple,
        target.os
    );
    #[cfg(windows)]
    windows_env::prepare(&target)?;
    bundle::preflight(&project, &target)?;
    let icon_resources = if target.os == "windows" {
        Some(windows::compile_icon(&project)?)
    } else {
        None
    };
    let icon_path = icon_resources
        .as_ref()
        .map(|directory| directory.path().join("icon.res"));
    let command = cargo::build_command(&project, &target, icon_path.as_deref());
    let executable = cargo::build(command, &project)?;
    let output = bundle::create(
        &project,
        &target,
        &executable,
        args.codesign_identity.as_deref(),
    )?;
    println!("{}", output.display());
    Ok(())
}

pub(crate) fn checked(command: &mut Command) -> Result<()> {
    let status = command
        .status()
        .with_context(|| format!("could not start {command:?}"))?;
    ensure!(status.success(), "command failed ({status}): {command:?}");
    Ok(())
}

pub(crate) fn captured(command: &mut Command) -> Result<Vec<u8>> {
    let output = command
        .output()
        .with_context(|| format!("could not start {command:?}"))?;
    ensure!(
        output.status.success(),
        "command failed ({}): {command:?}\n{}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(output.stdout)
}
