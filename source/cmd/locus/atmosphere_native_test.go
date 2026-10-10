package main

import (
	"bytes"
	"fmt"
	"image/png"
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/egoist/mygo"
)

// Capture the same reference titles at narrow and wide reading widths. This
// exercises native painting, not a claim of pixel identity with GPUI captures.
func TestNativeAtmosphere(t *testing.T) {
	if os.Getenv("LOCUS_NATIVE_SMOKE") != "1" {
		t.Skip("requires a native desktop session")
	}
	root := t.TempDir()
	var fixtures []atmosphereReference
	for _, f := range atmosphereReferences(t) {
		if !f.Dark && f.Title != "" {
			fixtures = append(fixtures, f)
		}
	}
	for i, f := range fixtures {
		dir := filepath.Join(root, fmt.Sprintf("note-%d", i))
		if err := os.Mkdir(dir, 0700); err != nil {
			t.Fatal(err)
		}
		body := fmt.Sprintf("---\ntitle: %q\nsummary: A quiet, title-derived atmosphere. 中文描述。\nstatus: draft\ncontacts:\n  website: https://example.com\ntargets: [macOS, Windows]\n---\n# Reference article\n\nReadable content below the shared metadata panel.\n", f.Title)
		if err := os.WriteFile(filepath.Join(dir, "index.md"), []byte(body), 0600); err != nil {
			t.Fatal(err)
		}
	}
	a := &application{cursor: -1}
	a.model.Dispatch = mygo.RunOnMain
	a.model.Changed = func() { a.shell.Window().Invalidate() }
	mygo.RunOnMain(func() { a.start(root) })
	defer mygo.RunOnMain(func() { a.shell.Window().Destroy() })
	waitNative(t, func() bool { return !a.model.Loading })
	for _, width := range []int{980, 1320} {
		mygo.RunOnMain(func() { a.shell.Window().SetSize(width, 820) })
		for _, mode := range []mygo.ThemeSource{mygo.ThemeLight, mygo.ThemeDark} {
			mygo.RunOnMain(func() { mygo.Theme.SetSource(mode); a.shell.Window().Invalidate() })
			for i := range fixtures {
				mygo.RunOnMain(func() { a.selectNote(fmt.Sprintf("note-%d", i)) })
				waitNative(t, func() bool { return a.model.Document.Body != "" })
				time.Sleep(250 * time.Millisecond)
				data, err := a.shell.Window().CapturePage()
				if err != nil {
					t.Fatal(err)
				}
				im, err := png.Decode(bytes.NewReader(data))
				if err != nil || im.Bounds().Dx() < width {
					t.Fatalf("invalid capture: %v", err)
				}
				if dir := os.Getenv("LOCUS_SCREENSHOTS"); dir != "" {
					if err := os.MkdirAll(dir, 0700); err != nil {
						t.Fatal(err)
					}
					if err := os.WriteFile(filepath.Join(dir, fmt.Sprintf("atmosphere-%02d-%d-%s.png", i, width, mode)), data, 0600); err != nil {
						t.Fatal(err)
					}
				}
			}
		}
	}
}
