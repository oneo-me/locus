// Package library owns the file-backed note model, independently of the UI.
package library

import (
	"bufio"
	"errors"
	"fmt"
	"io"
	"net/url"
	"os"
	"path"
	"path/filepath"
	"sort"
	"strings"
	"unicode/utf8"

	"go.yaml.in/yaml/v3"
)

const maxMetadata = 1 << 20
const maxBody = 32 << 20

// Note paths are slash-separated library-relative directory names.
// Empty directories are omitted; organizing directories have HasBody=false.
type Note struct {
	Path, Title, Summary, Canvas, Icon string
	HasBody                            bool
	Archived                           bool
	Modified                           bool
	Children                           []*Note
}

type Document struct {
	Body     string
	Metadata *yaml.Node
}

type Library struct {
	root *os.Root
	Path string
}

func Open(directory string) (*Library, error) {
	absolute, err := filepath.Abs(directory)
	if err != nil {
		return nil, err
	}
	absolute, err = filepath.EvalSymlinks(absolute)
	if err != nil {
		return nil, err
	}
	root, err := os.OpenRoot(absolute)
	if err != nil {
		return nil, err
	}
	return &Library{root: root, Path: absolute}, nil
}
func (l *Library) Close() error { return l.root.Close() }

func allowed(name string) bool {
	return name != "" && utf8.ValidString(name) && !strings.ContainsAny(name, "\x00\\") && !strings.HasPrefix(name, ".") && name != "assets" && name != "canvas" && name != "node_modules"
}
func validNote(p string) bool {
	if p == "" || strings.Contains(p, "\\") || path.IsAbs(p) {
		return false
	}
	for _, part := range strings.Split(p, "/") {
		if !allowed(part) {
			return false
		}
	}
	return true
}

// Address encodes segments separately so separators retain their meaning.
func Address(p string) string {
	parts := strings.Split(p, "/")
	for i := range parts {
		parts[i] = url.PathEscape(parts[i])
	}
	return "locus://" + strings.Join(parts, "/")
}
func ParseAddress(address string) (string, error) {
	if !strings.HasPrefix(address, "locus://") || strings.ContainsAny(address, "?#") {
		return "", errors.New("not a Locus address")
	}
	parts := strings.Split(strings.TrimPrefix(address, "locus://"), "/")
	for i, part := range parts {
		value, err := url.PathUnescape(part)
		if err != nil || strings.ContainsAny(value, "/\\") {
			return "", errors.New("invalid encoded path segment")
		}
		parts[i] = value
	}
	p := strings.Join(parts, "/")
	if !validNote(p) {
		return "", errors.New("invalid note path")
	}
	return p, nil
}

// regular rejects symlinks and special files, including in parent components.
// os.Root additionally prevents a concurrent replacement from escaping the root.
func (l *Library) regular(p string) error {
	parts := strings.Split(p, "/")
	for i := range parts {
		info, err := l.root.Lstat(filepath.FromSlash(strings.Join(parts[:i+1], "/")))
		if err != nil {
			return err
		}
		if info.Mode()&os.ModeSymlink != 0 {
			return errors.New("symlinks are not note resources")
		}
		if i < len(parts)-1 && !info.IsDir() {
			return errors.New("not a directory")
		}
		if i == len(parts)-1 && !info.Mode().IsRegular() {
			return errors.New("not a regular file")
		}
	}
	return nil
}

func (l *Library) Scan() ([]*Note, error)              { return l.scan("") }
func (l *Library) scan(parent string) ([]*Note, error) { return l.scanAll(parent, false) }
func (l *Library) scanAll(parent string, includeArchived bool) ([]*Note, error) {
	if strings.Count(parent, "/") >= 255 {
		return nil, errors.New("note hierarchy exceeds 256 levels")
	}
	dir := parent
	if dir == "" {
		dir = "."
	}
	f, err := l.root.Open(filepath.FromSlash(dir))
	if err != nil {
		return nil, err
	}
	entries, err := f.ReadDir(-1)
	f.Close()
	if err != nil {
		return nil, err
	}
	sort.Slice(entries, func(i, j int) bool { return entries[i].Name() < entries[j].Name() })
	var notes []*Note
	for _, entry := range entries {
		if !entry.IsDir() || !allowed(entry.Name()) {
			continue
		}
		p := path.Join(parent, entry.Name())
		note := &Note{Path: p, Title: entry.Name()}
		bodyPath := path.Join(p, "index.md")
		err := l.regular(bodyPath)
		if err == nil {
			f, err := l.root.Open(filepath.FromSlash(bodyPath))
			if err != nil {
				return nil, err
			}
			metadata, _, err := frontmatter(bufio.NewReader(f))
			f.Close()
			if err != nil {
				return nil, fmt.Errorf("%s: %w", bodyPath, err)
			}
			note.Archived = scalar(metadata, "archived") == "true"
			if note.Archived && !includeArchived {
				continue
			}
			if title := scalar(metadata, "title"); title != "" {
				note.Title = title
			}
			note.Summary = scalar(metadata, "summary")
			note.Icon = strings.TrimSpace(scalar(metadata, "icon"))
			note.HasBody = true
			for _, name := range []string{"index.html", "index.json"} {
				candidate := path.Join(p, "canvas", name)
				if l.regular(candidate) == nil {
					note.Canvas = candidate
					break
				}
			}
		} else if !errors.Is(err, os.ErrNotExist) {
			return nil, fmt.Errorf("%s: %w", bodyPath, err)
		}
		note.Children, err = l.scanAll(p, includeArchived)
		if err != nil {
			return nil, err
		}
		if note.HasBody || len(note.Children) > 0 {
			notes = append(notes, note)
		}
	}
	return notes, nil
}

func scalar(node *yaml.Node, key string) string {
	if node == nil || len(node.Content) == 0 {
		return ""
	}
	mapping := node.Content[0]
	if mapping.Kind != yaml.MappingNode {
		return ""
	}
	for i := 0; i+1 < len(mapping.Content); i += 2 {
		value := mapping.Content[i+1]
		if mapping.Content[i].Value == key && value.Kind == yaml.ScalarNode {
			if key == "archived" {
				if value.Tag != "!!bool" {
					return ""
				}
				return strings.ToLower(value.Value)
			}
			if value.Tag != "!!str" {
				return ""
			}
			return value.Value
		}
	}
	return ""
}

// frontmatter returns consumed non-frontmatter bytes as a body prefix. Reading
// the first line is bounded even when an ordinary body has no newline.
func frontmatter(r *bufio.Reader) (*yaml.Node, string, error) {
	var raw strings.Builder
	for {
		line, err := boundedLine(r, maxMetadata-raw.Len())
		if err != nil && !errors.Is(err, io.EOF) {
			return nil, "", err
		}
		raw.WriteString(line)
		if raw.Len() > maxMetadata {
			return nil, "", errors.New("metadata or first line exceeds 1 MiB")
		}
		if raw.Len() == len(line) {
			if strings.TrimSpace(strings.TrimPrefix(line, "\ufeff")) != "---" {
				return nil, strings.TrimPrefix(line, "\ufeff"), nil
			}
		} else if strings.TrimSpace(line) == "---" {
			text := raw.String()
			first := strings.IndexByte(text, '\n')
			var node yaml.Node
			if err := yaml.Unmarshal([]byte(text[first+1:len(text)-len(line)]), &node); err != nil {
				return nil, "", err
			}
			return &node, "", nil
		}
		if err != nil {
			if errors.Is(err, io.EOF) {
				return nil, strings.TrimPrefix(raw.String(), "\ufeff"), nil
			}
			return nil, "", err
		}
	}
}

func boundedLine(r *bufio.Reader, limit int) (string, error) {
	var line strings.Builder
	for {
		part, err := r.ReadSlice('\n')
		if line.Len()+len(part) > limit {
			return "", errors.New("metadata or first line exceeds 1 MiB")
		}
		line.Write(part)
		if !errors.Is(err, bufio.ErrBufferFull) {
			return line.String(), err
		}
	}
}

func (l *Library) Read(p string) (Document, error) {
	if !validNote(p) {
		return Document{}, errors.New("invalid note path")
	}
	target := path.Join(p, "index.md")
	if err := l.regular(target); err != nil {
		return Document{}, err
	}
	f, err := l.root.Open(filepath.FromSlash(target))
	if err != nil {
		return Document{}, err
	}
	defer f.Close()
	r := bufio.NewReader(io.LimitReader(f, maxBody+1))
	metadata, prefix, err := frontmatter(r)
	if err != nil {
		return Document{}, err
	}
	rest, err := io.ReadAll(r)
	if err != nil {
		return Document{}, err
	}
	// Stat also counts frontmatter, which is excluded from the returned body.
	info, err := f.Stat()
	if err != nil {
		return Document{}, err
	}
	if info.Size() > maxBody || len(prefix)+len(rest) > maxBody {
		return Document{}, errors.New("note exceeds 32 MiB")
	}
	return Document{Body: prefix + string(rest), Metadata: metadata}, nil
}

func Find(notes []*Note, p string) *Note {
	for _, note := range notes {
		if note.Path == p {
			return note
		}
		if found := Find(note.Children, p); found != nil {
			return found
		}
	}
	return nil
}
