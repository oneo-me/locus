package main

import (
	"strings"

	"github.com/egoist/mygo/ui"
	"github.com/oneo-me/locus/internal/library"
	"github.com/oneo-me/onevra/source/components"
	"github.com/oneo-me/onevra/source/components/graph"
	"github.com/oneo-me/onevra/source/icons/lucide"
	"github.com/oneo-me/onevra/source/shells/sidebar"
)

// graphHierarchyOpacity keeps structural parent-child edges behind explicit
// note links while keeping the tree visible.
const graphHierarchyOpacity = 0.12

// toggleGraph shows the relation graph on Shell's replacement page, next to
// the library picker. Opening resets the camera and recomputes link edges.
func (a *application) toggleGraph() {
	if a.graphOpen {
		a.closeGraph()
		return
	}
	if a.model.Root == "" {
		return
	}
	a.graphOpen = true
	a.graphState = graph.State{}
	a.loadGraphRelations()
	a.syncPage()
}

func (a *application) closeGraph() {
	if !a.graphOpen {
		return
	}
	a.graphOpen = false
	a.syncPage()
}

// syncPage selects Shell's replacement page: the relation graph when open, the
// library picker without a library, and the default workspace otherwise.
func (a *application) syncPage() {
	if a.shell == nil {
		return
	}
	switch {
	case a.graphOpen:
		a.shell.SetPage(a.graphPage)
	case a.model.Root == "":
		a.shell.SetPage(a.libraryPicker)
	default:
		a.shell.SetPage(nil)
	}
}

// graphIconButton matches the sidebar icon buttons while using a smaller glyph:
// Onevra fixes the icon size, and the network glyph fills its viewbox more than
// the settings and theme glyphs do.
func graphIconButton(c *ui.Context, icon *ui.SVG, label string) ui.Element {
	t := c.Theme()
	size := max(32, t.Rem(2))
	b := ui.ButtonBase(c).Size(size, size).Radius(t.Radius).TextColor(t.TextMuted).Label(label)
	b.Draw(func(p *ui.Painter, r ui.Rect) {
		if b.IsDisabled() {
			return
		}
		if b.Pressed() {
			p.Fill(r, t.Text.Alpha(0.14), t.Radius)
		} else if b.Hovered() {
			p.Fill(r, t.Text.Alpha(0.08), t.Radius)
		}
	}).Children(func() {
		components.Icon(c, icon).Size(16, 16)
	})
	components.Tooltip(c, b, label)
	return b
}

// invalidateGraphRelations forces the next load to recompute links and drops
// any in-flight result from the previous scan.
func (a *application) invalidateGraphRelations() {
	a.graphLinksKey = ""
	a.graphTicket++
}

// loadGraphRelations reads link edges in the background once per library scan.
// The hierarchy and nodes stay available immediately; link edges appear when
// the read finishes.
func (a *application) loadGraphRelations() {
	root := a.model.Root
	if root == "" || a.graphLinksKey == root {
		return
	}
	a.graphLinksKey = root
	a.graphLinks = nil
	a.graphDirty = true
	a.graphTicket++
	ticket := a.graphTicket
	notes := a.model.Notes
	go func() {
		lib, err := library.Open(root)
		if err != nil {
			return
		}
		relations := lib.Relations(notes)
		lib.Close()
		a.post(func() {
			if ticket == a.graphTicket && a.model.Root == root {
				a.graphLinks = relations
				a.graphDirty = true
				a.shell.Window().Invalidate()
			}
		})
	}()
}

// openGraphNote leaves the graph and selects the double-clicked note.
func (a *application) openGraphNote(p string) {
	if library.Find(a.model.Notes, p) == nil {
		return
	}
	a.graphOpen = false
	a.navigate(p)
	a.selectNote(p)
}

// graphPage is Shell's replacement page for the relation graph. It covers the
// whole window, hides the sidebar, and keeps its own close control.
func (a *application) graphPage(c *ui.Context) {
	a.flushNotices(c)
	inset := sidebar.HeaderHeight(c)
	ui.Column(c).Fill().MinWidth(0).MinHeight(0).Padding(inset, 0, 0, 0).Children(func() {
		graph.View(c, &a.graphState, a.graphData(), graph.Options{
			Label: a.text("Relation graph"),
			Open:  a.openGraphNote,
			Empty: func(c *ui.Context) {
				components.Text(c, a.text("No notes to show.")).TextColor(c.Theme().TextMuted)
			},
			Overlay: func(c *ui.Context) {
				ui.Row(c).AlignItems(ui.Center).Gap(8).Children(func() {
					components.Text(c, a.text("Double-click a note to open it.")).TextColor(c.Theme().TextMuted)
					sidebar.IconButton(c, lucide.X(), a.text("Close")).OnClick(a.closeGraph)
				})
			},
		})
	})
	a.commandDialog(c)
	a.settingsDialog(c)
}

// graphData projects the document tree and discovered links into nodes and
// edges. It is cached until the model or link results change.
func (a *application) graphData() graph.Data {
	if !a.graphDirty {
		return a.graphDataCache
	}
	var nodes []graph.Node
	var edges []graph.Edge
	known := map[string]bool{}
	var walk func([]*library.Note, string)
	walk = func(notes []*library.Note, parent string) {
		for _, n := range notes {
			known[n.Path] = true
			nodes = append(nodes, graph.Node{ID: n.Path, Label: n.Title, Group: graphGroup(n.Path), Openable: true})
			if parent != "" {
				edges = append(edges, graph.Edge{Source: parent, Target: n.Path, Style: graph.EdgeStyle{Opacity: graphHierarchyOpacity}})
			}
			walk(n.Children, n.Path)
		}
	}
	walk(a.model.Notes, "")
	type edgeKey struct{ source, target string }
	seen := make(map[edgeKey]bool, len(edges)+len(a.graphLinks))
	for _, edge := range edges {
		seen[edgeKey{edge.Source, edge.Target}] = true
	}
	for _, relation := range a.graphLinks {
		key := edgeKey{relation.Source, relation.Target}
		if seen[key] || !known[key.source] || !known[key.target] {
			continue
		}
		seen[key] = true
		edges = append(edges, graph.Edge{Source: relation.Source, Target: relation.Target})
	}
	a.graphDataCache = graph.Data{Nodes: nodes, Edges: edges}
	a.graphDirty = false
	return a.graphDataCache
}

// graphGroup clusters top-level directories so the graph separates sections.
func graphGroup(p string) string {
	if i := strings.IndexByte(p, '/'); i >= 0 {
		return p[:i]
	}
	return ""
}
