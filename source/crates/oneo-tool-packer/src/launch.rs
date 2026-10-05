use crate::{bundle, cli::Target, config::Project};
use anyhow::Result;
use std::{ffi::OsString, process::Command};

pub fn command(project: &Project, target: &Target, args: &[OsString]) -> Command {
    let output = bundle::output_path(project, target);
    let executable = if target.os == "macos" {
        output.join("Contents/MacOS").join(&project.config.name)
    } else {
        output.join(format!("{}.exe", project.config.bin))
    };
    let mut command = Command::new(executable);
    command.current_dir(&project.root).args(args);
    command
}

pub fn run(mut command: Command) -> Result<()> {
    eprintln!("Running {command:?}");
    // Replacing the process preserves terminal signals and the app's exit status.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        Err(command.exec().into())
    }
    #[cfg(not(unix))]
    {
        let status = command.status()?;
        std::process::exit(status.code().unwrap_or(1));
    }
}
