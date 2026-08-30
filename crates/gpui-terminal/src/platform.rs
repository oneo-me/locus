use super::*;

pub(super) fn configured_shell(configured: &str) -> PathBuf {
    let configured = configured.trim();
    if !configured.is_empty() {
        let path = PathBuf::from(configured);
        if path.components().count() != 1 {
            return path;
        }
        return executable_in_path(configured)
            .or_else(|| executable_from_login_shell(configured))
            .unwrap_or(path);
    }
    std::env::var_os("SHELL")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            if cfg!(target_os = "macos") {
                PathBuf::from("/bin/zsh")
            } else {
                PathBuf::from("/bin/sh")
            }
        })
}

pub(super) fn executable_in_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|directory| directory.join(name))
        .find(|path| is_executable(path))
}

#[cfg(unix)]
pub(super) fn executable_from_login_shell(name: &str) -> Option<PathBuf> {
    let login_shell = std::env::var_os("SHELL")
        .map(PathBuf::from)
        .filter(|path| is_executable(path))
        .unwrap_or_else(|| {
            if cfg!(target_os = "macos") {
                PathBuf::from("/bin/zsh")
            } else {
                PathBuf::from("/bin/sh")
            }
        });
    let output = Command::new(login_shell)
        .args(["-l", "-i", "-c", "command -v -- \"$LOCUS_TERMINAL_SHELL\""])
        .env("LOCUS_TERMINAL_SHELL", name)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()?
        .lines()
        .rev()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .find(|path| path.is_absolute() && is_executable(path))
}

#[cfg(not(unix))]
pub(super) fn executable_from_login_shell(_: &str) -> Option<PathBuf> {
    None
}

pub(super) fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

pub(super) fn shell_arguments(_: &Path) -> Vec<String> {
    if cfg!(windows) {
        Vec::new()
    } else {
        vec!["-l".to_owned()]
    }
}
