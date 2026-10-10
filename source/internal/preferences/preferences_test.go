package preferences

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestPreserveAndConflict(t *testing.T) {
	p := filepath.Join(t.TempDir(), "settings.yaml")
	original := []byte("library_path: '/tmp/中文'\nui_font_size: 18\nagent: 'pi'\n")
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
	if !strings.Contains(string(b), "ui_font_size: 18") {
		t.Fatal("unrelated field lost")
	}
	if err = os.WriteFile(p, []byte("theme: 'light'\n"), 0600); err != nil {
		t.Fatal(err)
	}
	if err = store.Save(cfg); err == nil {
		t.Fatal("external change overwritten")
	}
}
func TestMalformedAndSymlink(t *testing.T) {
	p := filepath.Join(t.TempDir(), "settings.yaml")
	if err := os.WriteFile(p, []byte("invalid: ["), 0600); err != nil {
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
func TestYAMLRoundTrip(t *testing.T) {
	p := filepath.Join(t.TempDir(), "settings.yaml")
	cfg, store, err := Load(p)
	if err != nil || cfg != Default() {
		t.Fatalf("defaults: %+v %v", cfg, err)
	}
	cfg.Library = "/tmp/中文: notes"
	cfg.Export = "/tmp/export"
	cfg.MCP, cfg.Network, cfg.Port = true, true, 3456
	if err := store.Save(cfg); err != nil {
		t.Fatal(err)
	}
	got, store, err := Load(p)
	if err != nil || got != cfg {
		t.Fatalf("round trip: %+v %v", got, err)
	}
	cfg.Library, cfg.Export = "", ""
	if err := store.Save(cfg); err != nil {
		t.Fatal(err)
	}
	got, _, err = Load(p)
	if err != nil || got != cfg {
		t.Fatalf("clear optional fields: %+v %v", got, err)
	}
}

func TestNestedYAMLFields(t *testing.T) {
	p := filepath.Join(t.TempDir(), "settings.yaml")
	if err := os.WriteFile(p, []byte("custom:\n  list: [one, two]\n  enabled: true\n"), 0600); err != nil {
		t.Fatal(err)
	}
	cfg, store, err := Load(p)
	if err != nil {
		t.Fatal(err)
	}
	if err := store.Save(cfg); err != nil {
		t.Fatal(err)
	}
	_, store, err = Load(p)
	if err != nil {
		t.Fatal(err)
	}
	custom := store.fields["custom"].(map[string]any)
	if custom["enabled"] != true || len(custom["list"].([]any)) != 2 {
		t.Fatalf("unknown nested fields lost: %+v", custom)
	}
}

func TestInvalidYAML(t *testing.T) {
	for _, input := range []string{"theme: [", "theme: dark\ntheme: light\n", "mcp_port: wrong\n", "- dark\n", "theme: dark\n---\ntheme: light\n"} {
		t.Run(input, func(t *testing.T) {
			p := filepath.Join(t.TempDir(), "settings.yaml")
			if err := os.WriteFile(p, []byte(input), 0600); err != nil {
				t.Fatal(err)
			}
			if _, store, err := Load(p); err == nil || store != nil {
				t.Fatal("invalid YAML writable")
			}
			b, err := os.ReadFile(p)
			if err != nil || string(b) != input {
				t.Fatal("invalid YAML changed")
			}
		})
	}
}

func TestStateYAML(t *testing.T) {
	p := filepath.Join(t.TempDir(), "state.yaml")
	if got, err := LoadState(p); err != nil || got != (State{}) {
		t.Fatalf("missing state: %+v %v", got, err)
	}
	want := State{Library: "/tmp/中文", Selected: "note", SidebarWidth: 234.5, Collapsed: true}
	if err := SaveState(p, want); err != nil {
		t.Fatal(err)
	}
	if got, err := LoadState(p); err != nil || got != want {
		t.Fatalf("state round trip: %+v %v", got, err)
	}
	for _, input := range []string{"sidebar_width: [", "library: notes\n---\nselected: note\n"} {
		if err := os.WriteFile(p, []byte(input), 0600); err != nil {
			t.Fatal(err)
		}
		if _, err := LoadState(p); err == nil {
			t.Fatal("invalid state accepted")
		}
	}
}

func TestWriterFlush(t *testing.T) {
	p := filepath.Join(t.TempDir(), "settings.yaml")
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
