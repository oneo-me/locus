package main

import (
	"encoding/json"
	"math"
	"os"
	"reflect"
	"testing"
)

type atmosphereReference struct {
	Title       string
	Dark        bool
	Seed        uint32
	Wash, Panel atmosphereColor
	Backdrop    atmosphereBackdrop
	Shapes      []atmosphereShape
}

func atmosphereReferences(t *testing.T) []atmosphereReference {
	t.Helper()
	data, err := os.ReadFile("testdata/atmosphere_reference.json")
	if err != nil {
		t.Fatal(err)
	}
	var fixtures []atmosphereReference
	if err := json.Unmarshal(data, &fixtures); err != nil {
		t.Fatal(err)
	}
	return fixtures
}
func TestAtmosphereSourceParity(t *testing.T) {
	layouts, families, backdrops, kinds := map[uint32]bool{}, map[uint32]bool{}, map[backdropKind]bool{}, map[atmosphereKind]bool{}
	for _, want := range atmosphereReferences(t) {
		t.Run(want.Title+map[bool]string{true: "/dark", false: "/light"}[want.Dark], func(t *testing.T) {
			got := deriveNoteAtmosphere(want.Title, want.Dark)
			if got.seed != want.Seed || got.wash != want.Wash || got.panel != want.Panel || got.backdrop != want.Backdrop {
				t.Fatalf("palette/backdrop mismatch:\n%+v\nwant %+v", got, want)
			}
			if !reflect.DeepEqual(got.shapes, want.Shapes) {
				t.Fatalf("shapes mismatch:\n%+v\nwant %+v", got.shapes, want.Shapes)
			}
			if !reflect.DeepEqual(got, deriveNoteAtmosphere(want.Title, want.Dark)) {
				t.Fatal("unstable generation")
			}
			root := atmosphereMix(got.seed, 0x5a9e)
			layouts[atmosphereMix(root, 2)%6] = true
			families[atmosphereMix(root, 3)%10] = true
			backdrops[got.backdrop.Kind] = true
			for _, s := range got.shapes {
				kinds[s.Kind] = true
			}
		})
	}
	if len(layouts) != 6 || len(families) != 10 || len(backdrops) != 7 || len(kinds) != 5 {
		t.Fatalf("incomplete reference coverage: %d layouts, %d families, %d backdrops, %d shapes", len(layouts), len(families), len(backdrops), len(kinds))
	}
}
func TestAtmosphereLegacyExample(t *testing.T) {
	a := deriveNoteAtmosphere("Locus", false)
	if a.seed != 528513201 || a.wash.H != float32(241.6/360) || len(a.shapes) != 4 || a.backdrop.Kind != backdropWaves {
		t.Fatalf("reference's own regression case failed: %+v", a)
	}
	dark := deriveNoteAtmosphere("Locus", true)
	if a.panel.A <= dark.panel.A || a.backdrop.Color.A <= dark.backdrop.Color.A {
		t.Fatal("reference theme opacity relationship lost")
	}
}
func TestAtmosphereGeometryAndFades(t *testing.T) {
	for _, width := range []float32{320, 728, 832, 1200} {
		origin, scale := atmosphereGeometry(width)
		want := min(min(width-48, 832)/360, float32(2.08))
		if scale != want || origin != (width-360*scale)/2 {
			t.Fatalf("geometry for width %g: %g %g", width, origin, scale)
		}
	}
	for _, sample := range []struct{ y, want float32 }{{0, 1}, {187.6, 1}, {227.8, .5}, {268, 0}, {300, 0}} {
		if math.Abs(float64(atmosphereVerticalFade(sample.y)-sample.want)) > 1e-6 {
			t.Fatalf("vertical fade at %g", sample.y)
		}
	}
	if atmosphereShapeFade(75) != 1 || atmosphereShapeFade(87.5) != .5 || atmosphereShapeFade(100) != 0 {
		t.Fatal("shape vertical fade changed")
	}
	if atmosphereHorizontalFade(122.4) != 0 || atmosphereHorizontalFade(187.2) != 1 {
		t.Fatal("shape horizontal fade changed")
	}
	if got := atmosphereRotate(atmospherePoint{2, 1}, atmospherePoint{1, 1}, 90); math.Abs(float64(got.X-1))+math.Abs(float64(got.Y-2)) > 1e-6 {
		t.Fatalf("rotation: %+v", got)
	}
}
