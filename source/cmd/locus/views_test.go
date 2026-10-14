package main

import (
	"strings"
	"testing"

	"go.yaml.in/yaml/v3"

	"github.com/egoist/mygo/ui"
	"github.com/oneo-me/locus/internal/library"
	"github.com/oneo-me/locus/internal/preferences"
	"github.com/oneo-me/onevra/source/shells/sidebar"
	"github.com/oneo-me/onevra/source/theme"
)

func TestArticleHeaderFollowsScroll(t *testing.T) {
	for _, dark := range []bool{false, true} {
		name := "light"
		if dark {
			name = "dark"
		}
		t.Run(name, func(t *testing.T) {
			a := &application{cfg: preferences.Default()}
			a.model.Root, a.model.Selected = "library", "one"
			a.model.Notes = []*library.Note{{Path: "one", Title: "Article"}, {Path: "two", Title: "Article"}}
			a.model.Document.Body = strings.Repeat("A paragraph to scroll.\n\n", 80)
			var headerHeight float32
			var border ui.Color
			tt := ui.NewTester(func(c *ui.Context) {
				c.SetTheme(theme.Palette(dark))
				headerHeight, border = sidebar.HeaderHeight(c), c.Theme().Border
				ui.Column(c).Fill().Children(func() { a.content(c) })
			}, 650, 400)
			top := tt.Image().RGBAAt(325, 20)
			topEdge := tt.Image().RGBAAt(325, int(headerHeight)-1)
			checkScrolled := func() {
				t.Helper()
				tt.Scroll(325, 200, 0, 120)
				if a.articleScroll.Y <= 0 {
					t.Fatal("article did not scroll")
				}
				im := tt.Image()
				if im.RGBAAt(325, 20) == top {
					t.Fatal("header did not change after scrolling")
				}
				edge := im.RGBAAt(325, int(headerHeight)-1)
				if edge.R != border.R || edge.G != border.G || edge.B != border.B {
					t.Fatalf("missing bottom border: got %v, want %v", edge, border)
				}
			}
			checkTop := func() {
				t.Helper()
				tt.Frame()
				if a.articleScroll.Y != 0 || tt.Image().RGBAAt(325, 20) != top || tt.Image().RGBAAt(325, int(headerHeight)-1) != topEdge {
					t.Fatal("header did not return to its transparent top state")
				}
			}
			checkScrolled()
			tt.Scroll(325, 200, 0, -120)
			checkTop()
			checkScrolled()
			a.model.Selected = "two"
			checkTop()
			checkScrolled()
			a.model.Root = "other-library"
			checkTop()
		})
	}
}

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
				t.Fatal("more actions menu should appear for every note")
			}
			for _, label := range []string{"Copy reference", "Edit note", "Open directory", "Terminal"} {
				button, ok := tt.Find(label)
				if !ok || button.X+button.W > more.X {
					t.Fatalf("expected %s before more actions: %+v", label, button)
				}
			}
			for _, label := range []string{"Upload icon", "Export canvas"} {
				if _, ok := tt.Find(label); ok {
					t.Fatalf("%s should be inside the menu, not a header button", label)
				}
			}
		})
	}
}

func TestSettingsWithoutArchive(t *testing.T) {
	for _, language := range languageOptions {
		if language == "system" {
			continue
		}
		t.Run(language, func(t *testing.T) {
			a := &application{cfg: preferences.Default()}
			a.cfg.Language = language
			tt := ui.NewTester(func(c *ui.Context) { a.settings(c, false) }, 900, 700)
			for _, label := range []string{a.text("General"), a.text("Appearance"), a.text("Agents"), "MCP"} {
				if !tt.HasText(label) {
					t.Fatalf("missing settings category %s", label)
				}
			}
			for _, label := range []string{"Archived notes", "已归档笔记", "Restore", "恢复"} {
				if tt.HasText(label) {
					t.Fatalf("settings still exposes %s", label)
				}
			}
		})
	}
}

func TestArchivedMetadataIsCustomProperty(t *testing.T) {
	var node yaml.Node
	if err := yaml.Unmarshal([]byte("title: Note\narchived: true\n"), &node); err != nil {
		t.Fatal(err)
	}
	tt := ui.NewTester(func(c *ui.Context) { noteMetadataPanel(c, "", &node, c.Theme().Surface) }, 500, 300)
	if !tt.HasText("archived") || !tt.HasText("true") {
		t.Fatal("legacy archived field should be displayed as custom metadata")
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
