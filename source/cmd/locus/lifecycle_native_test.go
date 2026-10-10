package main

import (
	"os"
	"path/filepath"
	"runtime"
	"testing"

	"github.com/egoist/mygo"
	"github.com/oneo-me/locus/internal/preferences"
)

func TestNativeWindowCloseAndReopen(t *testing.T) {
	if os.Getenv("LOCUS_NATIVE_SMOKE") != "1" || runtime.GOOS != "darwin" {
		t.Skip("requires a macOS desktop session")
	}
	root := t.TempDir()
	root, err := filepath.EvalSymlinks(root)
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(root, "index.md"), []byte("Retained document"), 0600); err != nil {
		t.Fatal(err)
	}
	a := &application{cfg: preferences.Default(), statePath: filepath.Join(t.TempDir(), "state.yaml"), stateWritable: true}
	mygo.RunOnMain(func() { a.start(root) })
	defer mygo.RunOnMain(func() { a.shell.Window().Destroy() })
	waitNative(t, func() bool { return !a.model.Loading && a.treeRoot != "" })
	mygo.RunOnMain(func() {
		a.shell.SetLeftExpanded(false)
		a.shell.Window().Close()
		if a.shell.Window().IsVisible() || a.closing || a.watchCancel == nil {
			t.Error("closing should hide the window and retain the workspace")
		}
	})
	state, err := preferences.LoadState(a.statePath)
	if err != nil {
		t.Fatal(err)
	}
	if !state.Collapsed || state.Library != root {
		t.Fatal("closing did not save presentation state")
	}
	mygo.RunOnMain(func() {
		a.showWindow()
		if !a.shell.Window().IsVisible() || a.shell.LeftExpanded() {
			t.Error("reopening did not restore the retained window")
		}
		a.shell.Window().Close()
		// Explicit quit must also close a previously hidden window.
		a.quitting = true
		a.shell.Window().Close()
		if !a.closing {
			t.Error("quitting did not release the workspace")
		}
	})
}
