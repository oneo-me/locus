use std::path::Path;
use std::process::Command;

use locus_preferences::{AgentPreference, TerminalPreference};

pub fn open_agent(
    root: &Path,
    terminal: TerminalPreference,
    agent: AgentPreference,
) -> Result<(), String> {
    if !root.is_absolute() || !root.is_dir() {
        return Err("The library directory is unavailable.".to_owned());
    }
    let mut command = launch_command(root, terminal, agent)?;
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
    format!("'{}'", value.replace('\'', "'\"'\"'"))
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
        new window with configuration config
        activate
    end tell
end run"#;

#[cfg(target_os = "macos")]
fn launch_command(
    root: &Path,
    terminal: TerminalPreference,
    agent: AgentPreference,
) -> Result<Command, String> {
    let root_text = root
        .to_str()
        .ok_or("The library path is not valid Unicode.")?;
    let script = format!(
        "cd -- {} && {}; exec /bin/zsh -l",
        shell_quote(root_text),
        agent.command()
    );
    let mut command = match terminal {
        TerminalPreference::System => {
            let mut command = Command::new("/usr/bin/osascript");
            command.args(["-e", TERMINAL_SCRIPT, "--"]);
            command.arg(format!("/bin/zsh -lic {}", shell_quote(&script)));
            command
        }
        TerminalPreference::Ghostty => {
            let mut command = Command::new("/usr/bin/osascript");
            command.args(["-e", GHOSTTY_SCRIPT, "--", root_text]);
            command.arg(format!("/bin/zsh -lic {}", shell_quote(&script)));
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
) -> Result<Command, String> {
    use std::os::windows::process::CommandExt;
    if terminal != TerminalPreference::System {
        return Err("This terminal is unavailable on Windows.".to_owned());
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
) -> Result<Command, String> {
    let root_text = root
        .to_str()
        .ok_or("The library path is not valid Unicode.")?;
    let script = format!(
        "cd -- {} && {}; exec /bin/sh -l",
        shell_quote(root_text),
        agent.command()
    );
    let mut command = Command::new(match terminal {
        TerminalPreference::System => "x-terminal-emulator",
        TerminalPreference::Ghostty => "ghostty",
    });
    command
        .args(["-e", "/bin/sh", "-lc", &script])
        .current_dir(root);
    Ok(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn library_paths_are_literal_shell_arguments() {
        let path = "/tmp/笔记 ' \" $HOME $(exit 23) `exit 24` ; &\nnext";
        let output = Command::new("/bin/sh")
            .args(["-c", &format!("printf '%s' {}", shell_quote(path))])
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(String::from_utf8(output.stdout).unwrap(), path);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn each_terminal_receives_the_library_root_and_selected_agent() {
        let root = Path::new("/tmp/笔记 ' \" $HOME ; test");
        for agent in AgentPreference::ALL {
            for terminal in TerminalPreference::ALL {
                let command = launch_command(root, *terminal, agent).unwrap();
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
                            "/bin/zsh -lic {}",
                            shell_quote(&format!(
                                "cd -- {} && {}; exec /bin/zsh -l",
                                shell_quote(root.to_str().unwrap()),
                                agent.command()
                            ))
                        )
                    );
                }
            }
        }
    }

    #[test]
    fn invalid_library_is_rejected_before_launch() {
        assert!(
            open_agent(
                Path::new("relative-library"),
                TerminalPreference::System,
                AgentPreference::Codex
            )
            .is_err()
        );
    }
}
