package main

import (
	"os"
	"path/filepath"
	"testing"
)

func TestMigrateDataDirectoryMovesLegacyDirectory(t *testing.T) {
	home := t.TempDir()
	legacy := filepath.Join(home, ".oneo", "locus")
	data := filepath.Join(home, ".locus")
	if err := os.MkdirAll(legacy, 0700); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(legacy, "settings.yaml"), []byte("library: /notes\n"), 0600); err != nil {
		t.Fatal(err)
	}
	migrateDataDirectory(legacy, data)
	if body, err := os.ReadFile(filepath.Join(data, "settings.yaml")); err != nil || string(body) != "library: /notes\n" {
		t.Fatalf("settings = %q, %v", body, err)
	}
	if _, err := os.Lstat(legacy); !os.IsNotExist(err) {
		t.Fatalf("legacy directory remains: %v", err)
	}
}

func TestMigrateDataDirectoryKeepsExistingDirectory(t *testing.T) {
	home := t.TempDir()
	legacy := filepath.Join(home, ".oneo", "locus")
	data := filepath.Join(home, ".locus")
	for _, dir := range []string{legacy, data} {
		if err := os.MkdirAll(dir, 0700); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(dir, "settings.yaml"), []byte(dir), 0600); err != nil {
			t.Fatal(err)
		}
	}
	migrateDataDirectory(legacy, data)
	for _, dir := range []string{legacy, data} {
		if body, err := os.ReadFile(filepath.Join(dir, "settings.yaml")); err != nil || string(body) != dir {
			t.Fatalf("%s settings = %q, %v", dir, body, err)
		}
	}
}

func TestMigrateDataDirectoryWithoutLegacyDirectory(t *testing.T) {
	home := t.TempDir()
	data := filepath.Join(home, ".locus")
	migrateDataDirectory(filepath.Join(home, ".oneo", "locus"), data)
	if _, err := os.Lstat(data); !os.IsNotExist(err) {
		t.Fatalf("data directory created: %v", err)
	}
}
