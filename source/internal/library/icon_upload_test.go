package library

import (
	"bytes"
	"image"
	"image/png"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestIconUpload(t *testing.T) {
	root := t.TempDir()
	note := filepath.Join(root, "note")
	if err := os.Mkdir(note, 0755); err != nil {
		t.Fatal(err)
	}
	original := "\ufeff---\r\ntitle: Keep # comment\r\nicon: '😀'\r\ncustom: [a, b]\r\n---\r\nBody\r\n"
	if err := os.WriteFile(filepath.Join(note, "index.md"), []byte(original), 0640); err != nil {
		t.Fatal(err)
	}
	var input bytes.Buffer
	if err := png.Encode(&input, image.NewNRGBA(image.Rect(0, 0, 2048, 1024))); err != nil {
		t.Fatal(err)
	}
	source := filepath.Join(root, "source.png")
	os.WriteFile(source, input.Bytes(), 0600)
	data, err := PrepareIcon(source)
	if err != nil {
		t.Fatal(err)
	}
	cfg, err := png.DecodeConfig(bytes.NewReader(data))
	if err != nil || cfg.Width != 64 || cfg.Height != 32 {
		t.Fatalf("config: %+v, %v", cfg, err)
	}
	lib, err := Open(root)
	if err != nil {
		t.Fatal(err)
	}
	defer lib.Close()
	if err := lib.SetIcon("note", data); err != nil {
		t.Fatal(err)
	}
	got, _ := os.ReadFile(filepath.Join(note, "index.md"))
	if string(got) != strings.Replace(original, "icon: '😀'", "icon: assets/icon.png", 1) {
		t.Fatalf("modified unrelated source: %q", got)
	}
	asset, _ := lib.Resource("note/assets/icon.png")
	if !bytes.Equal(asset, data) {
		t.Fatal("icon not written")
	}
	if err := lib.SetIcon("../escape", data); err == nil {
		t.Fatal("accepted traversal")
	}
	os.Remove(filepath.Join(note, "assets/icon.png"))
	os.Symlink(source, filepath.Join(note, "assets/icon.png"))
	if err := lib.SetIcon("note", data); err == nil {
		t.Fatal("accepted symlink")
	}
}

func TestIconFrontmatter(t *testing.T) {
	for _, input := range []string{"Body\n", "---\ntitle: Keep\n---\nBody\n", "---\n---\nBody\n"} {
		got, err := iconFrontmatter([]byte(input))
		if err != nil {
			t.Fatal(err)
		}
		raw, body := Parts(got)
		if !bytes.Contains(raw, []byte("icon: assets/icon.png\n")) || string(body) != "Body\n" {
			t.Fatalf("%q", got)
		}
	}
	for _, input := range []string{"---\nicon: |\n  hello\n---\nBody", "---\n{icon: old, title: keep}\n---\nBody", "---\nicon: [a,b]\n---\nBody", "---\nicon: old\nicon: duplicate\n---\nBody", "---\nmissing end"} {
		if _, err := iconFrontmatter([]byte(input)); err == nil {
			t.Fatalf("accepted %q", input)
		}
	}
}

func TestPrepareIconLimits(t *testing.T) {
	p := filepath.Join(t.TempDir(), "image.png")
	os.WriteFile(p, []byte("invalid"), 0600)
	if _, err := PrepareIcon(p); err == nil {
		t.Fatal("accepted invalid PNG")
	}
	f, _ := os.Create(p)
	f.Truncate((16 << 20) + 1)
	f.Close()
	if _, err := PrepareIcon(p); err == nil {
		t.Fatal("accepted oversized file")
	}
	for _, size := range []image.Point{{12, 6}, {1, 2000}, {5000, 4000}} {
		var b bytes.Buffer
		png.Encode(&b, image.NewGray(image.Rectangle{Max: size}))
		os.WriteFile(p, b.Bytes(), 0600)
		data, err := PrepareIcon(p)
		if size.X*size.Y > 16_000_000 {
			if err == nil {
				t.Fatal("accepted excessive pixels")
			}
			continue
		}
		if err != nil {
			t.Fatal(err)
		}
		cfg, _ := png.DecodeConfig(bytes.NewReader(data))
		if size.X == 12 && (cfg.Width != 12 || cfg.Height != 6) {
			t.Fatal("small icon enlarged")
		}
		if size.X == 1 && (cfg.Width != 1 || cfg.Height != 64) {
			t.Fatal("thin icon lost")
		}
	}
}
