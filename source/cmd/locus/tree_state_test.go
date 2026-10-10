package main

import (
	"path/filepath"
	"reflect"
	"testing"

	"github.com/egoist/mygo/ui"
	"github.com/oneo-me/locus/internal/library"
	"github.com/oneo-me/locus/internal/preferences"
)

func expansionFixture(root string) *application {
	a := &application{cfg: preferences.Default()}
	a.cfg.Language = "english"
	a.model.Root = root
	a.model.Notes = []*library.Note{
		{Path: "parent", Children: []*library.Note{
			{Path: "parent/child", Children: []*library.Note{{Path: "parent/child/leaf"}}},
		}},
		{Path: "other", Children: []*library.Note{{Path: "other/leaf"}}},
	}
	a.resetTree()
	return a
}

func TestTreeExpansionRoundTrip(t *testing.T) {
	a := expansionFixture("/library/one")
	a.restoreTreeExpansion()
	a.tree.Open.Add("parent/child") // Retain a hidden child's state.
	a.tree.Open.Add("other")
	a.rememberTreeExpansion()
	path := filepath.Join(t.TempDir(), "state.yaml")
	if err := preferences.SaveState(path, a.state); err != nil {
		t.Fatal(err)
	}
	b := expansionFixture(a.model.Root)
	var err error
	b.state, err = preferences.LoadState(path)
	if err != nil {
		t.Fatal(err)
	}
	b.restoreTreeExpansion()
	if b.tree.Open.Has("parent") || !b.tree.Open.Has("parent/child") || !b.tree.Open.Has("other") {
		t.Fatal("expansion was not restored exactly")
	}
	b.focusNote("parent/child/leaf")
	if b.tree.Open.Has("parent") || b.cursor != -1 || b.model.Selected != "parent/child/leaf" {
		t.Fatal("restoring selection opened a collapsed ancestor")
	}
	b.selectNote("parent/child/leaf")
	if !b.tree.Open.Has("parent") || b.cursor != 2 {
		t.Fatal("explicit navigation must reveal the note")
	}
}

func TestTreeExpansionLibraryIsolationAndRefresh(t *testing.T) {
	a := expansionFixture("/library/one")
	a.restoreTreeExpansion()
	a.tree.Open.Add("parent")
	a.tree.Open.Add("parent/child")
	a.rememberTreeExpansion()
	a.resetTree()
	a.model.Root = "/library/two"
	a.restoreTreeExpansion()
	if a.tree.Open.Len() != 0 {
		t.Fatal("another library inherited expansion")
	}
	a.rememberTreeExpansion() // All collapsed is a saved state.
	a.resetTree()
	a.model.Root = "/library/one"
	a.restoreTreeExpansion()
	if !a.tree.Open.Has("parent") || !a.tree.Open.Has("parent/child") {
		t.Fatal("switching back lost expansion")
	}
	a.model.Notes[0].Children[0].Children = nil
	a.restoreTreeExpansion()
	if !a.tree.Open.Has("parent") || a.tree.Open.Has("parent/child") {
		t.Fatal("refresh did not retain valid branches and discard removed branches")
	}
	a.rememberTreeExpansion()
	want := map[string][]string{"/library/one": {"parent"}, "/library/two": nil}
	if !reflect.DeepEqual(a.state.ExpandedNodes, want) {
		t.Fatalf("saved expansion: %#v", a.state.ExpandedNodes)
	}
	a.resetTree()
	a.rememberTreeExpansion()
	if !reflect.DeepEqual(a.state.ExpandedNodes, want) {
		t.Fatal("an unfinished open overwrote saved state")
	}
}

func TestTreeKeyboardExpansionPersists(t *testing.T) {
	a := expansionFixture("/library")
	a.restoreTreeExpansion()
	a.cursor = 0
	tt := ui.NewTester(a.left, 252, 600)
	tt.Click("Document tree")
	tt.Key(0, ui.KeyRight)
	a.rememberTreeExpansion()
	if !a.tree.Open.Has("parent") {
		t.Fatal("keyboard did not expand parent")
	}
	tt.Key(0, ui.KeyLeft)
	a.rememberTreeExpansion()
	if len(a.state.ExpandedNodes[a.model.Root]) != 0 {
		t.Fatal("keyboard collapse was not saved")
	}
}
