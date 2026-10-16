// Package preferences owns YAML persistence, preserving unrelated fields and
// refusing to overwrite a settings file modified outside this process.
package preferences

import (
	"bytes"
	"errors"
	"io"
	"os"
	"path/filepath"
	"sync"

	"go.yaml.in/yaml/v3"
)

type Config struct {
	Library  string `yaml:"library_path,omitempty"`
	Theme    string `yaml:"theme"`
	Language string `yaml:"language"`
	Editor   string `yaml:"editor"`
	Terminal string `yaml:"terminal"`
	Agent    string `yaml:"agent"`
	Shell    string `yaml:"agent_shell"`
	Export   string `yaml:"export_directory,omitempty"`
	MCP      bool   `yaml:"mcp_enabled"`
	Port     int    `yaml:"mcp_port"`
}

// decodeDocument rejects extra documents instead of silently discarding them on save.
func decodeDocument(b []byte, value any) error {
	decoder := yaml.NewDecoder(bytes.NewReader(b))
	if err := decoder.Decode(value); err != nil {
		if errors.Is(err, io.EOF) {
			return nil
		}
		return err
	}
	var extra yaml.Node
	if err := decoder.Decode(&extra); !errors.Is(err, io.EOF) {
		if err != nil {
			return err
		}
		return errors.New("configuration must contain only one YAML document")
	}
	return nil
}

func Default() Config {
	return Config{Theme: "system", Language: "system", Editor: "system", Terminal: "system", Agent: "codex", Shell: "default", Port: 2301}
}

type Store struct {
	mu       sync.Mutex
	path     string
	original []byte
	fields   map[string]any
}

func Load(path string) (Config, *Store, error) {
	cfg := Default()
	b, err := os.ReadFile(path)
	if errors.Is(err, os.ErrNotExist) {
		err = nil
	}
	if err != nil {
		return cfg, nil, err
	}
	var fields map[string]any
	if len(b) > 0 {
		if err = decodeDocument(b, &fields); err != nil {
			return cfg, nil, err
		}
		if err = decodeDocument(b, &cfg); err != nil {
			return Default(), nil, err
		}
	}
	if fields == nil {
		fields = map[string]any{}
	}
	if cfg.Port < 1 || cfg.Port > 65535 {
		cfg.Port = 2301
	}
	return cfg, &Store{path: path, original: b, fields: fields}, nil
}
func (s *Store) Save(cfg Config) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	current, err := os.ReadFile(s.path)
	if err != nil && !errors.Is(err, os.ErrNotExist) {
		return err
	}
	if !bytes.Equal(current, s.original) {
		return errors.New("settings changed outside Locus; restart before saving")
	}
	b, err := yaml.Marshal(cfg)
	if err != nil {
		return err
	}
	var updated map[string]any
	if err = yaml.Unmarshal(b, &updated); err != nil {
		return err
	}
	fields := make(map[string]any, len(s.fields))
	for k, v := range s.fields {
		fields[k] = v
	}
	// Empty optional values and removed legacy keys delete their fields, not unrelated ones.
	for _, key := range []string{"library_path", "export_directory", "mcp_local_network"} {
		delete(fields, key)
	}
	for key, value := range updated {
		fields[key] = value
	}
	b, err = yaml.Marshal(fields)
	if err != nil {
		return err
	}
	if err = AtomicWrite(s.path, b); err != nil {
		return err
	}
	s.original = b
	s.fields = fields
	return nil
}
func AtomicWrite(path string, data []byte) error {
	if info, err := os.Lstat(path); err == nil && !info.Mode().IsRegular() {
		return errors.New("persistence target is not a regular file")
	} else if err != nil && !errors.Is(err, os.ErrNotExist) {
		return err
	}
	if err := os.MkdirAll(filepath.Dir(path), 0700); err != nil {
		return err
	}
	f, err := os.CreateTemp(filepath.Dir(path), ".locus-*")
	if err != nil {
		return err
	}
	defer os.Remove(f.Name())
	if _, err = f.Write(data); err != nil {
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
	return os.Rename(f.Name(), path)
}

type State struct {
	ExpandedNodes    map[string][]string `yaml:"expanded_nodes,omitempty"`
	Library          string              `yaml:"library"`
	Selected         string              `yaml:"selected"`
	SidebarWidth     float32             `yaml:"sidebar_width"`
	Collapsed        bool                `yaml:"sidebar_collapsed"`
	GraphAttachments bool                `yaml:"graph_attachments,omitempty"`
}

func LoadState(path string) (State, error) {
	var s State
	b, err := os.ReadFile(path)
	if errors.Is(err, os.ErrNotExist) {
		return s, nil
	}
	if err != nil {
		return s, err
	}
	err = decodeDocument(b, &s)
	return s, err
}
func SaveState(path string, s State) error {
	b, err := yaml.Marshal(s)
	if err != nil {
		return err
	}
	return AtomicWrite(path, b)
}

// Writer serializes snapshots away from the UI thread and flushes on Close.
// Submit and Close belong to the same owner; Report may run on a worker.
type Writer struct {
	queue chan Config
	done  chan struct{}
}

func NewWriter(store *Store, report func(error)) *Writer {
	w := &Writer{queue: make(chan Config, 1), done: make(chan struct{})}
	go func() {
		defer close(w.done)
		for cfg := range w.queue {
			report(store.Save(cfg))
		}
	}()
	return w
}
func (w *Writer) Submit(cfg Config) {
	select {
	case w.queue <- cfg:
	default:
		select {
		case <-w.queue:
		default:
		}
		w.queue <- cfg
	}
}
func (w *Writer) Close() {
	if w != nil {
		close(w.queue)
		<-w.done
	}
}
