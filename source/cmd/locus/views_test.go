package main

import (
	"testing"

	"github.com/egoist/mygo/ui"
	"github.com/oneo-me/locus/internal/library"
	"github.com/oneo-me/locus/internal/preferences"
)

func TestSidebarSearchOpensCommands(t *testing.T) {
	a := &application{cfg: preferences.Default()}
	a.cfg.Language = "english"
	tt := ui.NewTester(a.left, 252, 600)
	if err := tt.Click("Search notes"); err != nil {
		t.Fatal(err)
	}
	if !a.commandsOpen {
		t.Fatal("search did not open command palette")
	}
	tt = ui.NewTester(a.title, 650, 48)
	if !tt.HasText("Locus") {
		t.Fatal("missing content title")
	}
	for _, label := range []string{"Back", "Forward", "Search notes"} {
		if _, ok := tt.Find(label); ok {
			t.Fatalf("%s still in content header", label)
		}
	}
}

func TestSidebarLayout(t *testing.T) {
	a := &application{cfg: preferences.Default(), historyIndex: -1}
	a.cfg.Language = "english"
	a.model.Root = "/private/library"
	a.model.Notes = []*library.Note{{Path: "one", Title: "First note", HasBody: true, Modified: true}, {Path: "two", Title: "Second note"}}
	a.resetTree()
	tt := ui.NewTester(a.left, 252, 600)
	for _, label := range []string{"Open library", a.model.Root} {
		if tt.HasText(label) {
			t.Fatalf("sidebar exposes %s", label)
		}
	}
	search, ok := tt.Find("Search notes")
	if !ok || search.Y < 48 || search.X+search.W > 252 {
		t.Fatalf("search must fit below the sidebar header: %+v", search)
	}
	tree, ok := tt.Find("Document tree")
	if !ok || tree.X != 0 || tree.W != 252 {
		t.Fatalf("tree viewport must reach sidebar edges: %+v", tree)
	}
	marker, ok := tt.Find("Modified files")
	if !ok || marker.W != 5 || marker.H != 5 {
		t.Fatalf("expected a small fixed-size modification marker: %+v", marker)
	}
	first, ok := tt.Find("First note")
	if !ok {
		t.Fatal("missing first row")
	}
	second, ok := tt.Find("Second note")
	if !ok || second.Y-first.Y < 28 || second.Y-first.Y >= 36 {
		t.Fatalf("unexpected compact tree spacing: %+v %+v", first, second)
	}
}

func TestHeaderMoreActions(t *testing.T) {
	for _, canvas := range []string{"", "html"} {
		t.Run("canvas="+canvas, func(t *testing.T) {
			a := &application{cfg: preferences.Default()}
			a.cfg.Language = "english"
			a.model.Notes = []*library.Note{{Path: "note", Title: "Note", HasBody: true, Canvas: canvas}}
			a.model.Selected = "note"
			tt := ui.NewTester(a.title, 650, 48)
			more, ok := tt.Find("More actions")
			if !ok {
				t.Fatal("missing more actions menu")
			}
			for _, label := range []string{"Copy reference", "Edit note", "Open directory"} {
				button, ok := tt.Find(label)
				if !ok || button.X+button.W > more.X {
					t.Fatalf("expected %s before more actions: %+v", label, button)
				}
			}
			for _, label := range []string{"Archive note", "Export canvas"} {
				if _, ok := tt.Find(label); ok {
					t.Fatalf("%s should be inside the menu, not a header button", label)
				}
			}
		})
	}
}

func TestSidebarFooter(t *testing.T) {
	a := &application{cfg: preferences.Default(), settingsOpen: true}
	a.cfg.Language = "english"
	a.settingState.Query = "old query"
	tt := ui.NewTester(func(c *ui.Context) { ui.Toolbar(c, func() { a.toolbar(c) }).FillWidth() }, 252, 48)
	for _, label := range []string{"Refresh", "Open Agent"} {
		if _, ok := tt.Find(label); ok {
			t.Fatalf("unexpected footer action %s", label)
		}
	}
	theme, ok := tt.Find("Theme")
	if !ok {
		t.Fatal("missing theme menu")
	}
	mcp, ok := tt.Find("MCP")
	if !ok || mcp.X+mcp.W > theme.X || theme.X < 210 {
		t.Fatalf("expected MCP before trailing theme button: MCP=%+v theme=%+v", mcp, theme)
	}
	if mcp.W != theme.W || mcp.H != theme.H {
		t.Fatal("MCP and theme should have matching icon-button hit targets")
	}
	if err := tt.Click("MCP"); err != nil {
		t.Fatal(err)
	}
	if a.settingState.Category != "mcp" || a.settingState.Query != "" {
		t.Fatal("MCP did not select unfiltered settings category")
	}
}
