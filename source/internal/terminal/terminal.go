// Package terminal describes explicit launches in the built-in terminal.
// Onevra's MyGo terminal owns PTYs and terminal rendering.
package terminal

import (
	"fmt"
	"os"
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
