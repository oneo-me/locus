package library

import "testing"

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
