package main

import (
	"sync"
	"time"

	"github.com/egoist/mygo/plugins/terminal"
	"github.com/egoist/mygo/ui"
	launch "github.com/oneo-me/locus/internal/terminal"
	"github.com/oneo-me/onevra/source/components"
	"github.com/oneo-me/onevra/source/icons/lucide"
	"github.com/oneo-me/onevra/source/shells/sidebar"
)

// terminalSession guards asynchronous native loading, including completions
// after library changes and window shutdown. No worker retains application UI.
type terminalSession struct {
	mu         sync.Mutex
	generation uint64
	term       *terminal.Terminal
	loading    bool
	err        error
}

func (s *terminalSession) start(create func() (*terminal.Terminal, error), exited ...func(*terminal.Terminal)) {
	s.close()
	s.mu.Lock()
	generation := s.generation
	s.loading = true
	s.mu.Unlock()
	go func() {
		term, err := create()
		s.mu.Lock()
		if generation != s.generation {
			s.mu.Unlock()
			if term != nil {
				_ = term.Close()
			}
			return
		}
		s.term, s.err, s.loading = term, err, false
		s.mu.Unlock()
		if term != nil && len(exited) > 0 {
			<-term.Done()
			exited[0](term)
		}
	}()
}

func (s *terminalSession) snapshot() (*terminal.Terminal, bool, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	return s.term, s.loading, s.err
}

func (s *terminalSession) close() {
	s.mu.Lock()
	s.generation++
	term := s.term
	s.term, s.err, s.loading = nil, nil, false
	s.mu.Unlock()
	if term != nil {
		_ = term.Close()
	}
}

func (a *application) startTerminal(agent string) {
	if a.model.Root == "" || a.closing {
		return
	}
	a.terminalPreferencesChanged = false
	if agent == "none" {
		agent = ""
	}
	root, shell := a.model.Root, a.cfg.Shell
	a.terminal.start(func() (*terminal.Terminal, error) {
		argv, err := launch.Command(shell, agent)
		if err != nil {
			return nil, err
		}
		return terminal.New(terminal.Options{Command: argv, Dir: root})
	}, a.terminalExited)
}

// Exit delivery is identity-checked on the UI thread: a replaced session must
// never dismiss the popover or close the replacement.
func (a *application) terminalExited(term *terminal.Terminal) {
	a.post(func() { a.finishTerminal(term) })
}

func (a *application) finishTerminal(term *terminal.Terminal) {
	current, _, _ := a.terminal.snapshot()
	if current != term {
		return
	}
	// Preserve failures and their output instead of making the panel vanish.
	// 130 is the conventional SIGINT/Ctrl+C exit status.
	if code := term.ExitCode(); code != 0 && code != 130 {
		return
	}
	a.terminalOpen = false
	a.terminal.close()
}

func (a *application) toggleTerminal() {
	if a.model.Root == "" || a.closing {
		return
	}
	if a.settingsOpen {
		a.toggleSettings()
	}
	a.commandsOpen = false
	a.terminalOpen = !a.terminalOpen
	if a.terminalOpen {
		term, loading, _ := a.terminal.snapshot()
		exited := false
		if term != nil {
			select {
			case <-term.Done():
				exited = true
			default:
			}
		}
		if a.terminalPreferencesChanged || (!loading && (term == nil || exited)) {
			a.startTerminal(a.cfg.Agent)
		}
	}
	if a.shell != nil {
		a.shell.Window().Invalidate()
	}
}

func (a *application) terminalButton(c *ui.Context) {
	anchor := sidebar.IconButton(c, lucide.Terminal(), a.text("Terminal")).Disabled(a.model.Root == "").OnClick(a.toggleTerminal)
	components.Popover(c, anchor, &a.terminalOpen, func(panel ui.Element) {
		panel.Label(a.text("Terminal panel")).Width(600).Height(500).Padding(0).Gap(0).Clip()
		term, loading, err := a.terminal.snapshot()
		switch {
		case loading:
			c.After(50 * time.Millisecond)
		case err != nil:
			components.Text(c, err.Error()).Padding(12).Wrap().TextColor(c.Theme().Danger)
		case term != nil:
			terminal.View(c, term).Key(term).FillWidth().Grow(1).MinHeight(0).AutoFocus()
		}
	})
}
