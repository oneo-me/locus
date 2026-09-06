use std::ffi::OsStr;
use std::process::Command;

pub fn command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    #[cfg(target_os = "macos")]
    if let Some(path) = login_shell_path() {
        command.env("PATH", path);
    }
    command
}

#[cfg(target_os = "macos")]
fn login_shell_path() -> Option<&'static OsStr> {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    static PATH: OnceLock<Option<OsString>> = OnceLock::new();

    PATH.get_or_init(|| {
        let shell = std::env::var_os("SHELL")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/bin/zsh"));
        let output = Command::new(shell)
            .args(["-l", "-c", "printf '\\0%s\\0' \"$PATH\""])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        path_from_shell_output(&output.stdout).map(OsString::from_vec)
    })
    .as_deref()
}

#[cfg(target_os = "macos")]
fn path_from_shell_output(output: &[u8]) -> Option<Vec<u8>> {
    let end = output.iter().rposition(|byte| *byte == 0)?;
    let start = output[..end].iter().rposition(|byte| *byte == 0)?;
    let path = output[start + 1..end].to_vec();
    (!path.is_empty()).then_some(path)
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    fn extracts_path_between_trailing_delimiters() {
        assert_eq!(
            path_from_shell_output(
                b"profile output\n\0/usr/bin:/opt/homebrew/bin\0trailing output\n"
            ),
            Some(b"/usr/bin:/opt/homebrew/bin".to_vec())
        );
    }
}
