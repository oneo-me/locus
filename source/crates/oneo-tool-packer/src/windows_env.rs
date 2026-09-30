use anyhow::{Context, Result, ensure};
use std::{
    ffi::OsString,
    os::windows::{ffi::OsStringExt, process::CommandExt},
    path::PathBuf,
    process::Command,
};

const INITIALIZED: &str = "ONEO_PACKER_VS_INITIALIZED";

/// Restart with a child-only environment so all build tools and DLL discovery
/// inherit the SDK settings without mutating process-global environment state.
pub fn prepare(target: &crate::cli::Target) -> Result<()> {
    if tools_available() {
        return Ok(());
    }
    ensure!(
        std::env::var_os(INITIALIZED).is_none(),
        "Visual Studio environment was loaded but dumpbin or Windows SDK rc is missing; install the Desktop development with C++ workload and Windows SDK"
    );
    let environment = discover(target.arch).context(
        "could not load Visual Studio C++ environment automatically; install Desktop development with C++ and Windows SDK, or run from a configured developer terminal",
    )?;
    eprintln!("oneo-tool-packer: loaded Visual Studio C++ environment");
    let status = Command::new(std::env::current_exe()?)
        .args(std::env::args_os().skip(1))
        .envs(environment)
        .env(INITIALIZED, "1")
        .status()
        .context("restart packer with Visual Studio environment")?;
    std::process::exit(status.code().unwrap_or(1));
}

fn tools_available() -> bool {
    ["dumpbin", "rc"].iter().all(|tool| {
        probe(tool)
            .output()
            .is_ok_and(|output| output.status.success())
    })
}

fn probe(tool: &str) -> Command {
    let mut command = Command::new(tool);
    if tool == "dumpbin" {
        command
            .arg("/headers")
            .arg(std::env::current_exe().expect("packer executable path"));
    } else {
        command.arg("/?");
    }
    command
}

fn discover(arch: &str) -> Result<Vec<(OsString, OsString)>> {
    let installer = std::env::var_os("ProgramFiles(x86)")
        .or_else(|| std::env::var_os("ProgramFiles"))
        .context("Program Files directory is unavailable")?;
    let vswhere = PathBuf::from(installer).join("Microsoft Visual Studio/Installer/vswhere.exe");
    let output = crate::captured(Command::new(vswhere).args([
        "-latest",
        "-products",
        "*",
        "-requires",
        "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
        "-property",
        "installationPath",
        "-utf8",
    ]))?;
    let installation = std::str::from_utf8(&output)?
        .trim()
        .trim_start_matches('\u{feff}');
    ensure!(
        !installation.is_empty(),
        "no Visual Studio installation with C++ tools found"
    );
    let script = PathBuf::from(installation).join("Common7/Tools/VsDevCmd.bat");
    ensure!(script.is_file(), "missing {}", script.display());
    let arch = if arch == "aarch64" { "arm64" } else { "x64" };
    let host = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x64"
    };
    let mut command = Command::new(std::env::var_os("COMSPEC").unwrap_or_else(|| "cmd.exe".into()));
    command.env("ONEO_PACKER_VS_SCRIPT", script);
    // The command contains only fixed text and validated architecture names;
    // project paths and CLI arguments are passed directly to the child above.
    command.raw_arg(format!(r#"/d /u /s /c "call "%ONEO_PACKER_VS_SCRIPT%" -no_logo -arch={arch} -host_arch={host} >nul && set""#));
    parse_environment(&crate::captured(&mut command)?)
}

fn parse_environment(bytes: &[u8]) -> Result<Vec<(OsString, OsString)>> {
    ensure!(
        bytes.len() % 2 == 0,
        "invalid UTF-16 environment from VsDevCmd"
    );
    let wide: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect();
    let mut variables = Vec::new();
    for line in wide.split(|c| *c == 10) {
        let line = line.strip_suffix(&[13]).unwrap_or(line);
        let Some(separator) = line.iter().position(|c| *c == 61) else {
            continue;
        };
        // cmd's =C: drive entries are not ordinary environment variables.
        if separator == 0 {
            continue;
        }
        variables.push((
            OsString::from_wide(&line[..separator]),
            OsString::from_wide(&line[separator + 1..]),
        ));
    }
    ensure!(
        variables
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("PATH")),
        "VsDevCmd did not return PATH"
    );
    Ok(variables)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_unicode_spaces_and_equals_in_environment() {
        let bytes: Vec<u8> = "=C:=C:\\work\r\nPath=C:\\开发 工具\r\nVALUE=a=b\r\n"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        let variables = parse_environment(&bytes).unwrap();
        assert_eq!(
            variables,
            vec![
                ("Path".into(), "C:\\开发 工具".into()),
                ("VALUE".into(), "a=b".into())
            ]
        );
        assert!(parse_environment(&[1]).is_err());
        assert!(parse_environment(&[]).is_err());
    }

    #[test]
    #[ignore = "requires a local Visual Studio C++ installation and Windows SDK"]
    fn discovered_environment_runs_windows_tools() {
        let environment = discover("x86_64").unwrap();
        for tool in ["dumpbin", "rc"] {
            let output = probe(tool).envs(environment.clone()).output().unwrap();
            assert!(output.status.success(), "{tool}: {output:?}");
        }
    }
}
