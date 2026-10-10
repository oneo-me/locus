// Package preferences owns TOML persistence, preserving unrelated fields and
// refusing to overwrite a settings file modified outside this process.
package preferences

import (
	"bytes"
	"errors"
	"os"
	"path/filepath"
	"sync"

	"github.com/pelletier/go-toml/v2"
)

type Config struct {
	Library  string `toml:"library_path,omitempty"`
	Theme    string `toml:"theme"`
	Language string `toml:"language"`
	Editor   string `toml:"editor"`
	Terminal string `toml:"terminal"`
	Agent    string `toml:"agent"`
	Shell    string `toml:"agent_shell"`
	Export   string `toml:"export_directory,omitempty"`
	MCP      bool   `toml:"mcp_enabled"`
	Network  bool   `toml:"mcp_local_network"`
	Port     int    `toml:"mcp_port"`
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
		if err = toml.Unmarshal(b, &fields); err != nil {
			return cfg, nil, err
		}
		if err = toml.Unmarshal(b, &cfg); err != nil {
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
	b, err := toml.Marshal(cfg)
	if err != nil {
		return err
	}
	var updated map[string]any
	if err = toml.Unmarshal(b, &updated); err != nil {
		return err
	}
	fields := make(map[string]any, len(s.fields))
	for k, v := range s.fields {
		fields[k] = v
	}
	// Empty optional values remove their corresponding fields, not unrelated ones.
	for _, key := range []string{"library_path", "export_directory"} {
		delete(fields, key)
	}
	for key, value := range updated {
		fields[key] = value
	}
	b, err = toml.Marshal(fields)
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
	Library      string  `toml:"library"`
	Selected     string  `toml:"selected"`
	SidebarWidth float32 `toml:"sidebar_width"`
	Collapsed    bool    `toml:"sidebar_collapsed"`
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
	err = toml.Unmarshal(b, &s)
	return s, err
}
func SaveState(path string, s State) error {
	b, err := toml.Marshal(s)
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
