package library

import (
	"os"
	"path"
	"path/filepath"
	"sort"
	"strings"
)

// Attachment is a regular file directly inside a note's assets directory.
type Attachment struct {
	Note, Name string
}

// Attachments lists the regular files directly inside every note body's
// assets directory, excluding the reserved icon and hidden files. A missing,
// unsafe, or unreadable directory contributes nothing, so one note cannot
// hide the rest of the projection.
func (l *Library) Attachments(notes []*Note) []Attachment {
	var attachments []Attachment
	var walk func([]*Note)
	walk = func(list []*Note) {
		for _, n := range list {
			if n.HasBody {
				for _, name := range l.attachmentNames(n.Path) {
					attachments = append(attachments, Attachment{Note: n.Path, Name: name})
				}
			}
			walk(n.Children)
		}
	}
	walk(notes)
	return attachments
}

// attachmentNames reads one note's assets directory without following
// symlinks in any component, matching the resource boundary elsewhere.
func (l *Library) attachmentNames(note string) []string {
	dir := path.Join(note, "assets")
	parts := strings.Split(dir, "/")
	for i := range parts {
		info, err := l.root.Lstat(filepath.FromSlash(strings.Join(parts[:i+1], "/")))
		if err != nil || info.Mode()&os.ModeSymlink != 0 || !info.IsDir() {
			return nil
		}
	}
	f, err := l.root.Open(filepath.FromSlash(dir))
	if err != nil {
		return nil
	}
	entries, err := f.ReadDir(-1)
	f.Close()
	if err != nil {
		return nil
	}
	var names []string
	for _, entry := range entries {
		name := entry.Name()
		if !entry.Type().IsRegular() || name == "icon.png" || strings.HasPrefix(name, ".") {
			continue
		}
		names = append(names, name)
	}
	sort.Strings(names)
	return names
}
