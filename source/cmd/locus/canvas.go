package main

import (
	"context"
	"encoding/json"
	"net/url"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/egoist/mygo"
	"github.com/oneo-me/locus/internal/library"
)

func (a *application) syncCanvas() {
	note := library.Find(a.model.Notes, a.model.Selected)
	key := ""
	if note != nil && note.Canvas != "" {
		key = a.model.Root + "\x00" + note.Path
	}
	if key == a.canvasKey {
		return
	}
	a.canvasKey = key
	a.canvasGeneration++
	ticket := a.canvasGeneration
	a.canvasError = ""
	a.canvasLoading = false
	if a.web != nil {
		a.web.Destroy()
		a.web = nil
	}
	a.router.Close(a.session)
	a.session = nil
	if key == "" {
		return
	}
	root, p, title := a.model.Root, note.Path, note.Title
	a.canvasLoading = true
	go func() {
		session, err := a.router.Open(root, p, title)
		a.shell.Window().Update(func() {
			if a.closing || ticket != a.canvasGeneration {
				a.router.Close(session)
				return
			}
			a.canvasLoading = false
			if err != nil {
				a.canvasError = err.Error()
				return
			}
			a.session = session
			browser, err := a.shell.Window().NewWebView(mygo.WebViewOptions{Page: mygo.PageOptions{DevTools: mygo.DevToolsDisabled}})
			if err != nil {
				a.canvasError = err.Error()
				a.router.Close(session)
				a.session = nil
				return
			}
			a.web = browser
			browser.Page().SetPermissionHandler(func(mygo.PermissionRequest) bool { return false })
			browser.Page().SetWindowOpenHandler(func(req mygo.WindowOpenRequest) *mygo.WindowOptions {
				a.post(func() { a.openLink(req.URL) })
				return nil
			})
			browser.Page().OnWillNavigate(func(e *mygo.NavigateEvent) {
				if e.URL == session.URL() {
					return
				}
				e.PreventDefault()
				if strings.HasPrefix(e.URL, "locus://") || e.UserInitiated {
					a.post(func() { a.openLink(e.URL) })
				}
			})
			browser.Page().OnDidFailLoad(func(err *mygo.LoadError) {
				a.post(func() {
					if a.web == browser {
						a.canvasError = err.Error()
					}
				})
			})
			browser.Page().OnDOMReady(func() { a.post(a.syncCanvasTheme) })
			browser.Page().LoadURL(session.URL())
		})
	}()
}
func (a *application) exportCanvas() {
	if a.session == nil || a.exporting {
		return
	}
	a.exporting = true
	session := a.session
	directory := a.cfg.Export
	if directory == "" {
		home, _ := os.UserHomeDir()
		directory = filepath.Join(home, "Downloads")
	}
	if strings.HasPrefix(directory, "~/") {
		home, err := os.UserHomeDir()
		if err != nil {
			a.exporting = false
			a.feedback = err.Error()
			return
		}
		directory = filepath.Join(home, directory[2:])
	}
	if !filepath.IsAbs(directory) {
		a.exporting = false
		a.feedback = "Export directory must be an absolute path"
		return
	}
	go func() {
		target, err := session.Export(directory)
		a.post(func() {
			a.exporting = false
			if err != nil {
				a.feedback = err.Error()
			} else {
				a.feedback = a.text("Exported") + ": " + target
			}
		})
	}()
}
func (a *application) syncCanvasTheme() {
	if a.web == nil {
		return
	}
	browser := a.web
	mode := a.cfg.Theme
	value, _ := json.Marshal(mode)
	go func() {
		ctx, cancel := context.WithTimeout(context.Background(), time.Second)
		defer cancel()
		browser.Page().EvalContext(ctx, `document.documentElement.classList.toggle('dark', `+string(value)+` === 'dark' || (`+string(value)+` === 'system' && matchMedia('(prefers-color-scheme:dark)').matches)); document.documentElement.style.colorScheme = `+string(value)+` === 'system' ? 'light dark' : `+string(value))
	}()
}
func (a *application) openLink(target string) {
	if strings.HasPrefix(target, "locus://") {
		p, err := library.ParseAddress(target)
		if err != nil {
			a.feedback = err.Error()
		} else if library.Find(a.model.Notes, p) != nil {
			a.navigate(p)
			a.selectNote(p)
		}
		a.shell.Window().Invalidate()
		return
	}
	parsed, err := url.Parse(target)
	if err == nil && parsed.Scheme == "" && parsed.Host == "" && parsed.Path != "" {
		p, resolveErr := resourcePath(a.model.Selected, target)
		if resolveErr == nil {
			note := strings.TrimSuffix(p, "/index.md")
			if library.Find(a.model.Notes, note) != nil {
				a.navigate(note)
				a.selectNote(note)
				return
			}
			root := a.model.Root
			go func() {
				lib, err := library.Open(root)
				if err != nil {
					return
				}
				_, err = lib.Resource(p)
				full := filepath.Join(lib.Path, filepath.FromSlash(p))
				lib.Close()
				if err == nil {
					mygo.Shell.ShowItemInFolder(full)
				}
			}()
			return
		}
	}
	if err == nil && (parsed.Scheme == "http" || parsed.Scheme == "https" || parsed.Scheme == "mailto") {
		go func() {
			if err := mygo.Shell.OpenExternal(target); err != nil {
				a.post(func() { a.feedback = err.Error() })
			}
		}()
	}
}
