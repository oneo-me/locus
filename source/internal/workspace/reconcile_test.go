package workspace

import (
	"os"
	"path/filepath"
	"testing"
)

func TestRefreshRequestedDuringScan(t *testing.T) {
	root := fixture(t, "Body")
	queue := make(chan func(), 10)
	m := Model{Dispatch: func(fn func()) { queue <- fn }}
	m.Open(root)
	first := receive(t, queue)
	m.Refresh()
	if err := os.Mkdir(filepath.Join(root, "added"), 0700); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(root, "added/index.md"), []byte("Added"), 0600); err != nil {
		t.Fatal(err)
	}
	first()
	receive(t, queue)()
	if len(m.Notes) != 2 || m.Loading {
		t.Fatalf("queued refresh lost: %+v", m)
	}
}
func TestReadDoesNotHideScanFailure(t *testing.T) {
	root := fixture(t, "Body")
	queue := make(chan func(), 10)
	m := Model{Dispatch: func(fn func()) { queue <- fn }}
	m.Open(root)
	receive(t, queue)()
	if err := os.Mkdir(filepath.Join(root, "bad"), 0700); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(root, "bad/index.md"), []byte("---\ntitle: [broken\n---\n"), 0600); err != nil {
		t.Fatal(err)
	}
	m.Refresh()
	receive(t, queue)()
	if m.Error == nil {
		t.Fatal("scan error missing")
	}
	m.Select("note")
	receive(t, queue)()
	if m.Error == nil || m.Document.Body != "Body" {
		t.Fatalf("read hid projection failure: %+v", m)
	}
}
