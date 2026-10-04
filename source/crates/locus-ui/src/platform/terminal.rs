use std::path::Path;
use std::process::Command;

use locus_preferences::{AgentPreference, ShellPreference, TerminalPreference};

pub fn open_agent(
    root: &Path,
    terminal: TerminalPreference,
    agent: AgentPreference,
    shell: ShellPreference,
) -> Result<(), String> {
    if !root.is_absolute() || !root.is_dir() {
        return Err("The library directory is unavailable.".to_owned());
    }
    let mut command = launch_command(root, terminal, agent, shell)?;
    #[cfg(target_os = "macos")]
    {
        let output = command.output().map_err(|error| error.to_string())?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let mut child = command.spawn().map_err(|error| error.to_string())?;
        std::thread::spawn(move || {
            let _ = child.wait();
        });
    }
    Ok(())
}

#[cfg(unix)]
fn shell_quote(value: &str) -> String {
    // Concatenated quotes preserve backslashes in both POSIX shells and fish.
    format!(
        "'{}'",
        value.replace('\'', "'\"'\"'").replace('\\', "'\"\\\\\"'")
    )
}

#[cfg(unix)]
fn shell_path(shell: ShellPreference) -> Result<std::path::PathBuf, String> {
    use std::os::unix::fs::PermissionsExt;

    if shell == ShellPreference::Default {
        return Ok(if cfg!(target_os = "macos") {
            "/bin/zsh"
        } else {
            "/bin/sh"
        }
        .into());
    }
    let name = match shell {
        ShellPreference::Zsh => "zsh",
        ShellPreference::Bash => "bash",
        ShellPreference::Fish => "fish",
        ShellPreference::Default => unreachable!(),
    };
    let directories = std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .unwrap_or_default();
    directories
        .into_iter()
        .chain(
            ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin"]
                .map(std::path::PathBuf::from),
        )
        .filter(|directory| directory.is_absolute())
        .map(|directory| directory.join(name))
        .find(|path| {
            std::fs::metadata(path).is_ok_and(|metadata| {
                metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
            })
        })
        .ok_or_else(|| {
            format!(
                "The selected shell ({name}) is unavailable. Install it before launching an Agent."
            )
        })
}

#[cfg(unix)]
fn agent_script(root: &str, agent: AgentPreference, shell: &str) -> String {
    format!(
        "cd -- {} && {}; exec {} -l",
        shell_quote(root),
        agent.command(),
        shell_quote(shell)
    )
}

#[cfg(target_os = "macos")]
const TERMINAL_SCRIPT: &str = r#"on run argv
    tell application "Terminal"
        launch
        do script (item 1 of argv)
        activate
    end tell
end run"#;

#[cfg(target_os = "macos")]
const GHOSTTY_SCRIPT: &str = r#"on run argv
    tell application "Ghostty"
        launch
        set config to new surface configuration
        set initial working directory of config to item 1 of argv
        set command of config to item 2 of argv
        if (count of windows) > 0 then
            new tab in front window with configuration config
        else
            new window with configuration config
        end if
        activate
    end tell
end run"#;

#[cfg(target_os = "macos")]
fn launch_command(
    root: &Path,
    terminal: TerminalPreference,
    agent: AgentPreference,
    shell: ShellPreference,
) -> Result<Command, String> {
    let root_text = root
        .to_str()
        .ok_or("The library path is not valid Unicode.")?;
    let shell = shell_path(shell)?;
    let shell = shell
        .to_str()
        .ok_or("The shell path is not valid Unicode.")?;
    let script = agent_script(root_text, agent, shell);
    let shell_command = format!("{} -lic {}", shell_quote(shell), shell_quote(&script));
    let mut command = match terminal {
        TerminalPreference::System => {
            let mut command = Command::new("/usr/bin/osascript");
            command.args(["-e", TERMINAL_SCRIPT, "--"]);
            command.arg(&shell_command);
            command
        }
        TerminalPreference::Ghostty => {
            let mut command = Command::new("/usr/bin/osascript");
            command.args(["-e", GHOSTTY_SCRIPT, "--", root_text]);
            command.arg(&shell_command);
            command
        }
    };
    command.current_dir(root);
    Ok(command)
}

#[cfg(target_os = "windows")]
fn launch_command(
    root: &Path,
    terminal: TerminalPreference,
    agent: AgentPreference,
    shell: ShellPreference,
) -> Result<Command, String> {
    use std::os::windows::process::CommandExt;
    if terminal != TerminalPreference::System {
        return Err("This terminal is unavailable on Windows.".to_owned());
    }
    if shell != ShellPreference::Default {
        return Err("Shell selection is unavailable on Windows.".to_owned());
    }
    let mut command = Command::new("cmd.exe");
    command
        .args(["/D", "/K", agent.command()])
        .current_dir(root)
        .creation_flags(0x00000010);
    Ok(command)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn launch_command(
    root: &Path,
    terminal: TerminalPreference,
    agent: AgentPreference,
    shell: ShellPreference,
) -> Result<Command, String> {
    let root_text = root
        .to_str()
        .ok_or("The library path is not valid Unicode.")?;
    let flags = if shell == ShellPreference::Default {
        "-lc"
    } else {
        "-lic"
    };
    let shell = shell_path(shell)?;
    let script = agent_script(
        root_text,
        agent,
        shell
            .to_str()
            .ok_or("The shell path is not valid Unicode.")?,
    );
    let mut command = Command::new(match terminal {
        TerminalPreference::System => "x-terminal-emulator",
        TerminalPreference::Ghostty => "ghostty",
    });
    command
        .arg("-e")
        .arg(shell)
        .args([flags, &script])
        .current_dir(root);
    Ok(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn library_paths_are_literal_shell_arguments() {
        let path = "/tmp/笔记 ' \" $HOME $(exit 23) `exit 24` ; & \\\\ \\'\nnext";
        let output = Command::new("/bin/sh")
            .args(["-c", &format!("printf '%s' {}", shell_quote(path))])
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(String::from_utf8(output.stdout).unwrap(), path);
    }

    #[cfg(unix)]
    #[test]
    fn installed_shells_preserve_literal_arguments() {
        let value = "/tmp/笔记 ' \" $HOME $(exit 23) `exit 24` ; & \\\\ \\'\nnext";
        for preference in ShellPreference::ALL {
            let Ok(shell) = shell_path(*preference) else {
                continue;
            };
            let output = Command::new(&shell)
                .args(["-c", &format!("printf '%s' {}", shell_quote(value))])
                .output()
                .unwrap();
            assert!(output.status.success(), "{shell:?}: {:?}", output.stderr);
            assert_eq!(
                String::from_utf8(output.stdout).unwrap(),
                value,
                "{shell:?}"
            );
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn each_terminal_receives_the_library_root_and_selected_agent() {
        let root = Path::new("/tmp/笔记 ' \" $HOME ; test");
        for agent in AgentPreference::ALL {
            for terminal in TerminalPreference::ALL {
                let command =
                    launch_command(root, *terminal, agent, ShellPreference::Default).unwrap();
                assert_eq!(command.get_current_dir(), Some(root));
                let args: Vec<_> = command
                    .get_args()
                    .map(|arg| arg.to_str().unwrap())
                    .collect();
                if *terminal == TerminalPreference::System {
                    assert_eq!(args[..3], ["-e", TERMINAL_SCRIPT, "--"]);
                    assert!(args[3].contains(agent.command()));
                } else {
                    assert_eq!(args[..3], ["-e", GHOSTTY_SCRIPT, "--"]);
                    assert_eq!(args[3], root.to_str().unwrap());
                    assert!(args[4].contains(agent.command()));
                    assert_eq!(
                        args[4],
                        format!(
                            "'/bin/zsh' -lic {}",
                            shell_quote(&agent_script(root.to_str().unwrap(), agent, "/bin/zsh"))
                        )
                    );
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn selected_shell_is_used_for_launch_and_after_agent_exit() {
        let root = Path::new("/tmp/notes");
        for preference in ShellPreference::ALL {
            let Ok(shell) = shell_path(*preference) else {
                continue;
            };
            for terminal in TerminalPreference::ALL {
                let command =
                    launch_command(root, *terminal, AgentPreference::Pi, *preference).unwrap();
                let actual = command.get_args().last().unwrap().to_str().unwrap();
                let script = agent_script(
                    root.to_str().unwrap(),
                    AgentPreference::Pi,
                    shell.to_str().unwrap(),
                );
                assert_eq!(
                    actual,
                    format!(
                        "{} -lic {}",
                        shell_quote(shell.to_str().unwrap()),
                        shell_quote(&script)
                    )
                );
            }
        }
    }

    #[test]
    fn invalid_library_is_rejected_before_launch() {
        assert!(
            open_agent(
                Path::new("relative-library"),
                TerminalPreference::System,
                AgentPreference::Codex,
                ShellPreference::Default
            )
            .is_err()
        );
    }
}
