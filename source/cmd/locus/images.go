package main

import (
	"bytes"
	"errors"
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
	root, note, body := a.model.Root, a.model.Selected, a.model.Document.Body
	a.images = nil
	a.galleryPreviewOpen = false
	ready := make(chan struct{})
	a.imageMu.Lock()
	a.imageTicket++
	ticket := a.imageTicket
	a.imageReady = ready
	a.imageLoad = nil
	a.imageMu.Unlock()
	if root == "" || note == "" || body == "" {
		close(ready)
		return
	}
	go func() {
		images := map[string]*ui.Bitmap{}
		pixels := int64(0)
		lib, err := library.Open(root)
		if err != nil {
			close(ready)
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
		a.imageMu.Lock()
		if ticket == a.imageTicket {
			a.imageLoad = images
		}
		a.imageMu.Unlock()
		close(ready)
		a.post(func() {
			if ticket == a.imageTicket {
				a.images = images
			}
		})
	}()
}

// loadMarkdownImage resolves a Markdown image through the note's shared,
// bounded loader. MyGo calls it on its own goroutine, so it waits for the
// current generation and returns the decoded bitmap instead of loading files
// during frame construction.
func (a *application) loadMarkdownImage(target string) (ui.ImageSource, error) {
	a.imageMu.Lock()
	ready := a.imageReady
	a.imageMu.Unlock()
	if ready != nil {
		<-ready
	}
	a.imageMu.Lock()
	bitmap := a.imageLoad[target]
	a.imageMu.Unlock()
	if bitmap == nil {
		return nil, errors.New("image unavailable")
	}
	return bitmap, nil
}
