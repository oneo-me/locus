package main

import (
	"bytes"
	"image"
	"image/png"
	"path"
	"unicode/utf8"

	"github.com/egoist/mygo/ui"
	"github.com/oneo-me/locus/internal/library"
	"github.com/oneo-me/onevra/source/components"
	"github.com/oneo-me/onevra/source/icons/lucide"
)

// Text icons retain variation selectors, joiners, modifiers, and flag sequences.
// Paths, URLs, control characters, and ordinary prose are not rendered as icons.
func noteEmoji(value string) string {
	if utf8.RuneCountInString(value) > 32 {
		return ""
	}
	hasEmoji := false
	for _, r := range value {
		switch {
		case r >= 0x1f000 && r <= 0x1faff, r >= 0x2600 && r <= 0x27bf,
			r >= 0x2300 && r <= 0x23ff, r >= 0x2b00 && r <= 0x2bff,
			r >= 0x2190 && r <= 0x21ff, r == 0xa9, r == 0xae, r == 0x203c,
			r == 0x2049, r == 0x2122, r == 0x2139, r == 0x3030, r == 0x303d,
			r == 0x3297, r == 0x3299, r == 0x20e3:
			hasEmoji = true
		case r == 0xfe0f, r == 0xfe0e, r == 0x200d, r >= 0xe0020 && r <= 0xe007f,
			r >= '0' && r <= '9', r == '#', r == '*':
		default:
			return ""
		}
	}
	if !hasEmoji {
		return ""
	}
	return value
}

func (a *application) noteIcon(c *ui.Context, n *library.Note, size float32) {
	if bitmap := a.noteIcons[n.Path]; bitmap != nil {
		components.Image(c, bitmap).Label("Note icon").Size(size, size).Shrink(0).Fit(ui.Contain)
	} else if emoji := noteEmoji(n.Icon); emoji != "" {
		components.Text(c, emoji).Size(size, 20).Shrink(0).FontSize(size - 2).SingleLine()
	} else {
		icon := lucide.FileText
		if !n.HasBody {
			icon = lucide.Folder
		} else if n.Canvas != "" {
			icon = lucide.PanelsTopLeft
		}
		components.Icon(c, icon).Size(size, size).Shrink(0).TextColor(c.Theme().TextMuted)
	}
}

func iconImagePath(n *library.Note) string {
	if n.HasBody && (n.Icon == "assets/icon.png" || n.Icon == "icon.png") {
		return path.Join(n.Path, "assets/icon.png")
	}
	return ""
}

// A successful scan replaces the note objects even when only assets changed.
// Selection changes reuse the projection and do not restart icon decoding.
func (a *application) loadNoteIcons() {
	var projection *library.Note
	if len(a.model.Notes) > 0 {
		projection = a.model.Notes[0]
	}
	if a.iconRoot == a.model.Root && a.iconProjection == projection {
		return
	}
	a.iconRoot, a.iconProjection = a.model.Root, projection
	a.iconTicket++
	ticket, root := a.iconTicket, a.model.Root
	a.noteIcons = nil
	var targets []string
	var collect func([]*library.Note)
	collect = func(notes []*library.Note) {
		for _, n := range notes {
			if iconImagePath(n) != "" && len(targets) < 512 {
				targets = append(targets, n.Path)
			}
			collect(n.Children)
		}
	}
	collect(a.model.Notes)
	if root == "" || len(targets) == 0 {
		return
	}
	go func() {
		lib, err := library.Open(root)
		if err != nil {
			return
		}
		defer lib.Close()
		icons := map[string]*ui.Bitmap{}
		for _, note := range targets {
			data, err := lib.Resource(path.Join(note, "assets/icon.png"))
			if err != nil {
				continue
			}
			if bitmap := decodeNoteIcon(data); bitmap != nil {
				icons[note] = bitmap
			}
		}
		a.post(func() {
			if ticket == a.iconTicket && root == a.model.Root {
				a.noteIcons = icons
			}
		})
	}()
}

func decodeNoteIcon(data []byte) *ui.Bitmap {
	if len(data) > 4<<20 {
		return nil
	}
	size, err := png.DecodeConfig(bytes.NewReader(data))
	if err != nil || size.Width <= 0 || size.Height <= 0 || int64(size.Width)*int64(size.Height) > 1_000_000 {
		return nil
	}
	src, err := png.Decode(bytes.NewReader(data))
	if err != nil {
		return nil
	}
	// Cache small thumbnails rather than retaining full-resolution source images.
	w, h := 32, 32
	if size.Width > size.Height {
		h = max(1, 32*size.Height/size.Width)
	} else {
		w = max(1, 32*size.Width/size.Height)
	}
	dst := image.NewNRGBA(image.Rect(0, 0, w, h))
	for y := 0; y < h; y++ {
		for x := 0; x < w; x++ {
			dst.Set(x, y, src.At(x*size.Width/w, y*size.Height/h))
		}
	}
	return ui.NewBitmap(dst)
}
