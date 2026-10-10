package library

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func put(t *testing.T, root, p, body string) {
	t.Helper()
	target := filepath.Join(root, filepath.FromSlash(p))
	if err := os.MkdirAll(filepath.Dir(target), 0700); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(target, []byte(body), 0600); err != nil {
		t.Fatal(err)
	}
}
func openTest(t *testing.T, root string) *Library {
	t.Helper()
	lib, err := Open(root)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { lib.Close() })
	return lib
}
func TestScanStructure(t *testing.T) {
	root := t.TempDir()
	for p, body := range map[string]string{
		"guide/index.md":                "---\ntitle: 中文 Guide\nsummary: Description\ncustom: [a, b]\n---\nHello",
		"guide/child/index.md":          "Child",
		"group/note/index.md":           "Note",
		"upper/index.md":                "---\narchived: TRUE\n---\nHidden",
		"archive/index.md":              "---\narchived: true\n---\nHidden",
		"archive/child/index.md":        "Hidden child",
		"quoted/index.md":               "---\narchived: 'true'\n---\nVisible",
		"guide/assets/nested/index.md":  "Not a note",
		"guide/canvas/index.html":       "<html></html>",
		"guide/canvas/nested/index.md":  "Not a note",
		".hidden/index.md":              "Hidden",
		"node_modules/package/index.md": "Hidden",
		"empty/readme.md":               "Not a note",
	} {
		put(t, root, p, body)
	}
	notes, err := openTest(t, root).Scan()
	if err != nil {
		t.Fatal(err)
	}
	if len(notes) != 5 {
		t.Fatalf("roots=%d", len(notes))
	}
	note := Find(notes, "guide")
	if note == nil || note.Title != "中文 Guide" || note.Summary != "Description" || note.Canvas != "guide/canvas/index.html" || len(note.Children) != 1 {
		t.Fatalf("guide=%+v", note)
	}
	if group := Find(notes, "group"); group == nil || group.HasBody || len(group.Children) != 1 {
		t.Fatalf("group=%+v", group)
	}
	for _, p := range []string{"archive", "archive/child", "upper", "quoted"} {
		if Find(notes, p) == nil {
			t.Errorf("custom archived metadata hid note %s", p)
		}
	}
	for _, p := range []string{".hidden", "empty", "node_modules", "guide/assets/nested"} {
		if Find(notes, p) != nil {
			t.Errorf("unexpected note %s", p)
		}
	}
}
func TestReadFrontmatter(t *testing.T) {
	for _, tc := range []struct {
		name, source, body string
		metadata           bool
	}{
		{"plain", "# Title\nBody", "# Title\nBody", false},
		{"yaml", "---\ntitle: Note\ncustom:\n  tags: [一, two]\n---\nBody\n", "Body\n", true},
		{"legacy-archived", "---\narchived: true\n---\nBody\n", "Body\n", true},
		{"bom-crlf", "\ufeff---\r\ntitle: Note\r\n---\r\nBody\r\n", "Body\r\n", true},
		{"unterminated", "---\ntitle: Text", "---\ntitle: Text", false},
		{"empty", "", "", false},
	} {
		t.Run(tc.name, func(t *testing.T) {
			root := t.TempDir()
			put(t, root, "note/index.md", tc.source)
			doc, err := openTest(t, root).Read("note")
			if err != nil {
				t.Fatal(err)
			}
			if doc.Body != tc.body || (doc.Metadata != nil) != tc.metadata {
				t.Fatalf("document=%+v", doc)
			}
			got, err := os.ReadFile(filepath.Join(root, "note/index.md"))
			if err != nil || string(got) != tc.source {
				t.Fatal("read modified original")
			}
		})
	}
}
func TestAddressBoundary(t *testing.T) {
	for _, p := range []string{"parent/中文 note", "100%/#tag", "a/%2f"} {
		got, err := ParseAddress(Address(p))
		if err != nil || got != p {
			t.Fatalf("roundtrip %q = %q: %v", p, got, err)
		}
	}
	for _, address := range []string{"file:///tmp", "locus://", "locus:///abs", "locus://../x", "locus://%2e%2e/x", "locus://a%2Fb", "locus://a%5cb", "locus://a//b", "locus://a/assets", "locus://.hidden", "locus://a/%zz", "locus://a/%00", "locus://a/%ff", "locus://a#fragment", "locus://a?query"} {
		if _, err := ParseAddress(address); err == nil {
			t.Errorf("accepted %q", address)
		}
	}
	lib := openTest(t, t.TempDir())
	for _, p := range []string{"../outside", "/absolute", "a/../b", "a\\b", "a/canvas", ""} {
		if _, err := lib.Read(p); err == nil {
			t.Errorf("read accepted %q", p)
		}
	}
}
func TestSymlinksAndSpecialFiles(t *testing.T) {
	root, outside := t.TempDir(), t.TempDir()
	put(t, outside, "note/index.md", "Private")
	if err := os.Symlink(filepath.Join(outside, "note"), filepath.Join(root, "linked")); err != nil {
		t.Skip(err)
	}
	lib := openTest(t, root)
	notes, err := lib.Scan()
	if err != nil || len(notes) != 0 {
		t.Fatalf("scan followed symlink: %v %v", notes, err)
	}
	if _, err := lib.Read("linked"); err == nil {
		t.Fatal("read followed symlink")
	}
	if err := os.Mkdir(filepath.Join(root, "note"), 0700); err != nil {
		t.Fatal(err)
	}
	if err := os.Symlink(filepath.Join(outside, "note/index.md"), filepath.Join(root, "note/index.md")); err != nil {
		t.Fatal(err)
	}
	if _, err := lib.Read("note"); err == nil {
		t.Fatal("read followed file symlink")
	}
	if _, err := lib.Scan(); err == nil {
		t.Fatal("scan accepted unsafe body")
	}
}
func TestMalformedAndOversizedMetadata(t *testing.T) {
	for _, body := range []string{"---\ntitle: [broken\n---\nBody", strings.Repeat("x", maxMetadata+1), "---\n" + strings.Repeat("x", maxMetadata)} {
		root := t.TempDir()
		put(t, root, "note/index.md", body)
		lib := openTest(t, root)
		if _, err := lib.Scan(); err == nil {
			t.Fatal("scan accepted invalid metadata")
		}
		if _, err := lib.Read("note"); err == nil {
			t.Fatal("read accepted invalid metadata")
		}
	}
}
func TestScanDoesNotLoadBodies(t *testing.T) {
	root := t.TempDir()
	put(t, root, "note/index.md", "---\ntitle: Large\n---\n"+strings.Repeat("x", maxBody+1))
	lib := openTest(t, root)
	if _, err := lib.Scan(); err != nil {
		t.Fatal(err)
	}
	if _, err := lib.Read("note"); err == nil {
		t.Fatal("oversized body accepted")
	}
}
