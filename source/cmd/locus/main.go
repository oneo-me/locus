package main

import (
	"context"
	"flag"
	"log"
	"os"
	"path/filepath"
	"runtime"

	"github.com/egoist/mygo"
	"github.com/egoist/mygo/ui"
	"github.com/oneo-me/locus/internal/canvas"
	"github.com/oneo-me/locus/internal/discovery"
	"github.com/oneo-me/locus/internal/library"
	"github.com/oneo-me/locus/internal/preferences"
	"github.com/oneo-me/locus/internal/workspace"
	"github.com/oneo-me/onevra/source/components"
	"github.com/oneo-me/onevra/source/components/command"
	"github.com/oneo-me/onevra/source/components/markdown"
	"github.com/oneo-me/onevra/source/shells/sidebar"
)

type application struct {
	shell                      *sidebar.Shell
	model                      workspace.Model
	tree                       components.TreeVirtualizedState[string]
	cursor                     int
	treeRoot                   string
	renderedSource             string
	document                   *markdown.MarkdownDocument
	galleries                  map[int]galleryBlock
	galleryPreview             galleryItem
	galleryPreviewOpen         bool
	attachmentBlocks           map[string]attachmentBlock
	articleScroll              ui.ScrollState
	articleScrollRoot          string
	articleScrollNote          string
	choosing, closing          bool
	quitting                   bool
	index                      map[string]*library.Note
	children                   map[string][]string
	roots                      []string
	cfg                        preferences.Config
	writer                     *preferences.Writer
	state                      preferences.State
	statePath                  string
	stateWritable              bool
	restore                    bool
	hidden                     bool // Native tests prepare the window without showing it.
	notices                    []notice
	reportedError              string
	watchCancel                context.CancelFunc
	watchRoot                  string
	watchGeneration            uint64
	service                    *discovery.Service
	mcpKey, mcpError           string
	router                     canvas.Router
	web                        *mygo.WebView
	session                    *canvas.Session
	canvasKey                  string
	canvasGeneration           uint64
	canvasError                string
	canvasLoading              bool
	settingsOpen, commandsOpen bool
	settingState               components.SettingsState
	palette                    command.State
	history                    []string
	historyIndex               int
	exporting                  bool
	terminalOpen               bool
	terminalPreferencesChanged bool
	terminal                   terminalSession
	images                     map[string]*ui.Bitmap
	iconRoot                   string
	iconProjection             *library.Note
	iconTicket                 uint64
	noteIcons                  map[string]*ui.Bitmap
	imageKey                   string
	imageTicket                uint64
}

func main() {
	directory := flag.String("library", "", "Markdown library directory")
	flag.Parse()
	home, err := os.UserHomeDir()
	if err != nil {
		log.Fatal(err)
	}
	data := filepath.Join(home, ".oneo", "locus")
	cfg, store, loadErr := preferences.Load(filepath.Join(data, "settings.yaml"))
	a := &application{cursor: -1, cfg: cfg, statePath: filepath.Join(data, "state.yaml"), stateWritable: true, restore: true, historyIndex: -1}
	if loadErr != nil {
		a.errorNotice(loadErr.Error())
	}
	a.state, err = preferences.LoadState(a.statePath)
	if err != nil {
		a.stateWritable = false
		a.errorNotice(err.Error())
	}
	mygo.App.SetName("Locus")
	if err := os.MkdirAll(data, 0700); err != nil {
		log.Fatal(err)
	}
	mygo.App.SetPath(mygo.PathUserData, data)
	if !mygo.App.RequestSingleInstanceLock() {
		return
	}
	mygo.App.OnSecondInstance(func([]string, string) {
		a.showWindow()
	})
	mygo.App.OnActivate(func(hasVisibleWindows bool) {
		if !hasVisibleWindows {
			a.showWindow()
		}
	})
	mygo.App.WhenReady(func() {
		root := cfg.Library
		if *directory != "" {
			root = *directory
			a.restore = false
		}
		a.start(root)
		if store != nil {
			a.writer = preferences.NewWriter(store, func(err error) {
				if err != nil {
					a.post(func() { a.errorNotice(err.Error()) })
				}
			})
		}
	})
	mygo.App.OnBeforeQuit(func(*mygo.QuitEvent) {
		a.quitting = true
		a.saveState()
	})
	mygo.App.OnWillQuit(func(*mygo.QuitEvent) { a.stop(); a.writer.Close() })
	if err := mygo.App.Run(); err != nil {
		log.Fatal(err)
	}
}

// retainWindowOnClose reports whether closing the main window hides the
// retained workspace instead of quitting. Packaged macOS builds keep the app
// active in the Dock, while development runs exit so they leave no Dock
// activity behind.
func (a *application) retainWindowOnClose() bool {
	return runtime.GOOS == "darwin" && !a.quitting && !mygo.IsDev()
}
func (a *application) showWindow() {
	if a.shell != nil && !a.closing && !a.quitting {
		a.shell.Window().Show()
		a.shell.Window().Focus()
	}
}
func (a *application) post(fn func()) {
	if a.shell != nil {
		a.shell.Window().Update(func() {
			if !a.closing {
				fn()
			}
		})
	}
}
func (a *application) start(root string) {
	if a.cfg.Agent == "" {
		a.cfg = preferences.Default()
	}
	if err := mygo.Protocol.Handle(canvas.Scheme, &a.router); err != nil {
		a.errorNotice(err.Error())
	}
	a.shell = sidebar.NewShell(sidebar.ShellOptions{Title: "Locus", Hidden: a.hidden, Width: 980, Height: 820, MinWidth: 980, MinHeight: 820, LeftWidth: 252, LeftMinWidth: 252, LeftMaxWidth: 520, Left: a.left, Content: a.content, ContentTitle: a.title, LeftHeader: a.navigation, LeftToolbar: a.toolbar, Toasts: components.ToastOptions{Placement: components.ToastTop}})
	a.model.Dispatch = func(fn func()) { a.post(fn) }
	a.model.Changed = a.changed
	a.resetTree()
	a.applyAppearance()
	a.installMenu()
	a.shell.Window().OnClose(func(e *mygo.CloseEvent) {
		a.saveState()
		if a.retainWindowOnClose() {
			e.PreventDefault()
			a.shell.Window().Hide()
		}
	})
	a.shell.Window().OnClosed(a.stop)
	if a.state.SidebarWidth >= 252 {
		a.shell.SetLeftWidth(a.state.SidebarWidth)
	}
	a.shell.SetLeftExpanded(!a.state.Collapsed)
	if root != "" {
		a.model.Open(root)
	} else {
		a.shell.SetPage(a.libraryPicker)
	}
}
func (a *application) resetTree() {
	a.treeRoot = ""
	a.tree = components.TreeVirtualizedState[string]{}
	a.cursor = -1
	a.tree.List.Selected = &a.cursor
	a.tree.List.OnChange = func() {
		if a.cursor >= 0 && a.cursor < a.tree.Rows() {
			a.navigate(a.tree.Item(a.cursor))
		}
	}
}
func (a *application) changed() {
	if a.closing {
		return
	}
	if a.model.Error != nil {
		if message := a.model.Error.Error(); message != a.reportedError {
			a.reportedError = message
			a.errorNotice(message)
		}
	} else {
		a.reportedError = ""
	}
	if !a.model.Loading && a.model.Error == nil && a.model.Root != "" {
		a.restoreTreeExpansion()
		if a.watchRoot != a.model.Root {
			if a.watchCancel != nil {
				a.watchCancel()
			}
			a.watchRoot = a.model.Root
			a.watchGeneration++
			generation := a.watchGeneration
			ctx, cancel := context.WithCancel(context.Background())
			a.watchCancel = cancel
			root := a.model.Root
			go library.Watch(ctx, root, func() {
				a.post(func() {
					if generation == a.watchGeneration {
						a.imageKey = ""
						a.model.Refresh()
					}
				})
			}, func(err error) {
				a.post(func() {
					if generation == a.watchGeneration {
						a.errorNotice(err.Error())
					}
				})
			})
		}
		if a.mcpKey != a.model.Root {
			a.mcpKey = a.model.Root
			a.restartMCP()
		}
		if a.restore {
			a.restore = false
			if a.state.Library == a.model.Root && library.Find(a.model.Notes, a.state.Selected) != nil {
				if _, saved := a.state.ExpandedNodes[a.model.Root]; saved {
					a.focusNote(a.state.Selected)
				} else {
					a.selectNote(a.state.Selected)
				}
			}
		}
	}
	if a.shell != nil {
		if a.model.Root == "" {
			a.shell.SetPage(a.libraryPicker)
		} else {
			a.shell.SetPage(nil)
		}
	}
	a.syncCanvas()
	a.loadImages()
	a.loadNoteIcons()
	if a.shell != nil {
		a.shell.Window().Invalidate()
	}
}
func (a *application) openLibrary(root string) {
	a.terminalOpen = false
	a.terminal.close()
	a.rememberTreeExpansion()
	a.restore = false
	a.history = nil
	a.historyIndex = -1
	if a.watchCancel != nil {
		a.watchCancel()
		a.watchCancel = nil
	}
	a.watchRoot = ""
	a.watchGeneration++
	a.resetTree()
	a.model.Open(root)
	a.cfg.Library = root
	a.saveConfig()
	a.restartMCP()
}
func (a *application) chooseLibrary() {
	if a.choosing {
		return
	}
	a.choosing = true
	parent := a.shell.Window()
	title := a.text("Open library")
	go func() {
		paths, err := mygo.Dialog.Open(mygo.OpenDialogOptions{Parent: parent, Title: title, Directory: true})
		a.post(func() {
			a.choosing = false
			if err != nil {
				a.errorNotice(err.Error())
			} else if len(paths) == 1 {
				a.openLibrary(paths[0])
			}
		})
	}()
}
func (a *application) saveConfig() {
	if a.writer != nil {
		a.writer.Submit(a.cfg)
	}
	a.applyAppearance()
}
func (a *application) saveState() {
	if a.shell == nil || !a.stateWritable || a.statePath == "" {
		return
	}
	a.rememberTreeExpansion()
	state := preferences.State{ExpandedNodes: a.state.ExpandedNodes, Library: a.model.Root, Selected: a.model.Selected, SidebarWidth: a.shell.LeftWidth(), Collapsed: !a.shell.LeftExpanded()}
	if err := preferences.SaveState(a.statePath, state); err != nil {
		log.Print(err)
		a.errorNotice(err.Error())
	}
}
func (a *application) stop() {
	if a.closing {
		return
	}
	a.closing = true
	a.terminal.close()
	a.palette.ReleaseInputSource()
	if a.watchCancel != nil {
		a.watchCancel()
	}
	if a.service != nil {
		a.service.Close()
		a.service = nil
	}
	a.canvasGeneration++
	if a.web != nil {
		a.web.Destroy()
		a.web = nil
	}
	a.router.CloseAll()
	a.session = nil
}
func (a *application) applyAppearance() {
	switch a.cfg.Theme {
	case "light":
		mygo.Theme.SetSource(mygo.ThemeLight)
	case "dark":
		mygo.Theme.SetSource(mygo.ThemeDark)
	default:
		mygo.Theme.SetSource(mygo.ThemeSystem)
	}
	if a.shell != nil {
		a.shell.Window().Invalidate()
	}
}
func (a *application) restartMCP() {
	a.mcpKey = a.model.Root
	if a.service != nil {
		a.service.Close()
		a.service = nil
	}
	a.mcpError = ""
	if !a.cfg.MCP {
		return
	}
	service, err := discovery.Start(a.model.Root, a.cfg.Port)
	if err != nil {
		a.mcpError = err.Error()
		a.errorNotice(err.Error())
	} else {
		a.service = service
	}
}
func (a *application) navigate(p string) {
	if a.model.Selected != p {
		if a.historyIndex+1 < len(a.history) {
			a.history = a.history[:a.historyIndex+1]
		}
		a.history = append(a.history, p)
		a.historyIndex = len(a.history) - 1
	}
	a.model.Select(p)
}
func (a *application) back(delta int) {
	for i := a.historyIndex + delta; i >= 0 && i < len(a.history); i += delta {
		if library.Find(a.model.Notes, a.history[i]) != nil {
			a.historyIndex = i
			a.selectNote(a.history[i])
			return
		}
	}
}
func (a *application) refresh() {
	a.imageKey = ""
	a.canvasKey = ""
	a.model.Refresh()
	a.syncCanvas()
}
func (a *application) toggleSettings() {
	a.settingsOpen = !a.settingsOpen
	a.terminalOpen = false
	a.commandsOpen = false
	a.shell.Window().Invalidate()
}
func (a *application) installMenu() {
	action := func(fn func()) func(*mygo.MenuItem, *mygo.Window) { return func(*mygo.MenuItem, *mygo.Window) { fn() } }
	mygo.App.SetMenu(mygo.NewMenu([]*mygo.MenuItem{
		{Label: "Locus", Submenu: []*mygo.MenuItem{{Label: a.text("Settings"), Accelerator: "CmdOrCtrl+,", Click: action(a.toggleSettings)}, mygo.Separator(), {Role: mygo.RoleQuit}}},
		{Label: a.text("File"), Submenu: []*mygo.MenuItem{{Label: a.text("Open library"), Accelerator: "CmdOrCtrl+O", Click: action(a.chooseLibrary)}, {Label: a.text("Refresh"), Accelerator: "CmdOrCtrl+R", Click: action(a.refresh)}, {Role: mygo.RoleClose}}},
		{Role: mygo.RoleEditMenu},
		{Label: a.text("View"), Submenu: []*mygo.MenuItem{{Label: a.text("Search notes"), Accelerator: "CmdOrCtrl+F", Click: action(func() { a.commandsOpen = !a.commandsOpen; a.shell.Window().Invalidate() })}, {Label: a.text("Terminal"), Accelerator: "CmdOrCtrl+`", Click: action(a.toggleTerminal)}}},
	}))
}
