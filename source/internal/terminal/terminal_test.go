package terminal

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

func TestCommand(t *testing.T) {
	for _, tc := range []struct {
		shell, agent string
		want         []string
	}{
		{"default", "", nil},
		{"zsh", "", []string{"zsh", "-li"}},
		{"bash", "claude-code", []string{"bash", "-lic", "claude; exit $?"}},
		{"fish", "gemini", []string{"fish", "-lic", "gemini; exit $status"}},
		{"pwsh", "codex", []string{"pwsh", "-NoLogo", "-Command", "codex; exit $LASTEXITCODE"}},
		{"cmd", "aider", []string{"cmd", "/C", "aider"}},
	} {
		got, err := Command(tc.shell, tc.agent)
		if err != nil || !reflect.DeepEqual(got, tc.want) {
			t.Fatalf("%s/%s: %v, %v", tc.shell, tc.agent, got, err)
		}
	}
	for _, shell := range Shells() {
		for _, agent := range Agents() {
			if _, err := Command(shell, agent); err != nil {
				t.Fatal(err)
			}
		}
	}
}

func TestCommandRejectsInjection(t *testing.T) {
	for _, tc := range [][2]string{{"bash", "unknown;exit"}, {"bash;exit", "codex"}, {"bash", "$(touch bad)"}} {
		if _, err := Command(tc[0], tc[1]); err == nil {
			t.Fatalf("accepted %v", tc)
		}
	}
	t.Setenv("SHELL", "/tmp/unsupported")
	// Explicit choices never depend on the environment's shell.
	if _, err := Command("bash", "codex"); err != nil {
		t.Fatal(err)
	}
}

// Use isolated shell functions, never an installed Agent. This exercises shell
// startup and exit propagation, which argv-only tests cannot establish.
func TestAgentShellFunction(t *testing.T) {
	for _, shell := range []string{"fish", "zsh", "bash"} {
		program, err := exec.LookPath(shell)
		if err != nil {
			t.Run(shell, func(t *testing.T) { t.Skip(err) })
			continue
		}
		for _, code := range []int{0, 23, 130} {
			t.Run(fmt.Sprintf("%s/%d", shell, code), func(t *testing.T) {
				home := t.TempDir()
				config := filepath.Join(home, ".zshrc")
				body := fmt.Sprintf("pi() { printf 'fixture-agent-function'; return %d; }\n", code)
				switch shell {
				case "fish":
					config = filepath.Join(home, "fish", "config.fish")
					body = fmt.Sprintf("function pi\n printf 'fixture-agent-function'\n return %d\nend\n", code)
				case "bash":
					config = filepath.Join(home, ".bash_profile")
				}
				bin := filepath.Join(home, "bin")
				if err := os.MkdirAll(bin, 0700); err != nil {
					t.Fatal(err)
				}
				if err := os.WriteFile(filepath.Join(bin, "pi"), []byte("#!/bin/sh\nprintf bypassed-function\nexit 91\n"), 0700); err != nil {
					t.Fatal(err)
				}
				if shell == "fish" {
					body = fmt.Sprintf("set -gx PATH %q /usr/bin /bin\n", bin) + body
				} else {
					body = fmt.Sprintf("export PATH=%q:/usr/bin:/bin\n", bin) + body
				}
				if err := os.MkdirAll(filepath.Dir(config), 0700); err != nil {
					t.Fatal(err)
				}
				if err := os.WriteFile(config, []byte(body), 0600); err != nil {
					t.Fatal(err)
				}
				argv, err := Command(shell, "pi")
				if err != nil {
					t.Fatal(err)
				}
				cmd := exec.Command(program, argv[1:]...)
				cmd.Env = append(os.Environ(), "HOME="+home, "ZDOTDIR="+home, "XDG_CONFIG_HOME="+home)
				out, err := cmd.CombinedOutput()
				if !strings.Contains(string(out), "fixture-agent-function") {
					t.Fatalf("function not invoked: %s (%v)", out, err)
				}
				if cmd.ProcessState.ExitCode() != code {
					t.Fatalf("exit %d, want %d: %s", cmd.ProcessState.ExitCode(), code, out)
				}
			})
		}
	}
}
