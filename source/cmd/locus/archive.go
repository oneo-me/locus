package main

import (
	"github.com/egoist/mygo/ui"
	"github.com/oneo-me/locus/internal/library"
	"github.com/oneo-me/onevra/components"
)

func (a *application) askArchive(note string) {
	root := a.model.Root
	go func() {
		lib, err := library.Open(root)
		var data []byte
		if err == nil {
			data, err = lib.Resource(note + "/index.md")
			lib.Close()
		}
		a.post(func() {
			if root != a.model.Root {
				return
			}
			if err != nil {
				a.feedback = err.Error()
				return
			}
			a.archiveRoot = root
			a.archiveTarget = note
			a.archiveVersion = library.Version(data)
			a.archiveOpen = true
		})
	}()
}
func (a *application) archiveDialog(c *ui.Context) {
	switch components.AlertDialog(c, &a.archiveOpen, components.AlertDialogOptions{Title: a.text("Archive note"), Message: a.text("Hide this note and its descendants? Files are preserved and can be restored in Settings.") + "\n" + a.archiveTarget, ConfirmLabel: a.text("Archive"), CancelLabel: a.text("Cancel")}) {
	case components.DialogConfirmed:
		a.setArchived(a.archiveRoot, a.archiveTarget, a.archiveVersion, true)
	}
}
func (a *application) setArchived(root, note, version string, value bool) {
	go func() {
		lib, err := library.Open(root)
		if err == nil {
			err = lib.SetArchived(note, version, value)
			lib.Close()
		}
		a.post(func() {
			if err != nil {
				a.feedback = err.Error()
			} else if root == a.model.Root {
				a.model.Refresh()
				a.loadArchived()
			}
		})
	}()
}
func (a *application) loadArchived() {
	root := a.model.Root
	if root == "" {
		a.archived = nil
		return
	}
	go func() {
		lib, err := library.Open(root)
		var notes []*library.Note
		if err == nil {
			notes, err = lib.Archived()
			lib.Close()
		}
		a.post(func() {
			if root != a.model.Root {
				return
			}
			if err != nil {
				a.feedback = err.Error()
			} else {
				a.archived = notes
			}
		})
	}()
}
func (a *application) restoreNote(note string) {
	root := a.model.Root
	go func() {
		lib, err := library.Open(root)
		var data []byte
		if err == nil {
			data, err = lib.Resource(note + "/index.md")
			lib.Close()
		}
		a.post(func() {
			if err != nil {
				a.feedback = err.Error()
			} else {
				a.setArchived(root, note, library.Version(data), false)
			}
		})
	}()
}
