package workspace

import (
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/oneo-me/locus/internal/library"
)

func fixture(t *testing.T, body string) string {
	t.Helper()
	root := t.TempDir()
	if err := os.Mkdir(filepath.Join(root, "note"), 0700); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(root, "note/index.md"), []byte(body), 0600); err != nil {
		t.Fatal(err)
	}
	canonical, err := filepath.EvalSymlinks(root)
	if err != nil {
		t.Fatal(err)
	}
	return canonical
}
func receive(t *testing.T, queue <-chan func()) func() {
	t.Helper()
	select {
	case fn := <-queue:
		return fn
	case <-time.After(5 * time.Second):
		t.Fatal("background task did not finish")
		return nil
	}
}
func TestLateLibraryAndReadResults(t *testing.T) {
	first, second := fixture(t, "First"), fixture(t, "Second")
	queue := make(chan func(), 10)
	m := Model{Dispatch: func(fn func()) { queue <- fn }}
	m.Open(first)
	oldScan := receive(t, queue)
	m.Open(second)
	receive(t, queue)()
	oldScan()
	if m.Root != second || library.Find(m.Notes, "note") == nil {
		t.Fatalf("old scan replaced current library: %+v", m)
	}
	m.Select("note")
	oldRead := receive(t, queue)
	m.Open(first)
	receive(t, queue)()
	m.Select("note")
	receive(t, queue)()
	oldRead()
	if m.Document.Body != "First" {
		t.Fatalf("old read replaced body: %q", m.Document.Body)
	}
}
func TestClearInvalidatesPendingRead(t *testing.T) {
	queue := make(chan func(), 10)
	m := Model{Dispatch: func(fn func()) { queue <- fn }}
	m.Open(fixture(t, "Body"))
	receive(t, queue)()
	m.Select("note")
	pending := receive(t, queue)
	m.Select("")
	pending()
	if m.Document.Body != "" || m.Selected != "" {
		t.Fatal("pending read restored cleared note")
	}
}
func TestRefreshRemovalAndErrors(t *testing.T) {
	root := fixture(t, "Body")
	queue := make(chan func(), 10)
	m := Model{Dispatch: func(fn func()) { queue <- fn }}
	m.Open(root)
	receive(t, queue)()
	m.Select("note")
	receive(t, queue)()
	if err := os.Remove(filepath.Join(root, "note/index.md")); err != nil {
		t.Fatal(err)
	}
	m.Refresh()
	receive(t, queue)()
	if m.Selected != "" || m.Document.Body != "" {
		t.Fatal("removed note retained")
	}
	if err := os.RemoveAll(root); err != nil {
		t.Fatal(err)
	}
	m.Refresh()
	receive(t, queue)()
	if m.Error == nil || m.Loading {
		t.Fatal("scan failure hidden")
	}
}
