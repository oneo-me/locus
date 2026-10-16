package main

import (
	"reflect"
	"testing"

	"github.com/egoist/mygo/ui"
	"github.com/egoist/mygo/ui/markdown"
	"github.com/oneo-me/locus/internal/preferences"
	"github.com/oneo-me/onevra/source/theme"
)

func TestGallerySyntax(t *testing.T) {
	for _, body := range []string{
		"![Sea](sea.png)\n![Coast](assets/coast.png)",
		"![Sea](sea.png)\n\n![Coast](assets/coast.png)",
		"![Sea][a]\n![Coast](assets/coast.png)\n\n[a]: sea.png",
	} {
		got := parseGallery(body)
		want := []galleryItem{{"assets/sea.png", "Sea"}, {"assets/coast.png", "Coast"}}
		if got.Error != "" || !reflect.DeepEqual(got.Items, want) {
			t.Fatalf("%q: %+v", body, got)
		}
	}
	for _, body := range []string{"", "![](assets/a.png)", "Text ![A](assets/a.png)", "- ![A](assets/a.png)", "![A](https://example.com/a.png)", "![A](../a.png)", "![A](assets/%2e%2e/a.png)", "![A](/assets/a.png)", "![A](locus://other/assets/a.png)", "![A](assets/a.png?query)", "[![A](assets/a.png)](https://example.com)"} {
		if got := parseGallery(body); got.Error == "" || len(got.Items) != 0 {
			t.Errorf("accepted invalid gallery %q: %+v", body, got)
		}
	}
	got := parseGallery("![A &amp; B](assets/a%20b.png)")
	if got.Error != "" || got.Items[0] != (galleryItem{"assets/a%20b.png", "A & B"}) {
		t.Fatal(got)
	}
}

func TestGalleryFenceDiscovery(t *testing.T) {
	body := "![Outside](assets/out.png)\n\n```go\n![Not an image](assets/code.png)\n```\n\n> ```gallery\n> ![One](one.png)\n> ```\n\n~~~gallery\n![Two](assets/two.png)\n~~~\n\n```Gallery\n![Case sensitive](assets/no.png)\n```"
	galleries, targets := markdownResources(body)
	alts := map[string]bool{}
	for _, gallery := range galleries {
		for _, item := range gallery.Items {
			alts[item.Alt] = true
		}
	}
	if len(galleries) != 2 || !alts["One"] || !alts["Two"] {
		t.Fatal(galleries)
	}
	if !reflect.DeepEqual(targets, []string{"assets/out.png", "assets/one.png", "assets/two.png"}) {
		t.Fatal(targets)
	}
}

func TestGalleryPreview(t *testing.T) {
	bitmap, err := ui.DecodeBitmap(iconPNG(t, 320, 180))
	if err != nil {
		t.Fatal(err)
	}
	for _, dark := range []bool{false, true} {
		a := &application{cfg: preferences.Default(), images: map[string]*ui.Bitmap{"assets/one.png": bitmap}}
		a.cfg.Language = "english"
		body := "```gallery\n![One](one.png)\n![Missing](missing.png)\n```"
		a.galleries, _ = markdownResources(body)
		opts := markdown.Options{Code: map[string]func(*ui.Context, *markdown.CodeBlockView){"gallery": a.renderGallery}}
		tt := ui.NewTester(func(c *ui.Context) {
			c.SetTheme(theme.Palette(dark))
			markdown.View(c, body, &opts).FillWidth()
			a.galleryDialog(c)
		}, 420, 600)
		if !tt.HasText("Missing\nImage unavailable") {
			t.Fatal("missing image fallback absent")
		}
		if err := tt.Click("One"); err != nil {
			t.Fatal(err)
		}
		if !a.galleryPreviewOpen {
			t.Fatal("preview did not open")
		}
		tt.Frame()
		if err := tt.Click("Close"); err != nil {
			t.Fatal(err)
		}
		if a.galleryPreviewOpen {
			t.Fatal("preview did not close")
		}
		tt.Frame()
		if err := tt.Click("One"); err != nil {
			t.Fatal(err)
		}
		tt.Frame()
		tt.Key(0, ui.KeyEscape)
		if a.galleryPreviewOpen {
			t.Fatal("Escape did not close preview")
		}

	}
}
