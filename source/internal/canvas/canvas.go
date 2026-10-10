// Package canvas serves note-owned pages through the native browser's custom
// protocol. It never exposes a TCP listener or arbitrary filesystem paths.
package canvas

import (
	"archive/zip"
	"bytes"
	"crypto/rand"
	_ "embed"
	"encoding/json"
	"errors"
	"fmt"
	"html"
	"mime"
	"net/http"
	"os"
	"path"
	"path/filepath"
	"sort"
	"strings"
	"sync"

	"github.com/oneo-me/locus/internal/library"
)

//go:embed vendor/tailwind.js
var tailwind []byte

//go:embed runtime.js
var runtimeJS string

const Scheme = "locus-canvas"
const policy = "default-src 'none'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob: https:; font-src 'self' data:; connect-src 'self' https:; worker-src 'self' blob:; frame-src 'none'; object-src 'none'; form-action 'none'; base-uri 'self'"

type Router struct {
	mu       sync.RWMutex
	sessions map[string]*Session
	closed   bool
}
type Session struct {
	mu                                    sync.Mutex
	lib                                   *library.Library
	token, source, title, fragment, owner string
	data                                  json.RawMessage
	loaded                                map[string][]byte
	loadedBytes                           int
	overflow                              bool
	closed                                bool
}

func (r *Router) Open(root, note, title string) (*Session, error) {
	lib, err := library.Open(root)
	if err != nil {
		return nil, err
	}
	s := &Session{lib: lib, token: strings.ToLower(rand.Text()), source: note, owner: note, title: title, data: json.RawMessage("null"), loaded: map[string][]byte{}}
	if err = s.prepare(note); err != nil {
		lib.Close()
		return nil, err
	}
	r.mu.Lock()
	defer r.mu.Unlock()
	if r.closed {
		lib.Close()
		return nil, errors.New("canvas router is closed")
	}
	if r.sessions == nil {
		r.sessions = map[string]*Session{}
	}
	r.sessions[s.token] = s
	return s, nil
}
func (s *Session) prepare(note string) error {
	s.source = note
	s.data = json.RawMessage("null")
	htmlPath, jsonPath := path.Join(note, "canvas/index.html"), path.Join(note, "canvas/index.json")
	hasHTML, hasJSON := s.lib.Exists(htmlPath), s.lib.Exists(jsonPath)
	if hasHTML == hasJSON {
		return errors.New("canvas requires exactly one index.html or index.json")
	}
	if hasJSON {
		b, err := s.lib.Resource(jsonPath)
		if err != nil {
			return err
		}
		var fields map[string]json.RawMessage
		if err = json.Unmarshal(b, &fields); err != nil {
			return err
		}
		if len(fields) != 2 || fields["template"] == nil || fields["data"] == nil {
			return errors.New("canvas invocation requires exactly template and data")
		}
		var address string
		if err = json.Unmarshal(fields["template"], &address); err != nil {
			return err
		}
		s.source, err = library.ParseAddress(address)
		if err != nil {
			return err
		}
		if s.lib.Exists(path.Join(s.source, "canvas/index.json")) {
			return errors.New("template chains are not supported")
		}
		s.data = fields["data"]
	}
	fragment, err := s.lib.Resource(path.Join(s.source, "canvas/index.html"))
	if err != nil {
		return err
	}
	if len(bytes.TrimSpace(fragment)) == 0 {
		return errors.New("canvas fragment is empty")
	}
	if _, err = s.lib.Resource(path.Join(s.source, "canvas/main.js")); err != nil {
		return fmt.Errorf("canvas requires main.js: %w", err)
	}
	s.fragment = string(fragment)
	return nil
}
func (r *Router) CloseAll() {
	r.mu.Lock()
	r.closed = true
	sessions := r.sessions
	r.sessions = nil
	r.mu.Unlock()
	for _, s := range sessions {
		r.Close(s)
	}
}

func (s *Session) URL() string    { return Scheme + "://" + s.token + "/view" }
func (s *Session) prefix() string { return Scheme + "://" + s.token + "/" }
func (r *Router) Close(s *Session) {
	if s == nil {
		return
	}
	r.mu.Lock()
	delete(r.sessions, s.token)
	r.mu.Unlock()
	s.mu.Lock()
	defer s.mu.Unlock()
	if !s.closed {
		s.closed = true
		s.lib.Close()
	}
}
func encode(p string) string { return strings.TrimPrefix(library.Address(p), "locus://") }

func (s *Session) page(export bool) []byte {
	root := s.prefix()
	if export {
		root = "./"
	}
	base := root + "files/" + encode(s.source) + "/canvas/"
	config, _ := json.Marshal(map[string]any{"root": root, "main": "files/" + encode(s.source) + "/canvas/main.js", "data": s.data})
	// JSON escaping prevents template data from closing the bootstrap script.
	bootstrap := strings.Replace(runtimeJS, "__LOCUS_CONFIG__", string(config), 1)
	return []byte("<!doctype html><html><head><meta charset=\"utf-8\"><meta http-equiv=\"Content-Security-Policy\" content=\"" + html.EscapeString(policy) + "\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>" + html.EscapeString(s.title) + "</title><base href=\"" + html.EscapeString(base) + "\"><style>html,body{height:100%;margin:0;color-scheme:light dark;font:16px system-ui}body{background:light-dark(#fafaf8,#22211e);color:light-dark(#33332e,#eeece3)}.locus-error{padding:24px;white-space:pre-wrap}</style><script>" + strings.ReplaceAll(string(tailwind), "</script", "<\\/script") + "</script><script>" + bootstrap + "</script></head><body>" + s.fragment + "</body></html>")
}
func (r *Router) ServeHTTP(w http.ResponseWriter, req *http.Request) {
	if req.Method != http.MethodGet && req.Method != http.MethodHead {
		w.WriteHeader(405)
		return
	}
	r.mu.RLock()
	s := r.sessions[req.URL.Host]
	r.mu.RUnlock()
	if s == nil {
		http.NotFound(w, req)
		return
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	if s.closed {
		http.NotFound(w, req)
		return
	}
	w.Header().Set("Cache-Control", "no-store")
	w.Header().Set("X-Content-Type-Options", "nosniff")
	w.Header().Set("Content-Security-Policy", policy)
	if req.URL.Path == "/view" {
		if err := s.prepare(s.owner); err != nil {
			http.Error(w, err.Error(), 500)
			return
		}
		s.loaded = map[string][]byte{}
		s.loadedBytes = 0
		s.overflow = false
		w.Header().Set("Content-Type", "text/html; charset=utf-8")
		if req.Method != http.MethodHead {
			w.Write(s.page(false))
		}
		return
	}
	encoded, ok := strings.CutPrefix(req.URL.EscapedPath(), "/files/")
	if !ok {
		http.NotFound(w, req)
		return
	}
	p, err := library.DecodeResource(encoded)
	if err != nil {
		http.NotFound(w, req)
		return
	}
	data, err := s.lib.Resource(p)
	if err != nil {
		http.NotFound(w, req)
		return
	}
	part := req.URL.Query().Get("part")
	if part != "" && path.Base(p) != "index.md" {
		http.NotFound(w, req)
		return
	}
	switch part {
	case "":
	case "yaml":
		data, _ = library.Parts(data)
	case "body":
		_, data = library.Parts(data)
	default:
		http.NotFound(w, req)
		return
	}
	contentType := mime.TypeByExtension(path.Ext(p))
	if contentType == "" {
		contentType = "application/octet-stream"
	}
	if part != "" {
		contentType = "text/plain; charset=utf-8"
	}
	w.Header().Set("Content-Type", contentType)
	// Keep bounded snapshots for export; requests themselves always read disk.
	key := p
	if part != "" {
		key += ".locus-" + part
	}
	if s.loadedBytes-len(s.loaded[key])+len(data) <= 64<<20 {
		s.loadedBytes += len(data) - len(s.loaded[key])
		s.loaded[key] = bytes.Clone(data)
	} else {
		s.overflow = true
	}
	if req.Method != http.MethodHead {
		w.Write(data)
	}
}

// Export snapshots resources requested by this page. It never crawls unrelated
// notes or overwrites an existing export. Browser file-origin module limitations
// still apply after unpacking; serve canvas.html over local HTTP when necessary.
func (s *Session) snapshot() ([]byte, map[string][]byte, error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	if s.closed {
		return nil, nil, errors.New("canvas was closed")
	}
	if s.overflow {
		return nil, nil, errors.New("loaded resources exceed the 64 MiB export limit")
	}
	if _, ok := s.loaded[path.Join(s.source, "canvas/main.js")]; !ok {
		return nil, nil, errors.New("wait for the canvas to load before exporting")
	}
	loaded := make(map[string][]byte, len(s.loaded))
	for key, data := range s.loaded {
		loaded[key] = data
	} // Stored snapshots are immutable.
	return s.page(true), loaded, nil
}

func (s *Session) Export(directory string) (string, error) {
	page, loaded, err := s.snapshot()
	if err != nil {
		return "", err
	}
	if err := os.MkdirAll(directory, 0700); err != nil {
		return "", err
	}
	f, err := os.CreateTemp(directory, "Locus-canvas-*.zip")
	if err != nil {
		return "", err
	}
	target := f.Name()
	success := false
	defer func() {
		f.Close()
		if !success {
			os.Remove(target)
		}
	}()
	z := zip.NewWriter(f)
	add := func(name string, data []byte) error {
		w, err := z.Create(name)
		if err == nil {
			_, err = w.Write(data)
		}
		return err
	}
	if err = add("canvas.html", page); err != nil {
		return "", err
	}
	keys := make([]string, 0, len(loaded))
	for key := range loaded {
		keys = append(keys, key)
	}
	sort.Strings(keys)
	for _, key := range keys {
		if err = add("files/"+key, loaded[key]); err != nil {
			return "", err
		}
	}
	if err = add("README.txt", []byte("Open canvas.html using a local HTTP server. Only loaded resources are included. Online services still require network access.\n")); err != nil {
		return "", err
	}
	if err = add("TAILWIND-LICENSE.txt", tailwindLicense); err != nil {
		return "", err
	}
	if err = z.Close(); err != nil {
		return "", err
	}
	if err = f.Sync(); err != nil {
		return "", err
	}
	if err = f.Close(); err != nil {
		return "", err
	}
	success = true
	return filepath.Clean(target), nil
}

//go:embed vendor/LICENSE
var tailwindLicense []byte

var _ http.Handler = (*Router)(nil)
