package main

import (
	"github.com/egoist/mygo"
	"github.com/oneo-me/locus/internal/library"
)

func (a *application) uploadIcon(note string) {
	if a.choosing {
		return
	}
	a.choosing = true
	root := a.model.Root
	parent, title := a.shell.Window(), a.text("Upload icon")
	go func() {
		paths, err := mygo.Dialog.Open(mygo.OpenDialogOptions{Parent: parent, Title: title, Filters: []mygo.FileFilter{{Name: "PNG", Extensions: []string{"png"}}}})
		changed := false
		if err == nil && len(paths) == 1 {
			var data []byte
			data, err = library.PrepareIcon(paths[0])
			if err == nil {
				var lib *library.Library
				lib, err = library.Open(root)
				if err == nil {
					err = lib.SetIcon(note, data)
					lib.Close()
					changed = err == nil
				}
			}
		}
		a.post(func() {
			a.choosing = false
			if a.model.Root != root {
				return
			}
			if err != nil {
				a.feedback = a.text("Icon upload failed") + ": " + err.Error()
			} else if changed {
				a.feedback = a.text("Icon updated")
				a.model.Refresh()
			}
		})
	}()
}
