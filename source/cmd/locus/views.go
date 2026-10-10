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
	"go.yaml.in/yaml/v3"
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
	ui.Column(c).Fill().MinHeight(0).Padding(sidebar.HeaderHeight(c), 8, 8, 8).Gap(8).Children(func() {
		components.Button(c, a.text("Open library"), components.ButtonSecondary).FillWidth().Disabled(a.choosing).OnClick(a.chooseLibrary)
		components.Button(c, a.text("Search notes"), components.ButtonGhost).FillWidth().OnClick(func() { a.commandsOpen = true })
		if a.model.Root != "" {
			components.Text(c, a.model.Root).SingleLine().Ellipsis("…").TextColor(c.Theme().TextMuted)
		}
		components.TreeVirtualized(c, &a.tree, a.roots, func(p string) []string { return a.children[p] }, func(p string) {
			if n := a.index[p]; n != nil {
				if n.Modified {
					components.Text(c, "●").Label(a.text("Modified files")).TextColor(c.Theme().Accent)
				}
				components.Text(c, n.Title).SingleLine().Ellipsis("…").MinWidth(0)
			}
		}).FillWidth().Grow(1).MinHeight(0)
	})
}
func (a *application) toolbar(c *ui.Context) {
	sidebar.IconButton(c, lucide.Settings(), a.text("Settings")).OnClick(a.toggleSettings)
	sidebar.IconButton(c, lucide.RefreshCw(), a.text("Refresh")).Disabled(a.model.Loading || a.model.Root == "").OnClick(a.refresh)
	sidebar.IconButton(c, lucide.SquareTerminal(), a.text("Open Agent")).Disabled(a.model.Root == "" || a.launching).OnClick(a.launchAgent)
	if a.model.Loading {
		components.Text(c, a.text("Loading…"))
	}
	if a.service != nil {
		components.Text(c, "MCP").TextColor(c.Theme().Accent)
	}
}
func (a *application) title(c *ui.Context) {
	ui.Row(c).FillWidth().MinWidth(0).AlignItems(ui.Center).Gap(4).Children(func() {
		sidebar.IconButton(c, lucide.ArrowLeft(), a.text("Back")).Disabled(a.historyIndex <= 0).OnClick(func() { a.back(-1) })
		sidebar.IconButton(c, lucide.ArrowRight(), a.text("Forward")).Disabled(a.historyIndex+1 >= len(a.history)).OnClick(func() { a.back(1) })
		title := "Locus"
		note := library.Find(a.model.Notes, a.model.Selected)
		if note != nil {
			title = note.Title
		}
		components.Text(c, title).Bold().SingleLine().Ellipsis("…").Grow(1).MinWidth(0)
		if note != nil && note.HasBody {
			sidebar.IconButton(c, lucide.Link(), a.text("Copy reference")).OnClick(func() { mygo.Clipboard.WriteText(library.Address(note.Path)) })
			sidebar.IconButton(c, lucide.Pencil(), a.text("Edit note")).OnClick(func() { a.fileAction(note.Path, true) })
			sidebar.IconButton(c, lucide.Folder(), a.text("Open directory")).OnClick(func() { a.fileAction(note.Path, false) })
			sidebar.IconButton(c, lucide.Archive(), a.text("Archive note")).OnClick(func() { a.askArchive(note.Path) })
			if note.Canvas != "" {
				sidebar.IconButton(c, lucide.Download(), a.text("Export canvas")).Disabled(a.session == nil || a.exporting).OnClick(a.exportCanvas)
			}
		}
	})
}
func (a *application) content(c *ui.Context) {
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
	ui.Column(c).Fill().MinHeight(0).Padding(sidebar.HeaderHeight(c), 24, 16, 24).Children(func() {
		for _, message := range []string{a.storageError, a.feedback, a.mcpError} {
			if message != "" {
				components.Text(c, message).Wrap().TextColor(c.Theme().Danger)
			}
		}
		if a.model.Error != nil {
			components.Text(c, a.model.Error.Error()).Wrap().TextColor(c.Theme().Danger)
		}
		note := library.Find(a.model.Notes, a.model.Selected)
		if note == nil {
			message := "Select a note in the sidebar."
			if a.model.Root == "" {
				message = "Open a Markdown library to start reading."
			} else if !a.model.Loading && len(a.model.Notes) == 0 {
				message = "No visible notes. Notes use a directory containing index.md."
			}
			components.Text(c, a.text(message)).Wrap()
			return
		}
		if note.Canvas != "" {
			if note.Summary != "" {
				components.Text(c, note.Summary).Wrap()
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
		components.ScrollView(c, components.ScrollViewOptions{}).Key(note.Path).Fill().Children(func() {
			if note.Summary != "" {
				components.Text(c, note.Summary).Wrap().TextColor(c.Theme().TextMuted)
			}
			metadata(c, a.model.Document.Metadata)
			a.document.Render(c, markdown.MarkdownOptions{OnLink: a.openLink, Images: a.images}).FillWidth()
		})
	})
	a.commandDialog(c)
	a.archiveDialog(c)
	a.settingsDialog(c)
}
func metadata(c *ui.Context, node *yaml.Node) {
	if node == nil || len(node.Content) == 0 || node.Content[0].Kind != yaml.MappingNode {
		return
	}
	mapping := *node.Content[0]
	mapping.Content = nil
	for i := 0; i+1 < len(node.Content[0].Content); i += 2 {
		k, v := node.Content[0].Content[i], node.Content[0].Content[i+1]
		switch k.Value {
		case "title", "summary", "icon", "archived":
			continue
		}
		mapping.Content = append(mapping.Content, k, v)
	}
	if len(mapping.Content) > 0 {
		data, err := yaml.Marshal(&mapping)
		if err == nil {
			components.Text(c, strings.TrimSpace(string(data))).Wrap().TextColor(c.Theme().TextMuted)
		}
	}
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
