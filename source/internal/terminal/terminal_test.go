package terminal

import (
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"runtime"
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

func TestResolveShell(t *testing.T) {
	for _, tc := range []struct {
		name, program, platform, login, available, want string
		lookupErr                                       error
	}{
		{name: "PATH wins", program: "fish", platform: "darwin", login: "/custom/fish", available: "fish", want: "/path/fish"},
		{name: "login shell", program: "fish", platform: "darwin", login: "/custom/fish", available: "/custom/fish", want: "/custom/fish"},
		{name: "Apple Silicon Homebrew", program: "fish", platform: "darwin", available: "/opt/homebrew/bin/fish", want: "/opt/homebrew/bin/fish"},
		{name: "Intel Homebrew", program: "fish", platform: "darwin", available: "/usr/local/bin/fish", want: "/usr/local/bin/fish"},
		{name: "MacPorts", program: "fish", platform: "darwin", available: "/opt/local/bin/fish", want: "/opt/local/bin/fish"},
		{name: "system shell", program: "zsh", platform: "darwin", available: "/bin/zsh", want: "/bin/zsh"},
		{name: "other login shell", program: "fish", platform: "linux", login: "/custom/bash", available: "/custom/bash"},
		{name: "no mac paths on Linux", program: "fish", platform: "linux", available: "/opt/homebrew/bin/fish"},
		{name: "explicit missing path", program: "/missing/fish", platform: "darwin", available: "/opt/homebrew/bin/fish"},
		{name: "missing executable", program: "fish", platform: "darwin"},
		{name: "relative PATH rejected", program: "fish", platform: "darwin", available: "/opt/homebrew/bin/fish", lookupErr: exec.ErrDot},
	} {
		t.Run(tc.name, func(t *testing.T) {
			lookup := func(name string) (string, error) {
				if name == tc.available {
					if name == "fish" {
						return "/path/fish", nil
					}
					return name, nil
				}
				err := tc.lookupErr
				if err == nil {
					err = exec.ErrNotFound
				}
				return "", &exec.Error{Name: name, Err: err}
			}
			got, err := resolveShell(tc.program, tc.platform, tc.login, lookup)
			if got != tc.want || (err != nil) != (tc.want == "") {
				t.Fatalf("got %q, %v; want %q", got, err, tc.want)
			}
			if tc.lookupErr != nil && !errors.Is(err, tc.lookupErr) {
				t.Fatalf("lost lookup error: %v", err)
			}
		})
	}
}

func TestResolveCommandDesktopPATH(t *testing.T) {
	if runtime.GOOS != "darwin" {
		t.Skip("macOS desktop PATH regression")
	}
	program := "/opt/homebrew/bin/fish"
	if _, err := exec.LookPath(program); err != nil {
		t.Skip("Homebrew fish is not installed")
	}
	t.Setenv("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
	t.Setenv("SHELL", "/bin/zsh")
	for _, agent := range []string{"", "gemini"} {
		argv, err := ResolveCommand("fish", agent)
		if err != nil {
			t.Fatal(err)
		}
		want, _ := Command("fish", agent)
		want[0] = program
		if !reflect.DeepEqual(argv, want) {
			t.Fatalf("got %v; want %v", argv, want)
		}
	}
}
