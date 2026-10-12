package main

import (
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/egoist/mygo"
)

func TestNativeLibraryPicker(t *testing.T) {
	if os.Getenv("LOCUS_NATIVE_SMOKE") != "1" {
		t.Skip("requires a native desktop session")
	}
	a := &application{cursor: -1, hidden: nativeBackground()}
	mygo.RunOnMain(func() { a.start("") })
	defer mygo.RunOnMain(func() { a.shell.Window().Destroy() })
	capture := func(name string) {
		t.Helper()
		time.Sleep(250 * time.Millisecond)
		data, err := a.shell.Window().CapturePage()
		if err != nil {
			t.Fatal(err)
		}
		if dir := os.Getenv("LOCUS_SCREENSHOTS"); dir != "" {
			if err := os.MkdirAll(dir, 0700); err != nil {
				t.Fatal(err)
			}
			if err := os.WriteFile(filepath.Join(dir, name+".png"), data, 0600); err != nil {
				t.Fatal(err)
			}
		}
	}
	capture("library-picker")
	mygo.RunOnMain(func() { a.toggleSettings() })
	capture("library-picker-settings")
	mygo.RunOnMain(func() {
		a.toggleSettings()
		if a.model.Root != "" || a.settingsOpen {
			t.Error("closing settings changed the empty workspace")
		}
		a.shell.SetLeftWidth(280)
		a.shell.SetLeftExpanded(false)
	})
	root, err := filepath.EvalSymlinks(t.TempDir())
	if err != nil {
		t.Fatal(err)
	}
	mygo.RunOnMain(func() { a.openLibrary(root) })
	waitNative(t, func() bool { return !a.model.Loading })
	capture("empty-library")
	mygo.RunOnMain(func() {
		if a.model.Error != nil {
			t.Error(a.model.Error)
		}
		if a.model.Root != root || a.shell.LeftExpanded() || a.shell.LeftWidth() != 280 {
			t.Error("opening a library lost sidebar presentation")
		}
		a.toggleSettings()
	})
	capture("workspace-settings")
}
