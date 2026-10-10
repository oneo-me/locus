// Package terminal describes explicit launches in the built-in terminal.
// Onevra's MyGo terminal owns PTYs and terminal rendering.
package terminal

import (
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
)

func Shells() []string {
	if runtime.GOOS == "windows" {
		return []string{"default", "pwsh", "powershell", "cmd"}
	}
	return []string{"default", "zsh", "bash", "fish", "sh", "pwsh"}
}

func Agents() []string { return []string{"codex", "claude-code", "opencode", "pi", "gemini", "aider"} }

// ResolveCommand locates an explicitly selected shell before passing it to the
// PTY. A desktop app's PATH may omit package-manager installation directories.
// A nil command retains MyGo's platform discovery for default shell sessions.
func ResolveCommand(shell, agent string) ([]string, error) {
	argv, err := Command(shell, agent)
	if err != nil || len(argv) == 0 {
		return argv, err
	}
	argv[0], err = resolveShell(argv[0], runtime.GOOS, os.Getenv("SHELL"), exec.LookPath)
	if err != nil {
		return nil, err
	}
	return argv, nil
}

func resolveShell(program, platform, loginShell string, lookPath func(string) (string, error)) (string, error) {
	path, err := lookPath(program)
	if err == nil {
		return path, nil
	}
	// Preserve explicit paths and unsafe relative-PATH errors rather than
	// silently choosing a different executable.
	if filepath.Base(program) != program || !errors.Is(err, exec.ErrNotFound) {
		return "", err
	}
	candidates := []string{}
	if filepath.IsAbs(loginShell) && filepath.Base(loginShell) == program {
		candidates = append(candidates, loginShell)
	}
	if platform == "darwin" {
		for _, dir := range []string{"/opt/homebrew/bin", "/usr/local/bin", "/opt/local/bin", "/bin", "/usr/bin"} {
			candidates = append(candidates, filepath.Join(dir, program))
		}
	}
	for _, candidate := range candidates {
		if path, candidateErr := lookPath(candidate); candidateErr == nil {
			return path, nil
		}
	}
	return "", fmt.Errorf("cannot find shell %q; install it or select another shell in Settings: %w", program, err)
}

// Command returns argv, never a command containing a library path. The PTY
// receives its working directory separately. Agent names are fixed commands.
func Command(shell, agent string) ([]string, error) {
	program := shell
	if shell == "default" || shell == "" {
		if agent == "" {
			return nil, nil
		}
		program = os.Getenv("SHELL")
		if runtime.GOOS == "windows" {
			program = "powershell"
		} else if program == "" {
			program = "/bin/sh"
			if runtime.GOOS == "darwin" {
				program = "/bin/zsh"
			}
		}
	}
	name := strings.TrimSuffix(filepath.Base(program), ".exe")
	switch name {
	case "zsh", "bash", "fish", "sh", "pwsh", "powershell", "cmd":
	default:
		return nil, fmt.Errorf("unsupported shell: %s", program)
	}
	command := agent
	if agent == "claude-code" {
		command = "claude"
	}
	if agent != "" {
		valid := false
		for _, choice := range Agents() {
			if agent == choice {
				valid = true
			}
		}
		if !valid {
			return nil, fmt.Errorf("unknown Agent: %s", agent)
		}
	}
	switch name {
	case "pwsh", "powershell":
		if command == "" {
			return []string{program, "-NoLogo"}, nil
		}
		return []string{program, "-NoLogo", "-Command", command + "; exit $LASTEXITCODE"}, nil
	case "cmd":
		if command == "" {
			return []string{program}, nil
		}
		return []string{program, "/C", command}, nil
	default:
		if command == "" {
			return []string{program, "-li"}, nil
		}
		// Invoke through the shell so user-defined functions and aliases work.
		// Exit explicitly with the Agent status instead of leaving a prompt.
		status := "$?"
		if name == "fish" {
			status = "$status"
		}
		return []string{program, "-lic", command + "; exit " + status}, nil
	}
}
