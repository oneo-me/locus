package main

import (
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"

	"github.com/egoist/mygo"
	"github.com/egoist/mygo/ui"
	"github.com/oneo-me/locus/internal/library"
	"github.com/oneo-me/locus/internal/terminal"
	"github.com/oneo-me/onevra/components"
	"github.com/oneo-me/onevra/icons/lucide"
	"github.com/oneo-me/onevra/plugins/command"
	"github.com/oneo-me/onevra/plugins/markdown"
	"github.com/oneo-me/onevra/shells/sidebar"
)

func paths(notes []*library.Note) []string {
	if len(notes) == 0 {
		return nil
	}
	out := make([]string, len(notes))
	for i, n := range notes {
		out[i] = n.Path
	}
	return out
}
func (a *application) left(c *ui.Context) {
	a.index = map[string]*library.Note{}
	a.children = map[string][]string{}
	a.roots = paths(a.model.Notes)
	var project func([]*library.Note)
	project = func(notes []*library.Note) {
		for _, n := range notes {
			a.index[n.Path] = n
			a.children[n.Path] = paths(n.Children)
			project(n.Children)
		}
	}
	project(a.model.Notes)
	ui.Column(c).Fill().MinHeight(0).Padding(sidebar.HeaderHeight(c), 0, 0, 0).Gap(8).Children(func() {
		ui.Box(c).FillWidth().Shrink(0).Padding(0, 8).Children(func() { a.search(c) })
		components.TreeVirtualized(c, &a.tree, a.roots, func(p string) []string { return a.children[p] }, func(p string) {
			if n := a.index[p]; n != nil {
				ui.Row(c).Grow(1).MinWidth(0).MinHeight(20).AlignItems(ui.Center).Gap(8).Children(func() {
					a.noteIcon(c, n, 14)
					components.Text(c, n.Title).SingleLine().Ellipsis("…").Grow(1).MinWidth(0)
					if n.Modified {
						ui.Box(c).Label(a.text("Modified files")).Size(5, 5).Radius(2.5).Background(c.Theme().Accent).Shrink(0)
					}
				})
			}
		}).Label("Document tree").FillWidth().Grow(1).MinHeight(0).Padding(0, 8, 8, 8)
	})
}
func (a *application) toolbar(c *ui.Context) {
	sidebar.IconButton(c, lucide.Settings(), a.text("Settings")).OnClick(a.toggleSettings)
	icon := lucide.Monitor()
	switch a.cfg.Theme {
	case "light":
		icon = lucide.Sun()
	case "dark":
		icon = lucide.Moon()
	}
	if a.model.Loading {
		components.Text(c, a.text("Loading…"))
	}
	ui.Spacer(c)
	button := sidebar.IconButton(c, lucide.Radio(), "MCP").OnClick(a.openMCPSettings)
	if a.mcpError != "" {
		button.TextColor(c.Theme().Danger)
	} else if a.service != nil {
		button.TextColor(c.Theme().Accent)
	}
	sidebar.IconButton(c, icon, a.text("Theme")).Menu(a.themeMenu)
}

func (a *application) themeMenu(m *ui.Menu) {
	for _, mode := range []string{"system", "light", "dark"} {
		if m.Item(a.text(mode)).Checked(a.cfg.Theme == mode).Chosen() {
			a.cfg.Theme = mode
			a.saveConfig()
			a.syncCanvasTheme()
		}
	}
}

func (a *application) openMCPSettings() {
	a.settingState.Category = "mcp"
	a.settingState.Query = ""
	if !a.settingsOpen {
		a.toggleSettings()
	}
}

func (a *application) navigation(c *ui.Context) {
	sidebar.IconButton(c, lucide.ArrowLeft(), a.text("Back")).Disabled(a.historyIndex <= 0).OnClick(func() { a.back(-1) })
	sidebar.IconButton(c, lucide.ArrowRight(), a.text("Forward")).Disabled(a.historyIndex+1 >= len(a.history)).OnClick(func() { a.back(1) })
}

func (a *application) search(c *ui.Context) {
	t := c.Theme()
	// Match Onevra TextBox styling while retaining command-trigger semantics.
	button := components.Button(c, "", components.ButtonGhost).Label(a.text("Search notes")).FillWidth().Shrink(0).MinWidth(0).Gap(t.Space(1.5))
	border := t.Border
	if button.Hovered() || button.Focused() {
		border = border.Mix(t.Text, 0.25)
	}
	button.Padding(t.Space(1.5), t.Space(2.5)).Radius(t.Radius).
		Background(t.Surface.Alpha(0.5)).Border(1, border).
		TextColor(t.TextMuted).OnClick(func() { a.commandsOpen = true }).Children(func() {
		components.Icon(c, lucide.Search).Size(16, 16).Shrink(0)
		components.Text(c, a.text("Search notes")).Grow(1).MinWidth(0).SingleLine().Ellipsis("…")
		shortcut := "Ctrl E"
		if runtime.GOOS == "darwin" {
			shortcut = "⌘ E"
		}
		components.Text(c, shortcut).Shrink(0)
	})
}

func (a *application) title(c *ui.Context) {
	ui.Row(c).FillWidth().MinWidth(0).AlignItems(ui.Center).Gap(4).Children(func() {
		note := library.Find(a.model.Notes, a.model.Selected)
		title := "Locus"
		if note != nil {
			title = note.Title
			ui.Box(c).Shrink(0).Padding(0, 4, 0, 0).Children(func() { a.noteIcon(c, note, 16) })
		}
		components.Text(c, title).Bold().SingleLine().Ellipsis("…").Grow(1).MinWidth(0)
		if note != nil && note.HasBody {
			sidebar.IconButton(c, lucide.Link(), a.text("Copy reference")).OnClick(func() { mygo.Clipboard.WriteText(library.Address(note.Path)) })
			sidebar.IconButton(c, lucide.Pencil(), a.text("Edit note")).OnClick(func() { a.fileAction(note.Path, true) })
			sidebar.IconButton(c, lucide.Folder(), a.text("Open directory")).OnClick(func() { a.fileAction(note.Path, false) })
			if note.Canvas != "" {
				sidebar.IconButton(c, lucide.Ellipsis(), a.text("More actions")).Menu(func(m *ui.Menu) {
					if m.Item(a.text("Export canvas")).Disabled(a.session == nil || a.exporting).Chosen() {
						a.exportCanvas()
					}
				})
			}
		}
	})
}
func (a *application) content(c *ui.Context) {
	// MyGo maps native mouse side buttons to these navigation keys.
	if !a.settingsOpen && !a.commandsOpen && !a.choosing {
		if c.Shortcut(0, ui.KeyBack) {
			a.back(-1)
		}
		if c.Shortcut(0, ui.KeyForward) {
			a.back(1)
		}
	}
	mods := ui.Ctrl
	if runtime.GOOS == "darwin" {
		mods = ui.Cmd
	}
	if c.Shortcut(0, ui.KeyF5) {
		a.refresh()
	}
	if c.Shortcut(mods, ui.KeyE) {
		a.commandsOpen = !a.commandsOpen
	}
	note := library.Find(a.model.Notes, a.model.Selected)
	inset := sidebar.HeaderHeight(c)
	messages := func() {
		for _, message := range []string{a.storageError, a.feedback, a.mcpError} {
			if message != "" {
				components.Text(c, message).Wrap().TextColor(c.Theme().Danger)
			}
		}
		if a.model.Error != nil {
			components.Text(c, a.model.Error.Error()).Wrap().TextColor(c.Theme().Danger)
		}
	}
	contentInset := inset
	if note != nil && note.Canvas == "" {
		contentInset = 0
	}
	ui.Column(c).Fill().MinWidth(0).MinHeight(0).Padding(contentInset, 0, 0, 0).Children(func() {
		if note == nil || note.Canvas != "" {
			messages()
		}
		if note == nil {
			return
		}
		if note.Canvas != "" {
			if note.Summary != "" {
				components.Text(c, note.Summary).FillWidth().MinWidth(0).Shrink(0).Padding(0, 16, 12, 16).Wrap()
				ui.Box(c).FillWidth().Height(1).Shrink(0).Background(c.Theme().Border)
			}
			if a.canvasError != "" {
				components.Text(c, a.canvasError).Wrap().TextColor(c.Theme().Danger)
			} else if a.web != nil && !a.settingsOpen {
				if err := a.web.Err(); err != nil {
					components.Text(c, err.Error()).Wrap()
				} else {
					a.web.View(c).Fill()
				}
			} else {
				components.Text(c, a.text("Loading…"))
			}
			return
		}
		if a.document == nil || a.renderedSource != a.model.Document.Body {
			a.renderedSource = a.model.Document.Body
			a.document = markdown.ParseMarkdown(a.renderedSource)
		}
		atmosphere := deriveNoteAtmosphere(note.Title, c.Theme().Dark)
		components.ScrollView(c, components.ScrollViewOptions{}).Key(note.Path).Fill().MinWidth(0).Children(func() {
			// Inset the content, not the scroll viewport, so its background reaches
			// behind Shell's floating titlebar without being clipped below it.
			ui.Column(c).FillWidth().MinWidth(0).MinHeight(atmosphereHeight).Padding(inset, 0, 0, 0).AlignItems(ui.Center).Draw(func(p *ui.Painter, r ui.Rect) {
				atmosphere.draw(p, r)
			}).Children(func() {
				messages()
				ui.Column(c).FillWidth().MaxWidth(noteMaxWidth).MinWidth(0).Padding(max(0, 56-inset), 16, 56, 16).Gap(22).Children(func() {
					noteMetadataPanel(c, note.Summary, a.model.Document.Metadata, atmosphere.panel.ui())
					a.document.Render(c, markdown.MarkdownOptions{OnLink: a.openLink, Images: a.images}).FillWidth()
				})
			})
		})
	})
	a.commandDialog(c)
	a.settingsDialog(c)
}
func (a *application) selectNote(p string) {
	parts := strings.Split(p, "/")
	for i := 1; i < len(parts); i++ {
		a.tree.Open.Add(strings.Join(parts[:i], "/"))
	}
	row := 0
	var walk func([]*library.Note)
	walk = func(notes []*library.Note) {
		for _, n := range notes {
			if n.Path == p {
				a.cursor = row
			}
			row++
			if a.tree.Open.Has(n.Path) {
				walk(n.Children)
			}
		}
	}
	walk(a.model.Notes)
	a.tree.List.ScrollIntoView(a.cursor)
	a.model.Select(p)
}
func (a *application) commandDialog(c *ui.Context) {
	if !a.commandsOpen {
		return
	}
	items := []command.Item{{ID: "@settings", Title: a.text("Settings")}, {ID: "@open", Title: a.text("Open library")}, {ID: "@refresh", Title: a.text("Refresh")}, {ID: "@agent", Title: a.text("Open Agent")}}
	var walk func([]*library.Note)
	walk = func(notes []*library.Note) {
		for _, n := range notes {
			items = append(items, command.Item{ID: "note/" + n.Path, Title: n.Title, Keywords: []string{n.Path, n.Summary}})
			walk(n.Children)
		}
	}
	walk(a.model.Notes)
	command.CommandDialog(c, &a.commandsOpen, &a.palette, items, command.Options{OnActivate: func(id string) {
		a.commandsOpen = false
		switch id {
		case "@settings":
			a.toggleSettings()
		case "@open":
			a.chooseLibrary()
		case "@refresh":
			a.refresh()
		case "@agent":
			a.launchAgent()
		default:
			p := strings.TrimPrefix(id, "note/")
			a.navigate(p)
			a.selectNote(p)
		}
	}}, components.DialogOptions{})
}
func (a *application) fileAction(note string, edit bool) {
	root, editor := a.model.Root, a.cfg.Editor
	go func() {
		lib, err := library.Open(root)
		if err == nil {
			var dir string
			dir, err = lib.ResolvePath(note)
			lib.Close()
			if err == nil {
				target := dir
				if edit {
					target = filepath.Join(dir, "index.md")
				}
				if edit && editor == "visual-studio-code" {
					err = exec.Command("code", target).Run()
				} else {
					err = mygo.Shell.OpenPath(target)
				}
			}
		}
		if err != nil {
			a.post(func() { a.feedback = err.Error() })
		}
	}()
}
func (a *application) launchAgent() {
	if a.launching || a.model.Root == "" {
		return
	}
	a.launching = true
	root, cfg := a.model.Root, a.cfg
	go func() {
		err := terminal.Launch(root, cfg.Terminal, cfg.Shell, cfg.Agent)
		a.post(func() {
			a.launching = false
			if err != nil {
				a.feedback = err.Error()
			}
		})
	}()
}
