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
	// exited records that the current process ended, so that the running
	// indicator stops even when a failure retains the terminal for output.
	exited bool
	err    error
}

func (s *terminalSession) start(create func() (*terminal.Terminal, error), onExit ...func(*terminal.Terminal)) {
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
		if term == nil {
			return
		}
		<-term.Done()
		s.mu.Lock()
		if generation == s.generation {
			s.exited = true
		}
		s.mu.Unlock()
		if len(onExit) > 0 {
			onExit[0](term)
		}
	}()
}

func (s *terminalSession) snapshot() (*terminal.Terminal, bool, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	return s.term, s.loading, s.err
}

// running reports whether a terminal is starting or its process is alive, for
// the persistent header indicator that points at background work.
func (s *terminalSession) running() bool {
	s.mu.Lock()
	defer s.mu.Unlock()
	if s.loading {
		return true
	}
	return s.term != nil && !s.exited
}

func (s *terminalSession) close() {
	s.mu.Lock()
	s.generation++
	term := s.term
	s.term, s.err, s.loading, s.exited = nil, nil, false, false
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
		argv, err := launch.ResolveCommand(shell, agent)
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
	if a.terminal.running() {
		a.markRunningTerminal(c, anchor)
	}
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

// terminalRunningFrames breathe the header indicator from rest to full accent
// and back. Keyframes copies the slice when it starts a sequence.
var terminalRunningFrames = []ui.Keyframe{
	{At: 0, Value: 0},
	{At: 650 * time.Millisecond, Value: 1, Ease: ui.EaseInOut},
}

// markRunningTerminal makes the titlebar terminal button pulse while a session
// is starting or running, so the header still points at background work after
// the popover is dismissed. The pulse comes from a repeating keyframe
// sequence; Reduce Motion keeps the accent mark without movement.
func (a *application) markRunningTerminal(c *ui.Context, button ui.Element) {
	t := c.Theme()
	pulse := float32(1)
	if !c.Preferences().ReduceMotion {
		pulse = button.Keyframes("locus.terminal.running", terminalRunningFrames,
			ui.KeyframeOptions{Repeat: ui.RepeatForever, Alternate: true})
	}
	button.TextColor(t.TextMuted.Mix(t.Accent, 0.55+0.45*pulse))
	button.DrawOver(func(p *ui.Painter, r ui.Rect) {
		if r.W < 2 || r.H < 2 {
			return
		}
		p.Stroke(ui.Rect{X: r.X + 1, Y: r.Y + 1, W: r.W - 2, H: r.H - 2},
			t.Accent.Alpha(0.18+0.5*pulse), t.Radius, 1+0.5*pulse)
	})
}
