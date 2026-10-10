package main

import (
	"errors"
	"fmt"
	"net"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/egoist/mygo"
	"github.com/egoist/mygo/plugins/terminal"
	"github.com/egoist/mygo/ui"
	"github.com/oneo-me/locus/internal/preferences"
)

func TestTerminalPopover(t *testing.T) {
	for _, language := range []string{"english", "chinese"} {
		a := &application{cfg: preferences.Default()}
		a.cfg.Language = language
		a.model.Root = t.TempDir()
		// Keep native loading out of this interaction test.
		a.terminal.loading = true
		tt := ui.NewTester(func(c *ui.Context) {
			ui.Column(c).Fill().Children(func() { a.title(c) })
		}, 980, 820)
		if err := tt.Click(a.text("Terminal")); err != nil {
			t.Fatal(err)
		}
		panel, ok := tt.Find(a.text("Terminal panel"))
		if !ok || panel.W != 600 || panel.H != 500 || panel.X < 0 || panel.X+panel.W > 980 || panel.Y < 30 {
			t.Fatalf("invalid popover: %+v", panel)
		}
		for _, label := range []string{"Shell", "Agent", "Open Agent", "New shell", "Stop terminal"} {
			if _, ok := tt.Find(a.text(label)); ok {
				t.Fatalf("unexpected terminal control %s", label)
			}
		}
		tt.Key(0, ui.KeyEscape)
		if a.terminalOpen {
			t.Fatal("Escape did not close terminal")
		}
		if _, loading, _ := a.terminal.snapshot(); !loading {
			t.Fatal("dismissal discarded session state")
		}
		a.settingState.Category = "agents"
		tt = ui.NewTester(func(c *ui.Context) { a.settings(c, false) }, 900, 700)
		for _, label := range []string{"Shell", "Agent"} {
			if _, ok := tt.Find(a.text(label)); !ok {
				t.Fatalf("missing setting %s", label)
			}
		}
		for _, label := range []string{"Terminal", "Open Agent"} {
			if _, ok := tt.Find(a.text(label)); ok {
				t.Fatalf("settings still exposes %s", label)
			}
		}
	}
}

func TestTerminalLoadCancellation(t *testing.T) {
	var s terminalSession
	started, release, returned := make(chan struct{}), make(chan struct{}), make(chan struct{})
	s.start(func() (*terminal.Terminal, error) {
		close(started)
		<-release
		close(returned)
		return nil, errors.New("stale failure")
	})
	<-started
	s.close()
	s.start(func() (*terminal.Terminal, error) { return nil, errors.New("current failure") })
	close(release)
	<-returned
	deadline := time.Now().Add(time.Second)
	for time.Now().Before(deadline) {
		_, loading, err := s.snapshot()
		if !loading && err != nil {
			if err.Error() != "current failure" {
				t.Fatal(err)
			}
			return
		}
		time.Sleep(time.Millisecond)
	}
	t.Fatal("terminal creation did not complete")
}

// TestNativeTerminal uses a connection fixture: it renders the actual terminal
// on macOS without starting a shell, external terminal, or Agent.
func TestNativeTerminal(t *testing.T) {
	if os.Getenv("LOCUS_NATIVE_SMOKE") != "1" {
		t.Skip("requires a native desktop session")
	}
	client, server := net.Pipe()
	defer server.Close()
	defer client.Close()
	root := t.TempDir()
	a := &application{cfg: preferences.Default()}
	mygo.RunOnMain(func() { a.start(root) })
	defer mygo.RunOnMain(func() { a.shell.Window().Destroy() })
	waitNative(t, func() bool { return !a.model.Loading })
	mygo.RunOnMain(func() {
		a.terminal.start(func() (*terminal.Terminal, error) { return terminal.New(terminal.Options{Conn: client}) })
		a.terminalOpen = true
		a.shell.Window().Invalidate()
	})
	waitNative(t, func() bool {
		term, loading, err := a.terminal.snapshot()
		return !loading && (term != nil || err != nil)
	})
	term, _, err := a.terminal.snapshot()
	if err != nil {
		t.Fatal(err)
	}
	go func() { _, _ = server.Write([]byte("\x1b[32mLocus terminal\x1b[0m\r\n中文 terminal fixture\r\n$ ")) }()
	waitNative(t, func() bool { return strings.Contains(term.Text(), "Locus terminal") })
	for _, mode := range []mygo.ThemeSource{mygo.ThemeLight, mygo.ThemeDark} {
		mygo.RunOnMain(func() { mygo.Theme.SetSource(mode); a.shell.Window().Invalidate() })
		time.Sleep(200 * time.Millisecond)
		data, err := a.shell.Window().CapturePage()
		if err != nil {
			t.Fatal(err)
		}
		if dir := os.Getenv("LOCUS_SCREENSHOTS"); dir != "" {
			if err := os.MkdirAll(dir, 0700); err != nil {
				t.Fatal(err)
			}
			if err := os.WriteFile(filepath.Join(dir, "terminal-"+string(mode)+".png"), data, 0600); err != nil {
				t.Fatal(err)
			}
		}
	}
	mygo.RunOnMain(func() { a.toggleTerminal(); a.toggleTerminal() })
	current, _, _ := a.terminal.snapshot()
	if current != term {
		t.Fatal("reopening replaced terminal")
	}
	mygo.RunOnMain(func() { a.openLibrary(t.TempDir()) })
	current, _, _ = a.terminal.snapshot()
	if current != nil {
		t.Fatal("library change retained terminal")
	}
	select {
	case <-term.Done():
	case <-time.After(time.Second):
		t.Fatal("terminal connection was not closed")
	}
}

func TestTerminalStaleExit(t *testing.T) {
	a := &application{terminalOpen: true}
	previous, current := new(terminal.Terminal), new(terminal.Terminal)
	a.terminal.term = current
	a.finishTerminal(previous)
	if term, _, _ := a.terminal.snapshot(); term != current || !a.terminalOpen {
		t.Fatal("old exit dismissed the new terminal")
	}
	// These identity-only fixtures do not own native terminal resources.
	a.terminal.term = nil
}

func TestNativeTerminalExit(t *testing.T) {
	if os.Getenv("LOCUS_NATIVE_SMOKE") != "1" {
		t.Skip("requires a native desktop session")
	}
	client, server := net.Pipe()
	defer client.Close()
	defer server.Close()
	a := &application{cfg: preferences.Default()}
	root := t.TempDir()
	mygo.RunOnMain(func() { a.start(root) })
	defer mygo.RunOnMain(func() { a.shell.Window().Destroy() })
	waitNative(t, func() bool { return !a.model.Loading })
	mygo.RunOnMain(func() {
		a.terminalOpen = true
		a.terminal.start(func() (*terminal.Terminal, error) {
			return terminal.New(terminal.Options{Conn: client})
		}, a.terminalExited)
		a.shell.Window().Invalidate()
	})
	waitNative(t, func() bool {
		term, loading, err := a.terminal.snapshot()
		return !loading && (term != nil || err != nil)
	})
	if _, _, err := a.terminal.snapshot(); err != nil {
		t.Fatal(err)
	}
	_ = server.Close()
	waitNative(t, func() bool { term, _, _ := a.terminal.snapshot(); return term == nil && !a.terminalOpen })
}

func TestNativeTerminalProcessExit(t *testing.T) {
	if os.Getenv("LOCUS_NATIVE_SMOKE") != "1" {
		t.Skip("requires a native desktop session")
	}
	for _, code := range []int{0, 130, 127, 1} {
		t.Run(fmt.Sprint(code), func(t *testing.T) {
			a := &application{cfg: preferences.Default()}
			root := t.TempDir()
			mygo.RunOnMain(func() { a.start(root) })
			defer mygo.RunOnMain(func() { a.shell.Window().Destroy() })
			waitNative(t, func() bool { return !a.model.Loading })
			handled := make(chan struct{})
			mygo.RunOnMain(func() {
				a.terminalOpen = true
				a.terminal.start(func() (*terminal.Terminal, error) {
					return terminal.New(terminal.Options{Dir: root, Command: []string{"/bin/sh", "-c", fmt.Sprintf("printf 'fixture exit'; exit %d", code)}})
				}, func(term *terminal.Terminal) {
					a.post(func() { a.finishTerminal(term); close(handled) })
				})
				a.shell.Window().Invalidate()
			})
			select {
			case <-handled:
			case <-time.After(5 * time.Second):
				t.Fatal("exit not delivered")
			}
			mygo.RunOnMain(func() {
				term, _, _ := a.terminal.snapshot()
				if code == 0 || code == 130 {
					if a.terminalOpen || term != nil {
						t.Error("normal exit did not dismiss terminal")
					}
				} else if !a.terminalOpen || term == nil || !strings.Contains(term.Text(), "fixture exit") {
					t.Error("failure dismissed terminal or discarded output")
				}
			})
		})
	}
}
