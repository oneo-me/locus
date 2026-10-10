// Package workspace owns selection and asynchronous read/scan coordination.
package workspace

import (
	"errors"
	"github.com/oneo-me/locus/internal/library"
)

// Model is confined to the UI thread. Dispatch must deliver on that thread.
// Each task opens its own rooted handle, so switching libraries cannot close a
// handle still in use. Tickets isolate late scans and reads independently.
type Model struct {
	Root                   string
	Notes                  []*library.Note
	Selected               string
	Document               library.Document
	Error                  error
	scanError, readError   error
	Loading                bool
	refreshAgain           bool
	Changed                func()
	Dispatch               func(func())
	scanTicket, readTicket uint64
}

func (m *Model) notify() {
	m.Error = errors.Join(m.scanError, m.readError)
	if m.Changed != nil {
		m.Changed()
	}
}
func (m *Model) Open(root string) {
	m.scanTicket++
	m.readTicket++
	ticket := m.scanTicket
	m.refreshAgain = false
	m.Root = root
	m.Notes = nil
	m.Selected = ""
	m.Document = library.Document{}
	m.scanError, m.readError = nil, nil
	m.Loading = true
	m.notify()
	go func() {
		lib, err := library.Open(root)
		var notes []*library.Note
		canonical := root
		if err == nil {
			canonical = lib.Path
			notes, err = lib.Scan()
			if err == nil {
				library.MarkGit(canonical, notes)
			}
			lib.Close()
		}
		m.Dispatch(func() {
			if ticket != m.scanTicket {
				return
			}
			m.Loading = false
			m.scanError = err
			if err == nil {
				m.Root = canonical
				m.Notes = notes
			}
			m.scanned()
		})
	}()
}
func (m *Model) Refresh() {
	if m.Root == "" {
		return
	}
	if m.Loading {
		m.refreshAgain = true
		return
	}
	m.scanTicket++
	ticket, root := m.scanTicket, m.Root
	m.Loading = true
	m.notify()
	go func() {
		lib, err := library.Open(root)
		var notes []*library.Note
		if err == nil {
			notes, err = lib.Scan()
			if err == nil {
				library.MarkGit(lib.Path, notes)
			}
			lib.Close()
		}
		m.Dispatch(func() {
			if ticket != m.scanTicket {
				return
			}
			m.Loading = false
			m.scanError = err
			if err == nil {
				m.Notes = notes
				selected := m.Selected
				if library.Find(notes, selected) == nil {
					selected = ""
				}
				m.Select(selected)
			}
			m.scanned()
		})
	}()
}
func (m *Model) scanned() {
	m.notify()
	if m.refreshAgain {
		m.refreshAgain = false
		m.Refresh()
	}
}

func (m *Model) Select(p string) {
	m.readTicket++
	ticket, root := m.readTicket, m.Root
	previous := m.Selected
	m.Selected = p
	if previous != p {
		m.Document = library.Document{}
	}
	m.readError = nil
	note := library.Find(m.Notes, p)
	if note == nil || !note.HasBody {
		m.Document = library.Document{}
		m.notify()
		return
	}
	m.notify()
	go func() {
		lib, err := library.Open(root)
		var document library.Document
		if err == nil {
			document, err = lib.Read(p)
			lib.Close()
		}
		m.Dispatch(func() {
			if ticket != m.readTicket || root != m.Root {
				return
			}
			m.readError = err
			if err == nil {
				m.Document = document
			}
			m.notify()
		})
	}()
}
