package library

import (
	"errors"
	"io"
	"net/url"
	"os"
	"path"
	"strings"
	"unicode/utf8"
)

// DecodeResource decodes segments exactly once, never normalizing traversal.
func DecodeResource(encoded string) (string, error) {
	parts := strings.Split(encoded, "/")
	for i, part := range parts {
		value, err := url.PathUnescape(part)
		if err != nil || !utf8.ValidString(value) || value == "" || value == "." || value == ".." || value == "node_modules" || strings.ContainsAny(value, "/\\\x00") {
			return "", errors.New("invalid resource path")
		}
		parts[i] = value
	}
	return strings.Join(parts, "/"), nil
}

// Resource accepts only a note body or files owned by its assets/canvas tree.
// Hidden files inside resource trees are valid; hidden note paths are not.
func (l *Library) Resource(p string) ([]byte, error) {
	parts := strings.Split(p, "/")
	note := ""
	for i, part := range parts {
		if part == "assets" || part == "canvas" {
			if i == 0 || i == len(parts)-1 {
				return nil, errors.New("resource is not a file")
			}
			note = strings.Join(parts[:i], "/")
			break
		}
	}
	if note == "" && path.Base(p) == "index.md" {
		note = path.Dir(p)
	}
	if !validNote(note) {
		return nil, errors.New("resource has no owning note")
	}
	for _, part := range parts {
		if part == "" || part == "." || part == ".." || part == "node_modules" || strings.ContainsAny(part, "\\\x00") || !utf8.ValidString(part) {
			return nil, errors.New("unsafe resource path")
		}
	}
	if err := l.regular(path.Join(note, "index.md")); err != nil {
		return nil, err
	}
	if err := l.regular(p); err != nil {
		return nil, err
	}
	f, err := l.root.Open(p)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	data, err := io.ReadAll(io.LimitReader(f, maxBody+1))
	if err == nil && len(data) > maxBody {
		err = errors.New("resource exceeds 32 MiB")
	}
	return data, err
}

func (l *Library) Exists(p string) bool { return l.regular(p) == nil }

// Parts preserves the exact YAML/body source rather than reserializing values.
func Parts(data []byte) (yamlSource, body []byte) {
	data = []byte(strings.TrimPrefix(string(data), "\ufeff"))
	text := string(data)
	end := strings.IndexByte(text, '\n')
	if end < 0 || strings.TrimSpace(text[:end]) != "---" {
		return nil, data
	}
	for start := end + 1; start < len(text); {
		next := strings.IndexByte(text[start:], '\n')
		stop := len(text)
		if next >= 0 {
			stop = start + next
		}
		if strings.TrimSpace(text[start:stop]) == "---" {
			after := stop
			if after < len(text) {
				after++
			}
			return data[end+1 : start], data[after:]
		}
		if next < 0 {
			break
		}
		start = stop + 1
	}
	return nil, data
}

// ResolvePath is for native file actions only, after the rooted checks succeed.
func (l *Library) ResolvePath(note string) (string, error) {
	if !validNote(note) {
		return "", errors.New("invalid note path")
	}
	if err := l.regular(path.Join(note, "index.md")); err != nil {
		return "", err
	}
	return l.Path + string(os.PathSeparator) + strings.ReplaceAll(note, "/", string(os.PathSeparator)), nil
}
