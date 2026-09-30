use super::*;

pub(super) struct ActiveProcess {
    fallback_name: String,
    #[cfg(unix)]
    pty: Option<std::fs::File>,
    #[cfg(unix)]
    child_pid: u32,
}

impl ActiveProcess {
    #[cfg(unix)]
    pub(super) fn new(pty: &tty::Pty, shell: &Path) -> Self {
        Self {
            fallback_name: executable_name(shell),
            pty: pty.file().try_clone().ok(),
            child_pid: pty.child().id(),
        }
    }

    #[cfg(windows)]
    pub(super) fn new(_: &tty::Pty, shell: &Path) -> Self {
        Self {
            fallback_name: executable_name(shell),
        }
    }

    pub(super) fn name(&self) -> String {
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd as _;

            let foreground_pid = self
                .pty
                .as_ref()
                .map(|pty| unsafe { libc::tcgetpgrp(pty.as_raw_fd()) })
                .unwrap_or_default();
            let pid = if foreground_pid > 0 {
                foreground_pid as u32
            } else {
                self.child_pid
            };
            if let Some(name) = process_name(pid) {
                return name;
            }
        }

        self.fallback_name.clone()
    }
}

fn executable_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("terminal")
        .to_owned()
}

#[cfg(target_os = "macos")]
fn process_name(pid: u32) -> Option<String> {
    let mut buffer = vec![0_u8; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
    let length = unsafe {
        libc::proc_name(
            pid as libc::c_int,
            buffer.as_mut_ptr().cast(),
            buffer.len() as u32,
        )
    };
    (length > 0)
        .then(|| String::from_utf8_lossy(&buffer[..length as usize]).into_owned())
        .filter(|name| !name.is_empty())
}

#[cfg(target_os = "linux")]
fn process_name(pid: u32) -> Option<String> {
    std::fs::read_to_string(format!("/proc/{pid}/comm"))
        .ok()
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty())
}

#[cfg(all(unix, not(any(target_os = "macos", target_os = "linux"))))]
fn process_name(_: u32) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::executable_name;
    use std::path::Path;

    #[test]
    fn executable_name_uses_the_file_name() {
        assert_eq!(executable_name(Path::new("/bin/zsh")), "zsh");
        assert_eq!(executable_name(Path::new("")), "terminal");
    }
}
