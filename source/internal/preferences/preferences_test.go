package preferences

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestPreserveAndConflict(t *testing.T) {
	p := filepath.Join(t.TempDir(), "settings.toml")
	original := []byte("library_path = '/tmp/中文'\nui_font_size = 18\nagent = 'pi'\n")
	if err := os.WriteFile(p, original, 0600); err != nil {
		t.Fatal(err)
	}
	cfg, store, err := Load(p)
	if err != nil || cfg.Library != "/tmp/中文" || cfg.Port != 2301 {
		t.Fatalf("load: %+v %v", cfg, err)
	}
	cfg.Theme = "dark"
	if err = store.Save(cfg); err != nil {
		t.Fatal(err)
	}
	b, _ := os.ReadFile(p)
	if !strings.Contains(string(b), "ui_font_size = 18") {
		t.Fatal("unrelated field lost")
	}
	if err = os.WriteFile(p, []byte("theme = 'light'\n"), 0600); err != nil {
		t.Fatal(err)
	}
	if err = store.Save(cfg); err == nil {
		t.Fatal("external change overwritten")
	}
}
func TestMalformedAndSymlink(t *testing.T) {
	p := filepath.Join(t.TempDir(), "settings.toml")
	if err := os.WriteFile(p, []byte("invalid=["), 0600); err != nil {
		t.Fatal(err)
	}
	if _, store, err := Load(p); err == nil || store != nil {
		t.Fatal("malformed file writable")
	}
	link := filepath.Join(t.TempDir(), "link")
	if err := os.Symlink(p, link); err == nil {
		if err := AtomicWrite(link, []byte("replacement")); err == nil {
			t.Fatal("symlink overwritten")
		}
	}
}
func TestWriterFlush(t *testing.T) {
	p := filepath.Join(t.TempDir(), "settings.toml")
	cfg, store, err := Load(p)
	if err != nil {
		t.Fatal(err)
	}
	failures := make(chan error, 100)
	w := NewWriter(store, func(err error) {
		if err != nil {
			failures <- err
		}
	})
	for _, theme := range []string{"light", "dark", "system"} {
		cfg.Theme = theme
		w.Submit(cfg)
	}
	w.Close()
	close(failures)
	for err := range failures {
		t.Error(err)
	}
	result, _, err := Load(p)
	if err != nil || result.Theme != "system" {
		t.Fatalf("final state: %+v %v", result, err)
	}
}
