package skills

import (
	"bytes"
	"io/fs"
	"os"
	"path/filepath"
	"testing"
)

func TestInstallPreservesExisting(t *testing.T) {
	root := t.TempDir()
	if err := Install(root, "../../resources"); err != nil {
		t.Fatal(err)
	}
	target := filepath.Join(root, ".agents/skills/locus/SKILL.md")
	b, err := os.ReadFile(target)
	if err != nil || len(b) == 0 {
		t.Fatal("skill missing", err)
	}
	if err := Install(root, "../../resources"); err == nil {
		t.Fatal("existing skill replaced")
	}
	got, _ := os.ReadFile(target)
	if string(got) != string(b) {
		t.Fatal("existing skill changed")
	}
}
func TestInstallCopiesResourceTree(t *testing.T) {
	root := t.TempDir()
	if err := Install(root, "../../resources"); err != nil {
		t.Fatal(err)
	}
	source := os.DirFS("../../resources/skills/locus")
	err := fs.WalkDir(source, ".", func(path string, entry fs.DirEntry, err error) error {
		if err != nil || entry.IsDir() {
			return err
		}
		want, err := fs.ReadFile(source, path)
		if err != nil {
			return err
		}
		got, err := os.ReadFile(filepath.Join(root, ".agents/skills/locus", path))
		if err != nil {
			return err
		}
		if !bytes.Equal(got, want) {
			t.Errorf("resource differs: %s", path)
		}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
}

func TestInstallMissingResources(t *testing.T) {
	root := t.TempDir()
	if err := Install(root, t.TempDir()); err == nil {
		t.Fatal("missing resources accepted")
	}
	if _, err := os.Stat(filepath.Join(root, ".agents")); !os.IsNotExist(err) {
		t.Fatalf("missing resources modified destination: %v", err)
	}
}

func TestInstallRejectsRedirectedParent(t *testing.T) {
	root, outside := t.TempDir(), t.TempDir()
	if err := os.Symlink(outside, filepath.Join(root, ".agents")); err != nil {
		t.Skip(err)
	}
	if err := Install(root, "../../resources"); err == nil {
		t.Fatal("symlink parent accepted")
	}
	entries, err := os.ReadDir(outside)
	if err != nil || len(entries) != 0 {
		t.Fatal("modified redirected directory")
	}
}
