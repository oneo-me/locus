package main

import (
	"slices"

	"github.com/oneo-me/locus/internal/library"
)

// Only a successfully loaded library owns the live tree's expansion state.
// Closing during an initial scan or a failed open must preserve its saved state.
func (a *application) rememberTreeExpansion() {
	if a.treeRoot == "" {
		return
	}
	if a.state.ExpandedNodes == nil {
		a.state.ExpandedNodes = make(map[string][]string)
	}
	paths := slices.Collect(a.tree.Open.All())
	slices.Sort(paths)
	a.state.ExpandedNodes[a.treeRoot] = paths
}

func (a *application) restoreTreeExpansion() {
	if a.treeRoot != a.model.Root {
		a.tree.Open.Clear()
		for _, path := range a.state.ExpandedNodes[a.model.Root] {
			a.tree.Open.Add(path)
		}
		a.treeRoot = a.model.Root
	}
	// Include descendants of collapsed parents, but discard removed branches.
	branches := make(map[string]bool)
	var walk func([]*library.Note)
	walk = func(notes []*library.Note) {
		for _, note := range notes {
			if len(note.Children) > 0 {
				branches[note.Path] = true
				walk(note.Children)
			}
		}
	}
	walk(a.model.Notes)
	for path := range a.tree.Open.All() {
		if !branches[path] {
			a.tree.Open.Remove(path)
		}
	}
}
