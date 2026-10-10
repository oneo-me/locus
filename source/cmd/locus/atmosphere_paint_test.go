package main

import (
	"testing"

	"github.com/egoist/mygo/ui"
)

type atmospherePaintRecorder struct {
	gradientBounds        ui.Rect
	gradient              ui.LinearGradient
	fills, strokes, paths int
	pathColors            []ui.Color
}

func (r *atmospherePaintRecorder) Fill(_ ui.Rect, _ ui.Color, _ float32)      { r.fills++ }
func (r *atmospherePaintRecorder) Stroke(_ ui.Rect, _ ui.Color, _, _ float32) { r.strokes++ }
func (r *atmospherePaintRecorder) FillGradient(bounds ui.Rect, g ui.LinearGradient, _ float32) {
	r.gradientBounds = bounds
	r.gradient = g
}
func (r *atmospherePaintRecorder) FillPath(_ *ui.Path, _ ui.Color) { r.fills++ }
func (r *atmospherePaintRecorder) StrokePath(_ *ui.Path, _ float32, c ui.Color) {
	r.paths++
	r.pathColors = append(r.pathColors, c)
}

func TestAtmospherePaintHeightIndependentOfArticleLength(t *testing.T) {
	a := deriveNoteAtmosphere("Locus", false)
	for _, height := range []float32{100, 480, 4000} {
		var p atmospherePaintRecorder
		a.draw(&p, ui.Rect{X: 252, Y: 0, W: 728, H: height})
		if p.gradientBounds.H != 268 || p.gradientBounds.W != 728 || p.gradientBounds.X != 252 {
			t.Fatalf("wash bounds: %+v", p.gradientBounds)
		}
		if p.gradient.From != a.wash.ui() || p.gradient.To.A != 0 || p.gradient.Angle != 180 {
			t.Fatalf("wash: %+v", p.gradient)
		}
	}
}

func TestAtmosphereTextureMasksStayCellSized(t *testing.T) {
	// Full-width rotated rows occupy large rectangular masks in MyGo's glyph
	// atlas, causing unrelated text to disappear mid-frame. Require individual
	// cell submissions at both reading widths, preserving the reference fade.
	for _, f := range atmosphereReferences(t) {
		b := f.Backdrop
		if b.Kind == backdropNone || b.Kind == backdropDots {
			continue
		}
		for _, width := range []float32{728, 1068} {
			bounds := ui.Rect{X: 252, Y: 48, W: width, H: 268}
			var p atmospherePaintRecorder
			b.draw(&p, bounds)
			columns := 0
			for x := bounds.X - b.Spacing*3 + b.OffsetX; x < bounds.X+bounds.W+b.Spacing*3; x += b.Spacing {
				columns++
			}
			expected := 0
			for y := bounds.Y - b.Spacing*3 + b.OffsetY; y < bounds.Y+bounds.H+b.Spacing*3; y += b.Spacing {
				fade := atmosphereVerticalFade(y - bounds.Y)
				if fade == 0 {
					continue
				}
				color := b.Color.opacity(fade).ui()
				for i := 0; i < columns; i++ {
					if expected >= len(p.pathColors) || p.pathColors[expected] != color {
						t.Fatalf("%q: texture batching or row opacity changed", f.Title)
					}
					expected++
				}
			}
			if p.paths != expected {
				t.Fatalf("%q: submitted %d paths, want %d cells", f.Title, p.paths, expected)
			}
		}
	}
}
