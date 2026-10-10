package library

import (
	"bytes"
	"context"
	"os/exec"
	"path"
	"path/filepath"
	"strings"
	"time"
)

// MarkGit projects status only when the selected library is the repository
// root. Git failure degrades to no indicators and never blocks file reading.
func MarkGit(root string, notes []*Note) {
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()
	top, err := exec.CommandContext(ctx, "git", "-C", root, "rev-parse", "--show-toplevel").Output()
	if err != nil {
		return
	}
	canonical, err := filepath.EvalSymlinks(strings.TrimSpace(string(top)))
	if err != nil || canonical != root {
		return
	}
	status, err := exec.CommandContext(ctx, "git", "-C", root, "status", "--porcelain=v1", "-z", "--untracked-files=all").Output()
	if err != nil {
		return
	}
	changed := changedNotes(status)
	var walk func([]*Note) bool
	walk = func(notes []*Note) bool {
		any := false
		for _, n := range notes {
			children := walk(n.Children)
			n.Modified = changed[n.Path] || children
			any = any || n.Modified
		}
		return any
	}
	walk(notes)
}
func changedNotes(status []byte) map[string]bool {
	result := map[string]bool{}
	fields := bytes.Split(status, []byte{0})
	mark := func(file string) {
		parts := strings.Split(file, "/")
		note := ""
		for i, part := range parts {
			if part == "assets" || part == "canvas" {
				note = strings.Join(parts[:i], "/")
				break
			}
		}
		if note == "" && path.Base(file) == "index.md" {
			note = path.Dir(file)
		}
		if validNote(note) {
			result[note] = true
		}
	}
	for i := 0; i < len(fields); i++ {
		field := fields[i]
		if len(field) < 4 {
			continue
		}
		mark(string(field[3:]))
		if bytes.ContainsAny(field[:2], "RC") && i+1 < len(fields) {
			i++
			mark(string(fields[i]))
		}
	}
	return result
}
