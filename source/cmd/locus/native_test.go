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
	body := "---\ntitle: 中文 Reading test\nsummary: Native Go and Onevra\n---\n# Markdown\n\n中文与 English `inline code` and **bold text**.\n\n| Name | Value |\n| --- | --- |\n| 中文 | A table |\n\n```go\nfmt.Println(\"Locus\")\n```\n"
	if err := os.WriteFile(filepath.Join(root, "note/index.md"), []byte(body), 0600); err != nil {
		t.Fatal(err)
	}
	a := &application{cursor: -1}
	a.model.Dispatch = mygo.RunOnMain
	a.model.Changed = func() { a.shell.Window().Invalidate() }
	mygo.RunOnMain(func() { a.start(root) })
	defer mygo.RunOnMain(func() { a.shell.Window().Destroy() })
	waitNative(t, func() bool { return !a.model.Loading })
	mygo.RunOnMain(func() { a.selectNote("note") })
	waitNative(t, func() bool { return a.model.Document.Body != "" })
	for _, mode := range []mygo.ThemeSource{mygo.ThemeLight, mygo.ThemeDark} {
		mygo.RunOnMain(func() { mygo.Theme.SetSource(mode); a.shell.Window().Invalidate() })
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
			if err := os.WriteFile(filepath.Join(dir, fmt.Sprintf("reading-%s.png", mode)), data, 0600); err != nil {
				t.Fatal(err)
			}
		}
	}
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
