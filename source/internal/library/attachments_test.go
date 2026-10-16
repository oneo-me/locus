package library

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestLibraryAttachments(t *testing.T) {
	root := t.TempDir()
	put := func(p, body string) {
		t.Helper()
		full := filepath.Join(root, filepath.FromSlash(p))
		if err := os.MkdirAll(filepath.Dir(full), 0755); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(full, []byte(body), 0644); err != nil {
			t.Fatal(err)
		}
	}
	put("note/index.md", "Body\n")
	put("note/assets/photo.png", "png")
	put("note/assets/report.pdf", "pdf")
	put("note/assets/icon.png", "icon")
	put("note/assets/.hidden", "hidden")
	put("note/assets/nested/child.txt", "nested")
	put("other/index.md", "Body\n")
	put("plain/index.md", "Body\n")

	lib, err := Open(root)
	if err != nil {
		t.Fatal(err)
	}
	defer lib.Close()
	notes, err := lib.Scan()
	if err != nil {
		t.Fatal(err)
	}
	got := lib.Attachments(notes)
	names := map[string][]string{}
	for _, attachment := range got {
		names[attachment.Note] = append(names[attachment.Note], attachment.Name)
	}
	if want := strings.Join([]string{"photo.png", "report.pdf"}, ","); strings.Join(names["note"], ",") != want {
		t.Fatalf("note attachments = %v, want %s", names["note"], want)
	}
	if len(names["other"]) != 0 || len(names["plain"]) != 0 {
		t.Fatalf("unexpected attachments: %v", names)
	}
}

func TestLibraryAttachmentsRejectsSymlinkDirectory(t *testing.T) {
	root := t.TempDir()
	if err := os.MkdirAll(filepath.Join(root, "note"), 0755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(root, "note", "index.md"), []byte("Body\n"), 0644); err != nil {
		t.Fatal(err)
	}
	outside := t.TempDir()
	if err := os.WriteFile(filepath.Join(outside, "secret.txt"), []byte("secret"), 0644); err != nil {
		t.Fatal(err)
	}
	if err := os.Symlink(outside, filepath.Join(root, "note", "assets")); err != nil {
		t.Skipf("symlinks unavailable: %v", err)
	}
	lib, err := Open(root)
	if err != nil {
		t.Fatal(err)
	}
	defer lib.Close()
	notes, err := lib.Scan()
	if err != nil {
		t.Fatal(err)
	}
	if got := lib.Attachments(notes); len(got) != 0 {
		t.Fatalf("symlink assets listed: %+v", got)
	}
}
