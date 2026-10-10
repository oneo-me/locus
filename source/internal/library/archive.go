package library

import (
	"bytes"
	"crypto/rand"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"os"
	"path"
	"strings"

	"go.yaml.in/yaml/v3"
)

func Version(data []byte) string { sum := sha256.Sum256(data); return hex.EncodeToString(sum[:]) }

// SetArchived preserves unrelated source bytes and rejects changed versions.
// Complex/non-scalar archive fields must be edited externally rather than
// reserialized lossily. Multi-process writes are optimistic, not transactional.
func (l *Library) SetArchived(note, version string, archived bool) error {
	target := path.Join(note, "index.md")
	data, err := l.Resource(target)
	if err != nil {
		return err
	}
	if Version(data) != version {
		return errors.New("note changed; reload before archiving")
	}
	updated, err := archiveSource(data, archived)
	if err != nil {
		return err
	}
	info, err := l.root.Stat(target)
	if err != nil {
		return err
	}
	temp := path.Join(note, ".locus-"+strings.ToLower(rand.Text()))
	f, err := l.root.OpenFile(temp, os.O_CREATE|os.O_EXCL|os.O_WRONLY, info.Mode().Perm())
	if err != nil {
		return err
	}
	defer l.root.Remove(temp)
	if _, err = f.Write(updated); err != nil {
		f.Close()
		return err
	}
	if err = f.Sync(); err != nil {
		f.Close()
		return err
	}
	if err = f.Close(); err != nil {
		return err
	}
	current, err := l.Resource(target)
	if err != nil {
		return err
	}
	if Version(current) != version {
		return errors.New("note changed while preparing archive update")
	}
	return l.root.Rename(temp, target)
}
func archiveSource(data []byte, archived bool) ([]byte, error) {
	value := "false"
	if archived {
		value = "true"
	}
	newline := "\n"
	if bytes.Contains(data, []byte("\r\n")) {
		newline = "\r\n"
	}
	prefix := ""
	text := string(data)
	if strings.HasPrefix(text, "\ufeff") {
		prefix = "\ufeff"
		text = strings.TrimPrefix(text, prefix)
	}
	raw, _ := Parts(data)
	if raw == nil {
		if strings.HasPrefix(text, "---\n") || strings.HasPrefix(text, "---\r\n") {
			return nil, errors.New("frontmatter is not closed")
		}
		return []byte(prefix + "---" + newline + "archived: " + value + newline + "---" + newline + text), nil
	}
	var node yaml.Node
	if err := yaml.Unmarshal(raw, &node); err != nil {
		return nil, err
	}
	if len(node.Content) > 0 && (node.Content[0].Kind != yaml.MappingNode || node.Content[0].Style&yaml.FlowStyle != 0) {
		return nil, errors.New("frontmatter must be a mapping")
	}
	lines := strings.SplitAfter(string(raw), "\n")
	changed := false
	if len(node.Content) > 0 {
		mapping := node.Content[0]
		for i := 0; i+1 < len(mapping.Content); i += 2 {
			k, v := mapping.Content[i], mapping.Content[i+1]
			if k.Value != "archived" {
				continue
			}
			if changed || v.Kind != yaml.ScalarNode || k.Line != v.Line || v.Tag != "!!bool" || v.Anchor != "" {
				return nil, errors.New("edit complex archived metadata externally")
			}
			line := lines[k.Line-1]
			colon := strings.IndexByte(line, ':')
			if colon < 0 || strings.TrimSpace(line[:colon]) != "archived" {
				return nil, errors.New("invalid archive field")
			}
			comment := ""
			if pos := strings.Index(line[colon+1:], "#"); pos >= 0 {
				comment = " " + strings.TrimRight(line[colon+1+pos:], "\r\n")
			}
			lines[k.Line-1] = line[:colon+1] + " " + value + comment + newline
			changed = true
		}
	}
	replacement := strings.Join(lines, "")
	if !changed {
		indent := ""
		if len(node.Content) > 0 && len(node.Content[0].Content) > 0 {
			indent = strings.Repeat(" ", node.Content[0].Content[0].Column-1)
		}
		replacement += indent + "archived: " + value + newline
	}
	start := strings.IndexByte(text, '\n') + 1
	return []byte(prefix + text[:start] + replacement + text[start+len(raw):]), nil
}

func (l *Library) Archived() ([]*Note, error) {
	notes, err := l.scanAll("", true)
	if err != nil {
		return nil, err
	}
	var out []*Note
	var walk func([]*Note)
	walk = func(notes []*Note) {
		for _, n := range notes {
			if n.Archived {
				out = append(out, n)
			}
			walk(n.Children)
		}
	}
	walk(notes)
	return out, nil
}
