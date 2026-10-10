package skills

import (
	"os"
	"path/filepath"
	"testing"
)

func TestInstallPreservesExisting(t *testing.T) {
	root := t.TempDir()
	if err := Install(root); err != nil {
		t.Fatal(err)
	}
	target := filepath.Join(root, ".agents/skills/locus/SKILL.md")
	b, err := os.ReadFile(target)
	if err != nil || len(b) == 0 {
		t.Fatal("skill missing", err)
	}
	if err := Install(root); err == nil {
		t.Fatal("existing skill replaced")
	}
	got, _ := os.ReadFile(target)
	if string(got) != string(b) {
		t.Fatal("existing skill changed")
	}
}
func TestInstallRejectsRedirectedParent(t *testing.T) {
	root, outside := t.TempDir(), t.TempDir()
	if err := os.Symlink(outside, filepath.Join(root, ".agents")); err != nil {
		t.Skip(err)
	}
	if err := Install(root); err == nil {
		t.Fatal("symlink parent accepted")
	}
	entries, err := os.ReadDir(outside)
	if err != nil || len(entries) != 0 {
		t.Fatal("modified redirected directory")
	}
}
