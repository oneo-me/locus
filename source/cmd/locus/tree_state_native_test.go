package main

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/egoist/mygo"
	"github.com/oneo-me/locus/internal/preferences"
)

func TestNativeTreeExpansionReopen(t *testing.T) {
	if os.Getenv("LOCUS_NATIVE_SMOKE") != "1" {
		t.Skip("requires a native desktop session")
	}
	root := t.TempDir()
	for _, path := range []string{"parent/child/leaf", "other/leaf"} {
		if err := os.MkdirAll(filepath.Join(root, path), 0700); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(root, path, "index.md"), []byte("Note"), 0600); err != nil {
			t.Fatal(err)
		}
	}
	otherRoot := t.TempDir()
	statePath := filepath.Join(t.TempDir(), "state.yaml")
	a := &application{cfg: preferences.Default(), statePath: statePath, stateWritable: true, hidden: nativeBackground()}
	mygo.RunOnMain(func() { a.start(root) })
	waitNative(t, func() bool { return a.treeRoot != "" && !a.model.Loading })
	mygo.RunOnMain(func() {
		a.tree.Open.Add("parent/child")
		a.tree.Open.Add("other")
		a.focusNote("parent/child/leaf")
		a.saveState()
		a.shell.Window().Destroy()
	})
	state, err := preferences.LoadState(statePath)
	if err != nil {
		t.Fatal(err)
	}
	b := &application{cfg: preferences.Default(), state: state, restore: true, hidden: nativeBackground()}
	mygo.RunOnMain(func() { b.start(root) })
	defer mygo.RunOnMain(func() { b.shell.Window().Destroy() })
	waitNative(t, func() bool { return b.treeRoot != "" && !b.model.Loading && !b.restore })
	mygo.RunOnMain(func() {
		if b.tree.Open.Has("parent") || !b.tree.Open.Has("parent/child") || !b.tree.Open.Has("other") {
			t.Error("native window did not restore saved expansion")
		}
		if b.model.Selected != "parent/child/leaf" || b.cursor != -1 {
			t.Error("restoring hidden selection changed the tree")
		}
		b.shell.Window().Invalidate()
		b.openLibrary(otherRoot)
	})
	waitNative(t, func() bool { return b.treeRoot != "" && !b.model.Loading })
	mygo.RunOnMain(func() {
		if b.tree.Open.Len() != 0 {
			t.Error("library switch inherited expansion")
		}
		b.openLibrary(root)
	})
	waitNative(t, func() bool { return b.treeRoot == state.Library && !b.model.Loading })
	mygo.RunOnMain(func() {
		if b.tree.Open.Has("parent") || !b.tree.Open.Has("parent/child") || !b.tree.Open.Has("other") {
			t.Error("switching back lost expansion")
		}
	})
}
