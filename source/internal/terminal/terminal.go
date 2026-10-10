// Package terminal hands explicit user launches to external terminals. It does
// not own Agent sessions or launch software as a side effect of startup.
package terminal

import (
	"errors"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
)

func Quote(value string) string { return "'" + strings.ReplaceAll(value, "'", "'\\''") + "'" }
func Command(root, shell, agent string) (string, error) {
	switch agent {
	case "codex", "pi", "opencode", "claude":
	default:
		return "", errors.New("unknown Agent")
	}
	if info, err := os.Stat(root); err != nil || !info.IsDir() {
		return "", errors.New("library directory is unavailable")
	}
	if shell == "default" || shell == "" {
		shell = "zsh"
	}
	switch shell {
	case "zsh", "bash", "fish":
	default:
		return "", errors.New("unknown shell")
	}
	program, err := exec.LookPath(shell)
	if err != nil {
		return "", err
	}
	program, err = filepath.Abs(program)
	if err != nil {
		return "", err
	}
	// The outer command uses POSIX quoting; the login shell receives one command.
	quoteInner := Quote
	if shell == "fish" {
		quoteInner = func(s string) string { return "'" + strings.NewReplacer("\\", "\\\\", "'", "\\'").Replace(s) + "'" }
	}
	inner := "cd -- " + quoteInner(root) + " && " + agent + "; exec " + quoteInner(program) + " -li"
	return "exec " + Quote(program) + " -lic " + Quote(inner), nil
}
func Launch(root, terminal, shell, agent string) error {
	if agent == "claude-code" {
		agent = "claude"
	}
	switch runtime.GOOS {
	case "darwin":
		command, err := Command(root, shell, agent)
		if err != nil {
			return err
		}
		script := `on run argv
 tell application "Terminal"
  launch
  do script (item 1 of argv)
  activate
 end tell
end run`
		args := []string{"-e", script, command}
		if terminal == "ghostty" {
			script = `on run argv
 tell application "Ghostty"
  launch
  set config to new surface configuration
  set initial working directory of config to item 2 of argv
  set command of config to item 1 of argv
  if (count of windows) > 0 then
   new tab in front window with configuration config
  else
   new window with configuration config
  end if
  activate
 end tell
end run`
			args = []string{"-e", script, command, root}
		} else if terminal != "system" {
			return errors.New("unknown terminal")
		}
		return exec.Command("/usr/bin/osascript", args...).Run()
	case "windows":
		if terminal != "system" && terminal != "" {
			return errors.New("this platform supports only the system terminal")
		}
		switch agent {
		case "codex", "pi", "opencode", "claude":
		default:
			return errors.New("unknown Agent")
		}
		if info, err := os.Stat(root); err != nil || !info.IsDir() {
			return errors.New("library directory is unavailable")
		}
		cmd := exec.Command("cmd.exe", "/c", "start", "", "cmd.exe", "/k", agent)
		cmd.Dir = root
		return cmd.Run()
	default:
		return errors.New("external terminal launching is not implemented on this platform")
	}
}
