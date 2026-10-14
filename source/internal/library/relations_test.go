package library

import (
	"os"
	"path/filepath"
	"reflect"
	"testing"
)

func TestResolveNoteLink(t *testing.T) {
	cases := []struct {
		note, target string
		want         string
		ok           bool
	}{
		{"guide/intro", "locus://reference/setup", "reference/setup", true},
		{"guide/intro", "../../reference/setup", "reference/setup", true},
		{"guide/intro", "../../outside", "outside", true},
		{"guide/intro", "../../../outside", "", false},
		{"guide/intro", "reference/setup/index.md", "guide/intro/reference/setup", true},
		{"guide", "child/index.md", "guide/child", true},
		{"guide/intro", "./", "guide/intro", true},
		{"guide/intro", "#section", "", false},
		{"guide/intro", "https://example.com/note", "", false},
		{"guide/intro", "mailto:someone@example.com", "", false},
		{"guide/intro", "assets/image.png", "", false},
		{"guide/intro", "locus://bad/../escape", "", false},
	}
	for _, tc := range cases {
		got, ok := ResolveNoteLink(tc.note, tc.target)
		if ok != tc.ok || got != tc.want {
			t.Errorf("ResolveNoteLink(%q, %q) = %q, %v; want %q, %v", tc.note, tc.target, got, ok, tc.want, tc.ok)
		}
	}
}

func TestNoteLinksReadsVisibleMarkdownLinks(t *testing.T) {
	body := "See [setup](../../reference/setup) and `locus://ignored/in/code`.\n\n" +
		"[self]: locus://guide/intro\n[self]\n\n" +
		"![image](assets/picture.png)\n\n" +
		"A bare locus://bare/link is not a Markdown link.\n\n" +
		"```\n[fenced](locus://fenced/note)\n```\n"
	want := []string{"reference/setup"}
	if got := NoteLinks("guide/intro", body); !reflect.DeepEqual(got, want) {
		t.Fatalf("NoteLinks = %v, want %v", got, want)
	}
}

func TestRelationsBetweenNotes(t *testing.T) {
	root := t.TempDir()
	put(t, root, "index-note/index.md", "Root\n")
	put(t, root, "guide/index.md", "---\ntitle: Guide\n---\nSee [root](locus://index-note) and [setup](../reference/setup).\n")
	put(t, root, "guide/child/index.md", "No links here.\n")
	put(t, root, "reference/setup/index.md", "[back](../../guide/index.md) and [missing](locus://ghost).\n")
	put(t, root, "orphan/index.md", "![only](assets/picture.png)\n")
	lib := openTest(t, root)
	notes, err := lib.Scan()
	if err != nil {
		t.Fatal(err)
	}
	got := lib.Relations(notes)
	want := []Relation{
		{Source: "guide", Target: "index-note"},
		{Source: "guide", Target: "reference/setup"},
		{Source: "reference/setup", Target: "guide"},
	}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("Relations = %+v, want %+v", got, want)
	}
}

func TestRelationsSkipUnreadableBodies(t *testing.T) {
	root := t.TempDir()
	put(t, root, "one/index.md", "One\n")
	put(t, root, "two/index.md", "[one](locus://one)\n")
	lib := openTest(t, root)
	notes, err := lib.Scan()
	if err != nil {
		t.Fatal(err)
	}
	// Remove one body after scanning so reading it fails. The edge from two
	// still resolves because the note path is known.
	if err := os.Remove(filepath.Join(root, "one", "index.md")); err != nil {
		t.Fatal(err)
	}
	got := lib.Relations(notes)
	want := []Relation{{Source: "two", Target: "one"}}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("Relations = %+v, want %+v", got, want)
	}
}
