package library

import (
	"os"
	"path/filepath"
	"testing"
)

func TestArchiveFidelityAndConflict(t *testing.T) {
	root := t.TempDir()
	source := "\ufeff---\r\ntitle: 中文\r\ncustom: [one, two] # keep\r\narchived: false # state\r\n---\r\nBody\r\n"
	put(t, root, "note/index.md", source)
	lib := openTest(t, root)
	if err := lib.SetArchived("note", Version([]byte(source)), true); err != nil {
		t.Fatal(err)
	}
	raw, err := os.ReadFile(filepath.Join(root, "note/index.md"))
	if err != nil {
		t.Fatal(err)
	}
	expected := "\ufeff---\r\ntitle: 中文\r\ncustom: [one, two] # keep\r\narchived: true # state\r\n---\r\nBody\r\n"
	if string(raw) != expected {
		t.Fatalf("unexpected rewrite: %q", raw)
	}
	notes, err := lib.Scan()
	if err != nil || len(notes) != 0 {
		t.Fatal("archive still visible", err)
	}
	archived, err := lib.Archived()
	if err != nil || len(archived) != 1 {
		t.Fatal("archive missing", err)
	}
	if err := lib.SetArchived("note", Version([]byte(source)), false); err == nil {
		t.Fatal("stale version accepted")
	}
	if err := lib.SetArchived("note", Version(raw), false); err != nil {
		t.Fatal(err)
	}
	raw, _ = os.ReadFile(filepath.Join(root, "note/index.md"))
	if string(raw) != source {
		t.Fatal("restore changed unrelated bytes")
	}
}
