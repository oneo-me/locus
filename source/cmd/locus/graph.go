package main

import (
	"math"
	"strings"

	"github.com/egoist/mygo/ui"
	"github.com/oneo-me/locus/internal/library"
	"github.com/oneo-me/onevra/source/components"
	"github.com/oneo-me/onevra/source/components/graph"
	"github.com/oneo-me/onevra/source/icons/lucide"
	"github.com/oneo-me/onevra/source/theme"
)

// graphHierarchyOpacity keeps structural parent-child edges behind explicit
// note links while keeping the tree visible.
const graphHierarchyOpacity = 0.12

// graphAttachmentOpacity keeps a note's files visible without competing with
// note-to-note links.
const graphAttachmentOpacity = 0.35

// graphAttachmentGroup is a reserved group name. Note paths never contain an
// assets segment, so attachments start in their own sector of the layout.
const graphAttachmentGroup = "assets"

// graphBackdropOpacity dims the workspace deeply so the graph stays legible
// over the content behind it, while a little translucency still shows through.
const graphBackdropOpacity = 0.82

// Attachment nodes use yellow rather than a group color so files read as a
// distinct kind, independent of the note groups around them. Graph groups skip
// the matching hue band so no note group can be mistaken for a file.
var (
	graphAttachmentFill  = ui.Hex("#f0c23c")
	graphAttachmentLabel = ui.Hex("#f7e7a9")
)

const (
	graphAttachmentHueStart = 28.0
	graphAttachmentHueEnd   = 96.0
)

// graphDarkPalette returns the graph's fixed dark palette. The graph ignores
// the workspace appearance and always uses the dark palette; the background
// stays transparent so the dialog's dimmed backdrop shows through.
func graphDarkPalette() *graph.Palette {
	palette := graph.DefaultPalette(true)
	return &palette
}

// graphGroupColor assigns the fixed dark graph's group colors. It reserves the
// attachment yellow band and keeps fills as saturated mid-tones, so note
// groups stay distinct over the dimmed workspace instead of washing out.
func graphGroupColor(group string, index, count int, _ bool) (ui.Color, ui.Color) {
	hue := graphGroupHue(index, count)
	return graphHSL(hue, 66, 58), graphHSL(hue, 36, 85)
}

// graphGroupHue spaces group hues evenly around the wheel while skipping the
// attachment yellow band. The allowed arc starts just after the band and wraps
// through 0, so no two samples can collide at the seam.
func graphGroupHue(index, count int) float64 {
	const firstHue = 205.0
	span := 360 - (graphAttachmentHueEnd - graphAttachmentHueStart)
	start := math.Mod(firstHue-graphAttachmentHueEnd, span)
	t := math.Mod(start+float64(index)*span/float64(max(1, count)), span)
	return math.Mod(graphAttachmentHueEnd+t, 360)
}

// graphHSL converts an HSL color to sRGB. The graph package keeps its own
// conversion private, but the graph needs a hue range that skips attachment
// yellow.
func graphHSL(hue, saturation, lightness float64) ui.Color {
	h := math.Mod(math.Mod(hue, 360)+360, 360) / 60
	s := max(0, min(saturation/100, 1))
	l := max(0, min(lightness/100, 1))
	chroma := (1 - math.Abs(2*l-1)) * s
	second := chroma * (1 - math.Abs(math.Mod(h, 2)-1))
	var r, g, b float64
	switch {
	case h < 1:
		r, g = chroma, second
	case h < 2:
		r, g = second, chroma
	case h < 3:
		g, b = chroma, second
	case h < 4:
		g, b = second, chroma
	case h < 5:
		r, b = second, chroma
	default:
		r, b = chroma, second
	}
	m := l - chroma/2
	channel := func(v float64) uint8 { return uint8(math.Round((v + m) * 255)) }
	return ui.RGB(channel(r), channel(g), channel(b))
}

// toggleGraph shows the relation graph as a full-window dialog over the
// workspace. Opening resets the camera and recomputes link edges.
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
}

func (a *application) closeGraph() {
	if !a.graphOpen {
		return
	}
	a.graphOpen = false
}

// syncPage selects Shell's replacement page: the library picker without a
// library, and the default workspace otherwise. The relation graph is a
// dialog over the workspace, not a page.
func (a *application) syncPage() {
	if a.shell == nil {
		return
	}
	if a.model.Root == "" {
		a.shell.SetPage(a.libraryPicker)
		return
	}
	a.shell.SetPage(nil)
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
	a.graphAttachments = nil
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
		attachments := lib.Attachments(notes)
		lib.Close()
		a.post(func() {
			if ticket == a.graphTicket && a.model.Root == root {
				a.graphLinks = relations
				a.graphAttachments = attachments
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

// graphDialog shows the relation graph full-window over a dimmed backdrop,
// exactly like the image preview. It covers Shell's floating header, so its
// display options and close control float in the window's top trailing corner.
func (a *application) graphDialog(c *ui.Context) {
	if !a.graphOpen {
		return
	}
	if a.model.Root == "" {
		a.graphOpen = false
		return
	}
	ui.DialogBase(c, &a.graphOpen, func(backdrop, panel ui.Element) {
		backdrop.Background(ui.RGBA(0, 0, 0, graphBackdropOpacity))
		panel.Fill().Children(func() {
			graph.View(c, &a.graphState, a.graphData(), graph.Options{
				Label:      a.text("Relation graph"),
				Open:       a.openGraphNote,
				Palette:    graphDarkPalette(),
				GroupColor: graphGroupColor,
				Empty: func(c *ui.Context) {
					components.Text(c, a.text("No notes to show.")).TextColor(theme.DarkPalette().TextMuted)
				},
			})
		})
		backdrop.Children(func() {
			ui.Box(c).Absolute().Top(16).Right(16).Children(func() {
				ui.Row(c).AlignItems(ui.Center).Gap(8).Children(func() {
					floatingDiscButton(c, lucide.SlidersHorizontal(), a.text("Relation graph options")).Menu(a.graphOptionsMenu)
					floatingDiscButton(c, lucide.X(), a.text("Close")).OnClick(a.closeGraph)
				})
			})
		})
	})
}

// graphOptionsMenu toggles the note attachments that appear in the graph.
func (a *application) graphOptionsMenu(m *ui.Menu) {
	if m.Item(a.text("Show attachments")).Checked(a.state.GraphAttachments).Chosen() {
		a.state.GraphAttachments = !a.state.GraphAttachments
		a.graphDirty = true
	}
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
	seen := make(map[edgeKey]bool, len(edges)+len(a.graphLinks)+len(a.graphAttachments))
	for _, edge := range edges {
		seen[edgeKey{edge.Source, edge.Target}] = true
	}
	if a.state.GraphAttachments {
		for _, attachment := range a.graphAttachments {
			if !known[attachment.Note] {
				continue
			}
			id := graphAttachmentID(attachment.Note, attachment.Name)
			nodes = append(nodes, graph.Node{
				ID: id, Label: attachment.Name, Group: graphAttachmentGroup,
				Color: graphAttachmentFill, LabelColor: graphAttachmentLabel,
			})
			edges = append(edges, graph.Edge{Source: attachment.Note, Target: id, Style: graph.EdgeStyle{Opacity: graphAttachmentOpacity}})
			seen[edgeKey{attachment.Note, id}] = true
		}
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

// graphAttachmentID addresses an attachment inside its note. Note paths never
// contain an assets segment, so the ID cannot collide with a note path.
func graphAttachmentID(note, name string) string {
	return note + "/assets/" + name
}
