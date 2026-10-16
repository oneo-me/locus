package main

import (
	"fmt"
	"image"
	"image/color"
	"os"
	"path/filepath"
	"testing"
)

func TestResourcePathKeepsFileNameBytes(t *testing.T) {
	for _, target := range []string{"assets/a.png", "assets/a b.png", "assets/a&b.png", "assets/a#b.png", "assets/a?b.png", "assets/中文.png"} {
		p, err := resourcePath("note", target)
		if err != nil {
			t.Fatalf("%q: %v", target, err)
		}
		if want := "note/" + target; p != want {
			t.Errorf("%q: got %q, want %q", target, p, want)
		}
	}
	if p, err := resourcePath("note", "https://example.com/a.png"); err != nil || p != "" {
		t.Errorf("remote image resolved: %q, %v", p, err)
	}
	if p, err := resourcePath("note", "locus://note/assets/a.png"); err != nil || p != "note/assets/a.png" {
		t.Errorf("address resolved: %q, %v", p, err)
	}
}

func TestDecodeGalleryImageScalesOversized(t *testing.T) {
	data := iconPNG(t, 4000, 3000)
	source := int64(4000) * 3000
	bitmap, retained, err := decodeGalleryImage(data, source, 4_000_000)
	if err != nil {
		t.Fatal(err)
	}
	if retained < 3_000_000 || retained > 4_000_000 {
		t.Fatalf("retained %d pixels outside the budget", retained)
	}
	w, h := bitmap.Size()
	if w*h > 4_000_000 || w < 2000 || h < 1500 {
		t.Fatalf("scaled size %dx%d", w, h)
	}
}

func TestLoadImagesScalesOversizedInsteadOfDropping(t *testing.T) {
	root := t.TempDir()
	os.MkdirAll(filepath.Join(root, "note", "assets"), 0700)
	if err := os.WriteFile(filepath.Join(root, "note", "index.md"), []byte("x"), 0600); err != nil {
		t.Fatal(err)
	}
	var entries []string
	body := "```gallery\n"
	for i := range 5 {
		name := fmt.Sprintf("photo%d.png", i)
		if err := os.WriteFile(filepath.Join(root, "note", "assets", name), iconPNG(t, 4000, 3000), 0600); err != nil {
			t.Fatal(err)
		}
		entries = append(entries, "assets/"+name)
		body += fmt.Sprintf("![Photo %d](assets/%s)\n", i, name)
	}
	body += "```"
	a := &application{}
	a.model.Root = root
	a.model.Selected = "note"
	a.model.Document.Body = body
	a.loadImages()
	<-a.imageReady
	for _, target := range entries {
		if a.imageLoad[target] == nil {
			t.Fatalf("%s marked unavailable", target)
		}
	}
}

func TestOrientedImageBounds(t *testing.T) {
	img := image.NewNRGBA(image.Rect(0, 0, 2, 1))
	img.Set(0, 0, color.NRGBA{R: 255, A: 255})
	img.Set(1, 0, color.NRGBA{G: 255, A: 255})
	o := orientedImage{img: img, o: 6} // rotate 90° clockwise
	if b := o.Bounds(); b.Dx() != 1 || b.Dy() != 2 {
		t.Fatalf("bounds %v", b)
	}
	if got := o.At(0, 0); got != (color.NRGBA{R: 255, A: 255}) {
		t.Errorf("At(0,0) = %v", got)
	}
	if got := o.At(0, 1); got != (color.NRGBA{G: 255, A: 255}) {
		t.Errorf("At(0,1) = %v", got)
	}
}
