package library

import (
	"strings"
	"testing"
)

func TestArchiveRejectsComplexYAML(t *testing.T) {
	for _, metadata := range []string{"{title: Example, archived: false}\n", "archived: &state false\ncustom: *state\n", "'archived': false\n", "archived: [false]\n", "archived: 'true'\n"} {
		input := []byte("---\n" + metadata + "---\nBody")
		if _, err := archiveSource(input, true); err == nil {
			t.Errorf("accepted complex archive source %q", input)
		}
	}
}
func TestArchiveInsertsWithoutReformatting(t *testing.T) {
	for _, input := range []string{"Body\n", "---\ntitle: 'Original' # keep\n---\nBody\n", "---\n  title: Original\n---\nBody\n"} {
		updated, err := archiveSource([]byte(input), true)
		if err != nil {
			t.Fatal(err)
		}
		if !strings.Contains(string(updated), "archived: true") || !strings.HasSuffix(string(updated), "Body\n") {
			t.Fatalf("invalid update %q", updated)
		}
	}
}
func TestGitStatusAttribution(t *testing.T) {
	status := []byte(" M note/index.md\x00?? note/canvas/nested/view.js\x00R  renamed/assets/a.png\x00old/assets/a.png\x00 M unrelated.txt\x00")
	got := changedNotes(status)
	for _, note := range []string{"note", "renamed", "old"} {
		if !got[note] {
			t.Errorf("missing owner %s", note)
		}
	}
	if len(got) != 3 {
		t.Fatalf("unexpected attribution: %v", got)
	}
}
