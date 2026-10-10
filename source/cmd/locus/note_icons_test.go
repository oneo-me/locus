package main

import (
	"bytes"
	"image"
	"image/color"
	"image/png"
	"testing"

	"github.com/egoist/mygo/ui"
	"github.com/oneo-me/locus/internal/library"
	"github.com/oneo-me/locus/internal/preferences"
)

func TestNoteEmoji(t *testing.T) {
	for _, value := range []string{"😀", "👩🏽‍💻", "🇨🇳", "❤️", "1️⃣"} {
		if noteEmoji(value) != value {
			t.Errorf("lost emoji %q", value)
		}
	}
	for _, value := range []string{"", "assets/icon.png", "https://host/icon.png", "../icon.png", "title", "123", "😀\n"} {
		if noteEmoji(value) != "" {
			t.Errorf("accepted %q", value)
		}
	}
}

func iconPNG(t *testing.T, width, height int) []byte {
	t.Helper()
	src := image.NewNRGBA(image.Rect(0, 0, width, height))
	for y := 0; y < height; y++ {
		for x := 0; x < width; x++ {
			src.Set(x, y, color.NRGBA{R: 240, A: 255})
		}
	}
	var data bytes.Buffer
	if err := png.Encode(&data, src); err != nil {
		t.Fatal(err)
	}
	return data.Bytes()
}

func TestDecodeNoteIcon(t *testing.T) {
	if decodeNoteIcon(iconPNG(t, 64, 32)) == nil {
		t.Fatal("valid PNG rejected")
	}
	for _, data := range [][]byte{nil, []byte("not PNG"), iconPNG(t, 1001, 1000), make([]byte, (4<<20)+1)} {
		if decodeNoteIcon(data) != nil {
			t.Fatal("invalid or oversized PNG accepted")
		}
	}
}

func TestSidebarNoteIcons(t *testing.T) {
	a := &application{cfg: preferences.Default()}
	a.cfg.Language = "english"
	a.model.Notes = []*library.Note{
		{Path: "emoji", Title: "Emoji note", HasBody: true, Icon: "👩🏽‍💻"},
		{Path: "image", Title: "Image note", HasBody: true, Icon: "assets/icon.png"},
		{Path: "missing", Title: "Missing icon", HasBody: true, Icon: "assets/icon.png"},
	}
	a.noteIcons = map[string]*ui.Bitmap{"image": decodeNoteIcon(iconPNG(t, 32, 32))}
	a.resetTree()
	tt := ui.NewTester(a.left, 252, 600)
	if !tt.HasText("👩🏽‍💻") {
		t.Fatal("missing emoji")
	}
	for _, n := range a.model.Notes {
		if !tt.HasText(n.Title) {
			t.Fatalf("lost row %s", n.Path)
		}
	}
	if icon, ok := tt.Find("Note icon"); !ok || icon.W != 14 || icon.H != 14 {
		t.Fatalf("expected 14-pixel sidebar icon: %+v", icon)
	}
	a.model.Selected = "emoji"
	title := ui.NewTester(a.title, 650, 48)
	if !title.HasText("👩🏽‍💻") || !title.HasText("Emoji note") {
		t.Fatal("title lost note icon")
	}
	a.model.Selected = "image"
	title = ui.NewTester(a.title, 650, 48)
	icon, ok := title.Find("Note icon")
	if !ok || icon.W != 16 || icon.H != 16 {
		t.Fatalf("expected unchanged 16-pixel title icon: %+v", icon)
	}
	label, ok := title.Find("Image note")
	if !ok || label.X-(icon.X+icon.W) != 8 {
		t.Fatalf("expected 8-pixel icon/title gap: icon=%+v title=%+v", icon, label)
	}
	n := &library.Note{Path: "note", HasBody: true}
	for _, value := range []string{"assets/icon.png", "icon.png"} {
		n.Icon = value
		if iconImagePath(n) != "note/assets/icon.png" {
			t.Fatal("incorrect image path")
		}
	}
	n.Icon = "../icon.png"
	if iconImagePath(n) != "" {
		t.Fatal("unsafe image reference accepted")
	}
}
