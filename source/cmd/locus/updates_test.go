package main

import (
	"context"
	"errors"
	"testing"

	"github.com/egoist/mygo/ui"
	"github.com/oneo-me/locus/internal/preferences"
)

// updateTestApp returns an application whose update outcomes queue on a
// channel, so a test runs them as the UI thread would.
func updateTestApp(found *availableUpdate, err error) (*application, chan func()) {
	a := &application{cfg: preferences.Default()}
	a.cfg.Language = "en"
	queue := make(chan func(), 16)
	a.updates = updateFlow{enabled: true, post: func(fn func()) { queue <- fn }, check: func(context.Context) (*availableUpdate, error) { return found, err }}
	return a, queue
}

func aboutTester(a *application) *ui.Tester {
	a.settingState.Category = "about"
	return ui.NewTester(func(c *ui.Context) { a.settings(c, false) }, 900, 700)
}

func TestAboutWithoutUpdates(t *testing.T) {
	a := &application{cfg: preferences.Default()}
	a.cfg.Language = "en"
	tt := aboutTester(a)
	for _, label := range []string{"About", "Locus", "Development build", "This build does not update itself.", "Check for updates", "Check for updates automatically"} {
		if !tt.HasText(label) {
			t.Fatalf("missing %q", label)
		}
	}
	a.checkForUpdates()
	if !a.settingsOpen || a.settingState.Category != "about" || a.updates.phase != updateIdle {
		t.Fatalf("check without updates: open=%v category=%q phase=%v", a.settingsOpen, a.settingState.Category, a.updates.phase)
	}
}

func TestManualUpdateCheckAndInstall(t *testing.T) {
	installed := false
	found := &availableUpdate{Version: "9.0.0", Notes: "- Faster search", install: func(_ context.Context, progress func(int64, int64)) error {
		progress(5, 10)
		installed = true
		return nil
	}}
	a, queue := updateTestApp(found, nil)
	a.checkForUpdates()
	if !a.settingsOpen || a.settingState.Category != "about" || a.updates.phase != updateChecking {
		t.Fatalf("check did not open About: open=%v category=%q phase=%v", a.settingsOpen, a.settingState.Category, a.updates.phase)
	}
	(<-queue)()
	if a.updates.phase != updateAvailable || a.state.UpdateChecked.IsZero() || len(a.notices) != 0 || !a.updatePending() {
		t.Fatalf("manual find: phase=%v notices=%d", a.updates.phase, len(a.notices))
	}
	tt := aboutTester(a)
	for _, label := range []string{"Download and install", "Faster search", "New version 9.0.0"} {
		if !tt.HasText(label) {
			t.Fatalf("missing %q", label)
		}
	}
	a.installUpdate()
	(<-queue)() // progress
	if a.updates.progress != 0.5 {
		t.Fatalf("progress %v", a.updates.progress)
	}
	(<-queue)()
	if !installed || a.updates.phase != updateReady || len(a.notices) != 0 {
		t.Fatalf("install: installed=%v phase=%v notices=%d", installed, a.updates.phase, len(a.notices))
	}
	if !aboutTester(a).HasText("Restart to update") {
		t.Fatal("missing restart action")
	}
	a.checkUpdates(true)
	if a.updates.phase != updateReady {
		t.Fatal("a check replaced the pending relaunch")
	}
}

func TestBackgroundUpdatePrompts(t *testing.T) {
	a, queue := updateTestApp(&availableUpdate{Version: "9.0.0"}, nil)
	a.checkUpdates(false)
	(<-queue)()
	if len(a.notices) != 1 || a.notices[0].action != "Details" || a.settingsOpen {
		t.Fatalf("background find: %+v", a.notices)
	}
	a.notices[0].onAction()
	if !a.settingsOpen || a.settingState.Category != "about" {
		t.Fatal("Details did not open About")
	}
	a.updates.phase = updateCurrent
	a.checkUpdates(false)
	(<-queue)()
	if len(a.notices) != 1 {
		t.Fatal("the same version was announced twice")
	}
}

func TestBackgroundUpdateDownloads(t *testing.T) {
	found := &availableUpdate{Version: "9.0.0", install: func(context.Context, func(int64, int64)) error { return nil }}
	a, queue := updateTestApp(found, nil)
	a.cfg.UpdateDownloads = true
	a.checkUpdates(false)
	(<-queue)()
	if a.updates.phase != updateDownloading {
		t.Fatalf("automatic download did not start: %v", a.updates.phase)
	}
	(<-queue)()
	if a.updates.phase != updateReady || len(a.notices) != 1 || a.notices[0].action != "Restart" {
		t.Fatalf("ready: phase=%v notices=%+v", a.updates.phase, a.notices)
	}
}

func TestUpdateCheckFailure(t *testing.T) {
	a, queue := updateTestApp(nil, errors.New("offline"))
	a.checkUpdates(true)
	(<-queue)()
	if a.updates.phase != updateFailed || !a.state.UpdateChecked.IsZero() || !aboutTester(a).HasText("Update failed · offline") {
		t.Fatalf("failure: phase=%v", a.updates.phase)
	}
}
