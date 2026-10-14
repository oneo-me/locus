package main

import (
	"testing"

	"github.com/egoist/mygo/ui"
	"github.com/oneo-me/locus/internal/library"
	"github.com/oneo-me/locus/internal/preferences"
)

func graphFixture() *application {
	a := &application{cfg: preferences.Default(), historyIndex: -1}
	a.cfg.Language = "english"
	a.model.Root = "library"
	a.model.Dispatch = func(func()) {}
	a.model.Notes = []*library.Note{
		{Path: "index", Title: "Index", HasBody: true},
		{Path: "guide", Title: "Guide", HasBody: true, Children: []*library.Note{
			{Path: "guide/setup", Title: "Setup", HasBody: true},
		}},
		{Path: "reference", Title: "Reference", HasBody: true},
	}
	a.graphLinks = []library.Relation{{Source: "guide", Target: "reference"}}
	a.graphDirty = true
	a.resetTree()
	return a
}

func TestGraphDataProjectsHierarchyAndLinks(t *testing.T) {
	a := graphFixture()
	data := a.graphData()
	if len(data.Nodes) != 4 {
		t.Fatalf("nodes = %d, want 4", len(data.Nodes))
	}
	byID := map[string]int{}
	for i, node := range data.Nodes {
		byID[node.ID] = i
		if !node.Openable {
			t.Fatalf("node %s is not openable", node.ID)
		}
	}
	if _, ok := byID["guide/setup"]; !ok {
		t.Fatal("missing nested node")
	}
	if got := data.Nodes[byID["guide"]].Group; got != "" {
		t.Fatalf("top-level group = %q, want empty", got)
	}
	if got := data.Nodes[byID["guide/setup"]].Group; got != "guide" {
		t.Fatalf("nested group = %q, want guide", got)
	}
	edges := map[[2]string]float32{}
	for _, edge := range data.Edges {
		edges[[2]string{edge.Source, edge.Target}] = edge.Style.Opacity
	}
	if _, ok := edges[[2]string{"guide", "guide/setup"}]; !ok {
		t.Fatal("missing hierarchy edge")
	}
	if opacity, ok := edges[[2]string{"guide", "reference"}]; !ok || opacity != 0 {
		t.Fatalf("link edge = %v, %v; want default opacity", opacity, ok)
	}
	if a.graphDirty {
		t.Fatal("graph data cache was not filled")
	}
	if again := a.graphData(); len(again.Nodes) != len(data.Nodes) {
		t.Fatal("cached data changed")
	}
}

func TestGraphToolbarTogglesPage(t *testing.T) {
	a := graphFixture()
	tt := ui.NewTester(func(c *ui.Context) { ui.Toolbar(c, func() { a.toolbar(c) }).FillWidth() }, 252, 48)
	if _, ok := tt.Find("Relation graph"); !ok {
		t.Fatal("missing relation graph toolbar entry")
	}
	if err := tt.Click("Relation graph"); err != nil {
		t.Fatal(err)
	}
	if !a.graphOpen {
		t.Fatal("toolbar entry did not open the graph")
	}
	if err := tt.Click("Relation graph"); err != nil {
		t.Fatal(err)
	}
	if a.graphOpen {
		t.Fatal("second activation did not close the graph")
	}

	a.model.Root = ""
	tt = ui.NewTester(func(c *ui.Context) { ui.Toolbar(c, func() { a.toolbar(c) }).FillWidth() }, 252, 48)
	if err := tt.Click("Relation graph"); err != nil {
		t.Logf("disabled entry is not interactive: %v", err)
	}
	if a.graphOpen {
		t.Fatal("disabled graph entry opened the page")
	}
}

func TestGraphPageRendersAndOpensNotes(t *testing.T) {
	a := graphFixture()
	a.graphOpen = true
	tt := ui.NewTester(a.graphPage, 900, 700)
	tt.SetPreferences(ui.Preferences{ReduceMotion: true})
	tt.Frame()
	if _, ok := tt.Find("Relation graph"); !ok {
		t.Fatal("graph page is missing its accessible label")
	}
	if _, ok := tt.Find("Close"); !ok {
		t.Fatal("graph page is missing its close control")
	}
	a.openGraphNote("guide/setup")
	if a.graphOpen || a.model.Selected != "guide/setup" {
		t.Fatalf("opening a node: open=%v selected=%q", a.graphOpen, a.model.Selected)
	}
	a.openGraphNote("missing")
	if a.graphOpen || a.model.Selected != "guide/setup" {
		t.Fatal("an unknown node should be ignored")
	}
}

func TestGraphCommandEntry(t *testing.T) {
	a := graphFixture()
	a.commandsOpen = true
	tt := ui.NewTester(a.content, 900, 700)
	tt.SetPreferences(ui.Preferences{ReduceMotion: true})
	tt.Frame()
	if !tt.HasText("Relation graph") {
		t.Fatal("command palette is missing the relation graph entry")
	}
}
