package library

import (
	"bytes"
	"crypto/rand"
	"errors"
	"image"
	"image/png"
	"io"
	"os"
	"path"
	"strings"

	"go.yaml.in/yaml/v3"
	"golang.org/x/image/draw"
)

// PrepareIcon bounds source reads and decoding before producing a small PNG.
func PrepareIcon(filename string) ([]byte, error) {
	info, err := os.Lstat(filename)
	if err != nil {
		return nil, err
	}
	if !info.Mode().IsRegular() {
		return nil, errors.New("choose a regular PNG file")
	}
	if info.Size() > 16<<20 {
		return nil, errors.New("source PNG exceeds 16 MiB")
	}
	f, err := os.Open(filename)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	data, err := io.ReadAll(io.LimitReader(f, (16<<20)+1))
	if err != nil {
		return nil, err
	}
	if len(data) > 16<<20 {
		return nil, errors.New("source PNG exceeds 16 MiB")
	}
	cfg, err := png.DecodeConfig(bytes.NewReader(data))
	if err != nil {
		return nil, errors.New("choose a valid PNG image")
	}
	if cfg.Width <= 0 || cfg.Height <= 0 || int64(cfg.Width)*int64(cfg.Height) > 16_000_000 {
		return nil, errors.New("source PNG exceeds 16 million pixels")
	}
	src, err := png.Decode(bytes.NewReader(data))
	if err != nil {
		return nil, err
	}
	w, h := cfg.Width, cfg.Height
	if w > 64 || h > 64 {
		if w >= h {
			h, w = max(1, h*64/w), 64
		} else {
			w, h = max(1, w*64/h), 64
		}
		dst := image.NewNRGBA(image.Rect(0, 0, w, h))
		draw.CatmullRom.Scale(dst, dst.Bounds(), src, src.Bounds(), draw.Src, nil)
		src = dst
	}
	var out bytes.Buffer
	if err := png.Encode(&out, src); err != nil {
		return nil, err
	}
	if out.Len() > 256<<10 {
		return nil, errors.New("processed icon exceeds 256 KiB")
	}
	return out.Bytes(), nil
}

// iconFrontmatter changes only a simple icon field, retaining other source bytes.
// Complex YAML forms are rejected rather than rewriting unrelated metadata.
func iconFrontmatter(data []byte) ([]byte, error) {
	bom := ""
	text := string(data)
	if strings.HasPrefix(text, "\ufeff") {
		bom, text = "\ufeff", strings.TrimPrefix(text, "\ufeff")
	}
	nl := "\n"
	if strings.Contains(text, "\r\n") {
		nl = "\r\n"
	}
	raw, _ := Parts([]byte(text))
	field := "icon: assets/icon.png"
	if raw == nil {
		if strings.TrimSpace(strings.SplitN(text, "\n", 2)[0]) == "---" {
			return nil, errors.New("unclosed frontmatter")
		}
		result := []byte(bom + "---" + nl + field + nl + "---" + nl + text)
		if len(result) > maxBody {
			return nil, errors.New("updated note exceeds size limit")
		}
		return result, nil
	}
	var doc yaml.Node
	if err := yaml.Unmarshal(raw, &doc); err != nil {
		return nil, err
	}
	lines := strings.Split(string(raw), "\n")
	index := -1
	if len(doc.Content) > 0 {
		mapping := doc.Content[0]
		if mapping.Kind != yaml.MappingNode || mapping.Style&yaml.FlowStyle != 0 {
			return nil, errors.New("cannot safely update icon in complex frontmatter")
		}
		for i := 0; i+1 < len(mapping.Content); i += 2 {
			key, value := mapping.Content[i], mapping.Content[i+1]
			if key.Value != "icon" {
				continue
			}
			if index >= 0 || key.Column != 1 || value.Kind != yaml.ScalarNode || value.Line != key.Line || value.Style&(yaml.LiteralStyle|yaml.FoldedStyle) != 0 || value.Anchor != "" || strings.ContainsAny(value.Value, "\r\n") {
				return nil, errors.New("cannot safely replace complex icon field")
			}
			index = key.Line - 1
			// Multiline quoted scalars can fold to a single-line value.
			if index+1 < len(lines) && strings.TrimSpace(lines[index+1]) != "" && (strings.HasPrefix(lines[index+1], " ") || strings.HasPrefix(lines[index+1], "\t")) {
				return nil, errors.New("cannot safely replace multiline icon field")
			}
		}
	}
	updated := string(raw)
	if index >= 0 {
		suffix := ""
		if strings.HasSuffix(lines[index], "\r") {
			suffix = "\r"
		}
		lines[index] = field + suffix
		updated = strings.Join(lines, "\n")
	} else {
		updated += field + nl
	}
	var verified yaml.Node
	if err := yaml.Unmarshal([]byte(updated), &verified); err != nil {
		return nil, err
	}
	if scalar(&verified, "icon") != "assets/icon.png" {
		return nil, errors.New("cannot safely update icon field")
	}
	start := strings.IndexByte(text, '\n') + 1
	result := []byte(bom + text[:start] + updated + text[start+len(raw):])
	if len(result) > maxBody || len(updated) > maxMetadata-16 {
		return nil, errors.New("updated note exceeds size limit")
	}
	return result, nil
}

// SetIcon replaces the reserved asset and its declaration using rooted writes.
// Each replacement is atomic; the two files are not a filesystem transaction.
func (l *Library) SetIcon(note string, pngData []byte) error {
	if !validNote(note) {
		return errors.New("invalid note path")
	}
	cfg, err := png.DecodeConfig(bytes.NewReader(pngData))
	if err != nil || len(pngData) > 256<<10 || cfg.Width > 64 || cfg.Height > 64 {
		return errors.New("invalid prepared icon")
	}
	bodyPath := path.Join(note, "index.md")
	original, err := l.Resource(bodyPath)
	if err != nil {
		return err
	}
	updated, err := iconFrontmatter(original)
	if err != nil {
		return err
	}
	assets := path.Join(note, "assets")
	if err := l.root.Mkdir(assets, 0755); err != nil && !errors.Is(err, os.ErrExist) {
		return err
	}
	target := path.Join(assets, "icon.png")
	var previous []byte
	if err := l.regular(target); err == nil {
		previous, err = l.Resource(target)
		if err != nil {
			return err
		}
	} else if !errors.Is(err, os.ErrNotExist) {
		return err
	}
	// Validate the parent even when the icon does not exist.
	info, err := l.root.Lstat(assets)
	if err != nil {
		return err
	}
	if !info.IsDir() || info.Mode()&os.ModeSymlink != 0 {
		return errors.New("unsafe assets directory")
	}
	current, err := l.Resource(bodyPath)
	if err != nil {
		return err
	}
	if !bytes.Equal(current, original) {
		return errors.New("note changed during icon upload; retry")
	}
	if err := l.replaceFile(target, pngData); err != nil {
		return err
	}
	if err := l.replaceFile(bodyPath, updated); err != nil {
		var rollback error
		if previous != nil {
			rollback = l.replaceFile(target, previous)
		} else {
			rollback = l.root.Remove(target)
		}
		return errors.Join(err, rollback)
	}
	return nil
}

func (l *Library) replaceFile(target string, data []byte) error {
	mode := os.FileMode(0644)
	if info, err := l.root.Lstat(target); err == nil {
		if err := l.regular(target); err != nil {
			return err
		}
		mode = info.Mode().Perm()
	} else if !errors.Is(err, os.ErrNotExist) {
		return err
	}
	temp := path.Join(path.Dir(target), ".icon-upload-"+rand.Text())
	f, err := l.root.OpenFile(temp, os.O_WRONLY|os.O_CREATE|os.O_EXCL, mode)
	if err != nil {
		return err
	}
	defer l.root.Remove(temp)
	_, err = f.Write(data)
	if err == nil {
		err = f.Sync()
	}
	err = errors.Join(err, f.Close())
	if err != nil {
		return err
	}
	return l.root.Rename(temp, target)
}
