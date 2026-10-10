// Package skills embeds the product's authoring skill in every Go build.
package skills

import (
	"embed"
	"errors"
	"io/fs"
	"os"
	"path/filepath"
)

//go:embed bundled
var bundled embed.FS

// Install writes a new skill directory only. Existing files, directories and
// symlinks are never replaced. The user explicitly chooses library or home root.
func Install(root string) error {
	canonical, err := filepath.Abs(root)
	if err != nil {
		return err
	}
	for _, part := range []string{".agents", "skills"} {
		canonical = filepath.Join(canonical, part)
		info, err := os.Lstat(canonical)
		if errors.Is(err, os.ErrNotExist) {
			if err = os.Mkdir(canonical, 0700); err != nil {
				return err
			}
			continue
		}
		if err != nil {
			return err
		}
		if !info.IsDir() || info.Mode()&os.ModeSymlink != 0 {
			return errors.New("skill parent is not a real directory")
		}
	}
	target := filepath.Join(canonical, "locus")
	if _, err := os.Lstat(target); err == nil {
		return errors.New("Locus skill already exists; move it before installing a new copy")
	} else if !errors.Is(err, os.ErrNotExist) {
		return err
	}
	staging, err := os.MkdirTemp(canonical, ".locus-*")
	if err != nil {
		return err
	}
	defer os.RemoveAll(staging)
	err = fs.WalkDir(bundled, "bundled/locus", func(p string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		rel, err := filepath.Rel("bundled/locus", p)
		if err != nil {
			return err
		}
		dest := filepath.Join(staging, rel)
		if d.IsDir() {
			return os.MkdirAll(dest, 0700)
		}
		data, err := bundled.ReadFile(p)
		if err != nil {
			return err
		}
		return os.WriteFile(dest, data, 0600)
	})
	if err != nil {
		return err
	}
	// Reserve the destination without replacing a concurrent installation.
	if err := os.Mkdir(target, 0700); err != nil {
		return err
	}
	entries, err := os.ReadDir(staging)
	if err != nil {
		return err
	}
	for _, entry := range entries {
		if err := os.Rename(filepath.Join(staging, entry.Name()), filepath.Join(target, entry.Name())); err != nil {
			return errors.New("skill installation incomplete; inspect " + target + ": " + err.Error())
		}
	}
	return nil
}
