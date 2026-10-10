package main

import (
	"bytes"
	"fmt"
	"image/png"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/egoist/mygo"
)

func TestMain(m *testing.M) {
	if os.Getenv("LOCUS_NATIVE_SMOKE") != "1" {
		os.Exit(m.Run())
	}
	directory, err := os.MkdirTemp("", "locus-native-")
	if err != nil {
		panic(err)
	}
	mygo.App.SetName("Locus Go Test")
	mygo.App.SetPath(mygo.PathUserData, directory)
	mygo.App.OnWindowAllClosed(func() {})
	result := make(chan int, 1)
	mygo.App.WhenReady(func() { go func() { result <- m.Run(); mygo.App.Quit() }() })
	if err := mygo.App.Run(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	code := <-result
	os.RemoveAll(directory)
	os.Exit(code)
}

func TestNativeReading(t *testing.T) {
	if os.Getenv("LOCUS_NATIVE_SMOKE") != "1" {
		t.Skip("requires a native desktop session")
	}
	root := t.TempDir()
	if err := os.Mkdir(filepath.Join(root, "note"), 0700); err != nil {
		t.Fatal(err)
	}
	body := "---\ntitle: 中文 Reading test\nsummary: Native Go and Onevra\nicon: 👩🏽‍💻\ntags: [native, 中文]\nstatus: draft\n---\n# Markdown\n\n中文与 English `inline code` and **bold text**.\n\n| Name | Value |\n| --- | --- |\n| 中文 | A table |\n\n```go\nfmt.Println(\"Locus\")\n```\n"
	body += strings.Repeat("\nA paragraph for scrolling beneath the titlebar.\n", 60)
	if err := os.WriteFile(filepath.Join(root, "note/index.md"), []byte(body), 0600); err != nil {
		t.Fatal(err)
	}
	if err := os.MkdirAll(filepath.Join(root, "picture/assets"), 0700); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(root, "picture/index.md"), []byte("---\ntitle: Picture icon\nicon: assets/icon.png\n---\nImage note"), 0600); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(root, "picture/assets/icon.png"), iconPNG(t, 64, 32), 0600); err != nil {
		t.Fatal(err)
	}
	a := &application{cursor: -1}
	a.model.Dispatch = mygo.RunOnMain
	a.model.Changed = func() { a.shell.Window().Invalidate() }
	mygo.RunOnMain(func() { a.start(root) })
	defer mygo.RunOnMain(func() { a.shell.Window().Destroy() })
	waitNative(t, func() bool { return !a.model.Loading && a.noteIcons["picture"] != nil })
	mygo.RunOnMain(func() { a.selectNote("note") })
	waitNative(t, func() bool { return a.model.Document.Body != "" })
	for _, mode := range []mygo.ThemeSource{mygo.ThemeLight, mygo.ThemeDark} {
		mygo.RunOnMain(func() { mygo.Theme.SetSource(mode); a.shell.Window().Invalidate() })
		for step, offset := range []float32{0, 120, 0} {
			mygo.RunOnMain(func() { a.articleScroll.Y = offset; a.shell.Window().Invalidate() })
			time.Sleep(250 * time.Millisecond)
			data, err := a.shell.Window().CapturePage()
			if err != nil {
				t.Fatal(err)
			}
			decoded, err := png.Decode(bytes.NewReader(data))
			if err != nil || decoded.Bounds().Dx() < 980 {
				t.Fatalf("invalid capture: %v", err)
			}
			if dir := os.Getenv("LOCUS_SCREENSHOTS"); dir != "" {
				if err := os.MkdirAll(dir, 0700); err != nil {
					t.Fatal(err)
				}
				if err := os.WriteFile(filepath.Join(dir, fmt.Sprintf("reading-%s-%d.png", mode, step)), data, 0600); err != nil {
					t.Fatal(err)
				}
			}
			waitNative(t, func() bool { return a.articleScroll.Y == offset })
		}
	}
	mygo.RunOnMain(func() { a.selectNote("picture") })
	waitNative(t, func() bool { return a.model.Document.Body == "Image note" })
	var previousIconPresent bool
	mygo.RunOnMain(func() { previousIconPresent = a.noteIcons["picture"] != nil })
	if !previousIconPresent {
		t.Fatal("missing selected PNG icon")
	}
	if err := os.Remove(filepath.Join(root, "picture/assets/icon.png")); err != nil {
		t.Fatal(err)
	}
	// The recursive watcher must invalidate icons even when note text is unchanged.
	waitNative(t, func() bool { return a.noteIcons["picture"] == nil && !a.model.Loading })
	mygo.RunOnMain(func() {
		if a.model.Error != nil {
			t.Error(a.model.Error)
		}
		if !a.shell.Window().IsVisible() {
			t.Error("window not visible")
		}
		a.shell.SetLeftExpanded(false)
		a.shell.SetLeftExpanded(true)
	})
}
func waitNative(t *testing.T, ready func() bool) {
	t.Helper()
	deadline := time.Now().Add(5 * time.Second)
	for time.Now().Before(deadline) {
		done := false
		mygo.RunOnMain(func() { done = ready() })
		if done {
			return
		}
		time.Sleep(20 * time.Millisecond)
	}
	t.Fatal("native read timed out")
}
