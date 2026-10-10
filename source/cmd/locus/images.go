package main

import (
	"bytes"
	"image"
	_ "image/gif"
	_ "image/jpeg"
	_ "image/png"
	"net/url"
	"path"
	"strings"

	"github.com/egoist/mygo/ui"
	"github.com/oneo-me/locus/internal/library"
)

func resourcePath(note, target string) (string, error) {
	if strings.HasPrefix(target, "locus://") {
		return library.DecodeResource(strings.TrimPrefix(target, "locus://"))
	}
	u, err := url.Parse(target)
	if err != nil {
		return "", err
	}
	if u.Scheme != "" || u.Host != "" {
		return "", nil
	}
	decoded, err := url.PathUnescape(u.EscapedPath())
	if err != nil {
		return "", err
	}
	p := path.Join(note, decoded)
	return library.DecodeResource(strings.TrimPrefix(library.Address(p), "locus://"))
}
func (a *application) loadImages() {
	key := a.model.Root + "\x00" + a.model.Selected + "\x00" + a.model.Document.Body
	if key == a.imageKey {
		return
	}
	a.imageKey = key
	a.imageTicket++
	ticket := a.imageTicket
	root, note, body := a.model.Root, a.model.Selected, a.model.Document.Body
	a.images = nil
	a.galleryPreviewOpen = false
	if root == "" || note == "" || body == "" {
		return
	}
	go func() {
		images := map[string]*ui.Bitmap{}
		pixels := int64(0)
		lib, err := library.Open(root)
		if err != nil {
			return
		}
		defer lib.Close()
		_, targets := markdownResources(body)
		seen := map[string]bool{}
		for _, target := range targets {
			if seen[target] {
				continue
			}
			seen[target] = true
			if len(images) >= 64 {
				break
			}
			p, err := resourcePath(note, target)
			if err != nil || p == "" {
				continue
			}
			data, err := lib.Resource(p)
			if err != nil {
				continue
			}
			size, _, err := image.DecodeConfig(bytes.NewReader(data))
			if err != nil || size.Width <= 0 || size.Height <= 0 || int64(size.Width)*int64(size.Height) > 16_000_000 || pixels+int64(size.Width)*int64(size.Height) > 32_000_000 {
				continue
			}
			bitmap, err := ui.DecodeBitmap(data)
			if err == nil {
				images[target] = bitmap
				pixels += int64(size.Width) * int64(size.Height)
			}
		}
		a.post(func() {
			if ticket == a.imageTicket {
				a.images = images
			}
		})
	}()
}
