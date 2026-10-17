package main

import (
	"strings"

	"github.com/egoist/mygo/ui"
	"github.com/oneo-me/onevra/source/components"
)

// notice is one toast waiting for the next frame. Onevra builds toasts from a
// UI Context, which only frame builders hold, so background tasks and event
// callbacks queue outcomes here and the active page drains the queue while it
// builds.
type notice struct {
	title       string
	description string
	kind        string
	action      string
	onAction    func()
}

// errorNotice queues an error toast. Multi-line messages use the first line
// as the title and the remaining lines as the description.
func (a *application) errorNotice(message string) {
	if message == "" {
		return
	}
	title, description := message, ""
	if i := strings.IndexByte(message, '\n'); i >= 0 {
		title, description = message[:i], strings.TrimSpace(message[i+1:])
	}
	a.notices = append(a.notices, notice{title: title, description: description, kind: components.ToastError})
	a.invalidate()
}

// successNotice queues a confirmation toast.
func (a *application) successNotice(message string) {
	if message == "" {
		return
	}
	a.notices = append(a.notices, notice{title: message, kind: components.ToastSuccess})
	a.invalidate()
}

// actionNotice queues a toast that stays until the user closes it or runs
// its action, for outcomes that ask the user to do something.
func (a *application) actionNotice(title, description, action string, onAction func()) {
	a.notices = append(a.notices, notice{title: title, description: description, action: action, onAction: onAction})
	a.invalidate()
}

// flushNotices adds the queued toasts to the current frame. Shell builds the
// toast viewport after the page, so a toast queued earlier in the same frame
// appears immediately; one queued while building waits for the next frame.
func (a *application) flushNotices(c *ui.Context) {
	for _, n := range a.notices {
		toast := ui.Toast{ID: n.title, Title: n.title, Description: n.description, Type: n.kind}
		if n.action != "" {
			toast.Action, toast.OnAction, toast.Timeout = n.action, n.onAction, -1
		}
		c.AddToast(toast)
	}
	a.notices = a.notices[:0]
}

// invalidate asks for a frame unless the window is already gone.
func (a *application) invalidate() {
	if a.shell != nil && !a.closing {
		a.shell.Window().Invalidate()
	}
}
