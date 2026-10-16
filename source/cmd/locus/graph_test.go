package main

import (
	"slices"
	"testing"

	"github.com/egoist/mygo/ui"
	"github.com/oneo-me/locus/internal/library"
	"github.com/oneo-me/locus/internal/preferences"
	"github.com/oneo-me/onevra/source/components/graph"
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
	a.graphAttachments = []library.Attachment{{Note: "guide", Name: "setup.png"}}
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

func TestGraphDataShowsAttachmentsWhenEnabled(t *testing.T) {
	a := graphFixture()
	a.state.GraphAttachments = true
	data := a.graphData()
	if len(data.Nodes) != 5 {
		t.Fatalf("nodes = %d, want 5", len(data.Nodes))
	}
	byID := map[string]graph.Node{}
	for _, node := range data.Nodes {
		byID[node.ID] = node
	}
	attachment, ok := byID["guide/assets/setup.png"]
	if !ok {
		t.Fatal("missing attachment node")
	}
	if attachment.Openable || attachment.Label != "setup.png" || attachment.Group != graphAttachmentGroup {
		t.Fatalf("attachment node = %+v", attachment)
	}
	if attachment.Color != graphAttachmentFill || attachment.LabelColor != graphAttachmentLabel {
		t.Fatalf("attachment node color = %+v", attachment)
	}
	found := false
	for _, edge := range data.Edges {
		if edge.Source == "guide" && edge.Target == "guide/assets/setup.png" {
			found = true
		}
	}
	if !found {
		t.Fatal("missing note-to-attachment edge")
	}
	a.state.GraphAttachments = false
	a.graphDirty = true
	if got := len(a.graphData().Nodes); got != 4 {
		t.Fatalf("nodes after hiding attachments = %d, want 4", got)
	}
}

func TestGraphOverlayTogglesAttachments(t *testing.T) {
	a := graphFixture()
	a.graphOpen = true
	tt := ui.NewTester(a.graphDialog, 900, 700)
	tt.SetPreferences(ui.Preferences{ReduceMotion: true})
	tt.Frame()
	if tt.HasText("Double-click a note to open it.") {
		t.Fatal("the double-click hint should be removed")
	}
	close, ok := tt.Find("Close")
	if !ok {
		t.Fatal("graph overlay is missing its close control")
	}
	if close.X < 600 || close.Y > 160 {
		t.Fatalf("close control is not in the window's top-right: %+v", close)
	}
	if _, ok := tt.Find("Relation graph options"); !ok {
		t.Fatal("graph overlay is missing its options menu")
	}
	if err := tt.Click("Relation graph options"); err != nil {
		t.Fatal(err)
	}
	if got := tt.Menu(); !slices.Equal(got, []string{"Show attachments"}) {
		t.Fatalf("menu = %v, want [Show attachments]", got)
	}
	if err := tt.ChooseMenuItem("Show attachments"); err != nil {
		t.Fatal(err)
	}
	if !a.state.GraphAttachments {
		t.Fatal("choosing Show attachments did not enable attachments")
	}
}

func TestGraphUsesFixedDarkPalette(t *testing.T) {
	palette := graphDarkPalette()
	dark := graph.DefaultPalette(true)
	if palette.Node != dark.Node || palette.NodeLabel != dark.NodeLabel {
		t.Fatalf("palette = %+v, want dark defaults", *palette)
	}
	if palette.Background != (ui.Color{}) {
		t.Fatalf("graph background = %+v, want transparent", palette.Background)
	}
	for count := 1; count <= 24; count++ {
		for index := 0; index < count; index++ {
			if hue := graphGroupHue(index, count); hue >= graphAttachmentHueStart && hue < graphAttachmentHueEnd {
				t.Fatalf("group hue %v for %d/%d is in the attachment yellow band", hue, index, count)
			}
		}
	}
	if fill, _ := graphGroupColor("guide", 0, 1, false); fill == graphAttachmentFill {
		t.Fatal("a group color matches the attachment yellow")
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

func TestGraphDialogRendersAndOpensNotes(t *testing.T) {
	a := graphFixture()
	a.graphOpen = true
	tt := ui.NewTester(a.graphDialog, 900, 700)
	tt.SetPreferences(ui.Preferences{ReduceMotion: true})
	tt.Frame()
	canvas, ok := tt.Find("Relation graph")
	if !ok {
		t.Fatal("graph dialog is missing its accessible label")
	}
	if canvas.W < 880 || canvas.H < 680 {
		t.Fatalf("graph does not fill the window: %+v", canvas)
	}
	if _, ok := tt.Find("Close"); !ok {
		t.Fatal("graph dialog is missing its close control")
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
