package main

import (
	"reflect"
	"testing"

	"github.com/egoist/mygo/ui"
	"github.com/oneo-me/locus/internal/library"
	"github.com/oneo-me/locus/internal/preferences"
)

func navigationFixture() *application {
	a := &application{cfg: preferences.Default(), history: []string{"one", "removed", "two"}, historyIndex: 2}
	a.cfg.Language = "english"
	a.model.Notes = []*library.Note{{Path: "one", Title: "One"}, {Path: "two", Title: "Two"}}
	a.model.Selected = "two"
	a.resetTree()
	return a
}

func TestMouseNavigationKeys(t *testing.T) {
	a := navigationFixture()
	// Content owns the shortcuts even when the sidebar is not rendered.
	tt := ui.NewTester(a.content, 800, 600)
	for _, step := range []struct {
		key   ui.Key
		index int
		path  string
	}{
		{ui.KeyForward, 2, "two"}, // End of history.
		{ui.KeyBack, 0, "one"},    // Skip the deleted note.
		{ui.KeyBack, 0, "one"},    // Start of history.
		{ui.KeyForward, 2, "two"},
	} {
		tt.Key(0, step.key)
		if a.historyIndex != step.index || a.model.Selected != step.path {
			t.Fatalf("key %v: index=%d selected=%q", step.key, a.historyIndex, a.model.Selected)
		}
	}
	if !reflect.DeepEqual(a.history, []string{"one", "removed", "two"}) {
		t.Fatalf("navigation rewrote history: %v", a.history)
	}
}

func TestMouseNavigationBlockedByDialogs(t *testing.T) {
	for _, dialog := range []string{"settings", "commands", "picker"} {
		t.Run(dialog, func(t *testing.T) {
			a := navigationFixture()
			switch dialog {
			case "settings":
				a.settingsOpen = true
			case "commands":
				a.commandsOpen = true
			case "picker":
				a.choosing = true
			}
			tt := ui.NewTester(a.content, 800, 600)
			tt.Key(0, ui.KeyBack)
			if a.historyIndex != 2 || a.model.Selected != "two" {
				t.Fatal("side button navigated behind a dialog")
			}
		})
	}
}

func TestMouseNavigationWithoutHistory(t *testing.T) {
	a := &application{cfg: preferences.Default(), historyIndex: -1}
	a.cfg.Language = "english"
	tt := ui.NewTester(a.content, 800, 600)
	tt.Key(0, ui.KeyBack)
	tt.Key(0, ui.KeyForward)
	if a.historyIndex != -1 || a.model.Selected != "" {
		t.Fatal("empty history changed")
	}
}
