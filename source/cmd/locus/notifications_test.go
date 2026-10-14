package main

import (
	"errors"
	"testing"

	"github.com/egoist/mygo/ui"
	"github.com/oneo-me/locus/internal/preferences"
	"github.com/oneo-me/onevra/source/components"
	"github.com/oneo-me/onevra/source/theme"
)

func TestNoticesRenderAsToasts(t *testing.T) {
	a := &application{cfg: preferences.Default()}
	a.cfg.Language = "english"
	a.errorNotice("first line\nsecond line")
	a.successNotice("Saved")
	if len(a.notices) != 2 {
		t.Fatalf("queued %d notices, want 2", len(a.notices))
	}
	if a.notices[0].kind != components.ToastError || a.notices[0].title != "first line" || a.notices[0].description != "second line" {
		t.Fatalf("error notice %+v", a.notices[0])
	}
	if a.notices[1].kind != components.ToastSuccess || a.notices[1].title != "Saved" {
		t.Fatalf("success notice %+v", a.notices[1])
	}
	tt := ui.NewTester(func(c *ui.Context) {
		c.SetTheme(theme.LightPalette())
		a.content(c)
		components.Toasts(c)
	}, 650, 400)
	if len(a.notices) != 0 {
		t.Fatalf("flush left %d notices queued", len(a.notices))
	}
	if !tt.HasText("first line") || !tt.HasText("second line") || !tt.HasText("Saved") {
		t.Fatalf("toasts missing after flush: %q", tt.Texts())
	}
	a.errorNotice("")
	if len(a.notices) != 0 {
		t.Fatal("an empty message queued a notice")
	}
}

func TestModelErrorReportsOnce(t *testing.T) {
	a := &application{cfg: preferences.Default()}
	a.model.Error = errors.New("scan failed")
	a.changed()
	a.changed()
	if len(a.notices) != 1 || a.notices[0].title != "scan failed" {
		t.Fatalf("repeated error queued %+v", a.notices)
	}
	a.model.Error = nil
	a.changed()
	a.model.Error = errors.New("scan failed")
	a.changed()
	if len(a.notices) != 2 {
		t.Fatalf("a cleared error did not report again: %+v", a.notices)
	}
}
