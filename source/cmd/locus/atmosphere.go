package main

import (
	"math"

	"github.com/egoist/mygo/ui"
)

// Port of note_atmosphere.rs at eabab1e. Keep the tables, salts, rounding,
// float64 generation and float32 geometry in sync with that reference.
const (
	noteMaxWidth          float32 = 832
	atmosphereHeight      float32 = 268
	atmosphereShapeHeight float32 = 208
)

type atmosphereColor struct{ H, S, L, A float32 }

func atmosphereHSL(h, s, l, a float64) atmosphereColor {
	return atmosphereColor{float32(h / 360), float32(s / 100), float32(l / 100), float32(a)}
}
func (c atmosphereColor) opacity(a float32) atmosphereColor { c.A *= a; return c }
func (c atmosphereColor) ui() ui.Color {
	h, s, l := float64(c.H)*360, float64(c.S), float64(c.L)
	a := s * math.Min(l, 1-l)
	channel := func(n float64) uint8 {
		k := math.Mod(n+h/30, 12)
		return uint8(math.Round(255 * (l - a*math.Max(-1, math.Min(math.Min(k-3, 9-k), 1)))))
	}
	return ui.RGBA(channel(0), channel(8), channel(4), c.A)
}

type atmosphereKind uint8

const (
	atmosphereDisk atmosphereKind = iota
	atmosphereRing
	atmosphereRect
	atmosphereArc
	atmosphereTriangle
)

type backdropKind uint8

const (
	backdropNone backdropKind = iota
	backdropGrid
	backdropLines
	backdropDots
	backdropDiagonal
	backdropDiamonds
	backdropWaves
)

type atmospherePoint struct{ X, Y float32 }
type atmosphereShape struct {
	Kind                                                           atmosphereKind
	X, Y, Radius, Width, Height, Rotate, StrokeWidth, Start, Sweep float32
	Points                                                         [3]atmospherePoint
	Color                                                          atmosphereColor
}
type atmosphereBackdrop struct {
	Kind                                                              backdropKind
	Spacing, Rotate, OffsetX, OffsetY, StrokeWidth, Radius, Amplitude float32
	Color                                                             atmosphereColor
}
type noteAtmosphere struct {
	seed        uint32
	wash, panel atmosphereColor
	backdrop    atmosphereBackdrop
	shapes      []atmosphereShape
}
type atmosphereSlot [5]float64

var atmosphereLayouts = [6][6]atmosphereSlot{
	{{286, 38, 1, 15, 9}, {214, 74, .74, 13, 8}, {328, 74, .55, 9, 7}, {243, 19, .48, 11, 6}, {170, 54, .42, 8, 10}, {333, 18, .38, 8, 5}},
	{{270, 66, 1, 16, 9}, {328, 30, .72, 10, 8}, {205, 27, .58, 13, 8}, {238, 79, .48, 12, 6}, {176, 65, .4, 9, 8}, {315, 82, .36, 9, 5}},
	{{305, 50, 1, 15, 10}, {229, 24, .7, 13, 8}, {210, 78, .58, 12, 7}, {333, 20, .48, 9, 6}, {272, 82, .42, 11, 6}, {172, 48, .36, 8, 10}},
	{{246, 45, 1, 16, 10}, {321, 65, .74, 11, 8}, {296, 18, .58, 12, 6}, {189, 74, .48, 10, 7}, {343, 26, .42, 7, 7}, {170, 28, .36, 8, 8}},
	{{294, 68, 1, 16, 9}, {244, 24, .74, 13, 8}, {333, 45, .58, 9, 9}, {204, 72, .5, 11, 7}, {175, 34, .42, 8, 8}, {276, 84, .36, 11, 5}},
	{{263, 31, 1, 16, 9}, {320, 75, .72, 11, 7}, {208, 67, .58, 12, 8}, {341, 29, .48, 8, 7}, {176, 52, .42, 9, 10}, {250, 82, .36, 11, 5}},
}
var atmosphereFamilies = [10][6]atmosphereKind{
	{atmosphereDisk, atmosphereRing, atmosphereArc, atmosphereDisk, atmosphereRing, atmosphereTriangle},
	{atmosphereRect, atmosphereArc, atmosphereDisk, atmosphereRing, atmosphereRect, atmosphereTriangle},
	{atmosphereTriangle, atmosphereArc, atmosphereRect, atmosphereRing, atmosphereDisk, atmosphereArc},
	{atmosphereRing, atmosphereRect, atmosphereDisk, atmosphereTriangle, atmosphereArc, atmosphereRect},
	{atmosphereArc, atmosphereDisk, atmosphereTriangle, atmosphereRing, atmosphereRect, atmosphereDisk},
	{atmosphereDisk, atmosphereTriangle, atmosphereRect, atmosphereArc, atmosphereRing, atmosphereTriangle},
	{atmosphereRing, atmosphereArc, atmosphereRect, atmosphereDisk, atmosphereTriangle, atmosphereRing},
	{atmosphereRect, atmosphereRing, atmosphereTriangle, atmosphereDisk, atmosphereArc, atmosphereRect},
	{atmosphereTriangle, atmosphereDisk, atmosphereArc, atmosphereRect, atmosphereRing, atmosphereDisk},
	{atmosphereArc, atmosphereRect, atmosphereRing, atmosphereTriangle, atmosphereDisk, atmosphereArc},
}

func atmosphereHash(title string) uint32 {
	h := uint32(2166136261)
	for _, ch := range title {
		scalar := uint32(ch)
		if scalar > 0xffff {
			scalar = 0xd800 + ((scalar - 0x10000) >> 10)
		}
		h = (h ^ scalar) * 16777619
	}
	return h
}
func atmosphereMix(h, salt uint32) uint32 {
	n := h ^ salt
	n = (n ^ (n >> 16)) * 2246822507
	n = (n ^ (n >> 13)) * 3266489909
	return n ^ (n >> 16)
}
func atmosphereUnit(h uint32) float64 { return float64(h) / 4294967296 }
func atmosphereRound(v float64, digits int) float64 {
	factor := math.Pow10(digits)
	return math.Round(v*factor) / factor
}
func atmosphereClamp(v, lo, hi float64) float64 { return min(hi, max(lo, v)) }
func atmosphereTheme(dark bool, d, l float64) float64 {
	if dark {
		return d
	}
	return l
}
func atmosphereTone(hue float64, dark, fill bool, salt uint32) atmosphereColor {
	if fill {
		return atmosphereHSL(hue, atmosphereTheme(dark, 32, 36)+float64(salt%10), atmosphereTheme(dark, 52+float64(salt%10), 56+float64(salt%12)), 1)
	}
	return atmosphereHSL(hue, atmosphereTheme(dark, 28, 32)+float64(salt%8), atmosphereTheme(dark, 68, 40)+float64(salt%8), 1)
}
func deriveNoteAtmosphere(title string, dark bool) noteAtmosphere {
	seed := atmosphereHash(title)
	hue := atmosphereRound(190+atmosphereUnit(atmosphereMix(seed, 1))*60, 1)
	a := noteAtmosphere{seed: seed,
		wash:     atmosphereHSL(hue, atmosphereTheme(dark, 36, 40), atmosphereTheme(dark, 48, 62), atmosphereTheme(dark, .16, .12)),
		panel:    atmosphereHSL(hue, atmosphereTheme(dark, 34, 42), atmosphereTheme(dark, 28, 90), atmosphereTheme(dark, .42, .58)),
		backdrop: deriveAtmosphereBackdrop(seed, hue, dark)}
	root := atmosphereMix(seed, 0x5a9e)
	layout := atmosphereLayouts[atmosphereMix(root, 2)%6]
	family := atmosphereFamilies[atmosphereMix(root, 3)%10]
	for i := 0; i < int(4+atmosphereMix(root, 1)%3); i++ {
		a.shapes = append(a.shapes, deriveAtmosphereShape(root, i, layout[i], family[i], hue, dark))
	}
	return a
}
func deriveAtmosphereBackdrop(seed uint32, hue float64, dark bool) atmosphereBackdrop {
	local := atmosphereMix(seed, 0xbd01)
	unit := func(salt uint32) float64 { return atmosphereUnit(atmosphereMix(local, salt)) }
	opacity := atmosphereRound(atmosphereTheme(dark, .035, .06)+unit(2)*atmosphereTheme(dark, .025, .04), 3)
	stroke := atmosphereTone(hue, dark, false, atmosphereMix(local, 3)%16).opacity(float32(opacity))
	spacing := float32(atmosphereRound(18+unit(4)*16, 2))
	p := atmosphereBackdrop{Kind: backdropKind(atmosphereMix(local, 1) % 7), Spacing: spacing,
		Rotate: float32(atmosphereRound(-8+unit(5)*16, 2)), OffsetX: float32(atmosphereRound(unit(6)*float64(spacing), 2)), OffsetY: float32(atmosphereRound(unit(7)*float64(spacing), 2)), StrokeWidth: float32(atmosphereRound(.5+unit(8)*.45, 2)), Color: stroke}
	switch p.Kind {
	case backdropNone:
		return atmosphereBackdrop{}
	case backdropGrid:
		p.Color = stroke.opacity(.72)
	case backdropDots:
		p.Radius = float32(atmosphereRound(.55+unit(9)*.55, 2))
	case backdropDiagonal:
		p.Spacing = float32(atmosphereRound(float64(spacing)*1.08, 2))
		p.Rotate = float32(atmosphereRound(float64(p.Rotate)+24, 2))
	case backdropDiamonds:
		p.Spacing = float32(atmosphereRound(float64(spacing)*1.16, 2))
		p.Rotate = float32(atmosphereRound(float64(p.Rotate)*.45, 2))
		p.StrokeWidth = float32(atmosphereRound(float64(p.StrokeWidth)*.9, 2))
		p.Color = stroke.opacity(.78)
	case backdropWaves:
		p.Spacing = float32(atmosphereRound(float64(spacing)*1.24, 2))
		p.Rotate = float32(atmosphereRound(float64(p.Rotate)*.32, 2))
		p.StrokeWidth = float32(atmosphereRound(float64(p.StrokeWidth)*.92, 2))
		p.Color = stroke.opacity(.84)
		p.Amplitude = float32(atmosphereRound(atmosphereClamp(float64(p.Spacing)*(.11+unit(10)*.05), 2.4, 5.8), 2))
	}
	return p
}
func deriveAtmosphereShape(root uint32, index int, slot atmosphereSlot, kind atmosphereKind, hue float64, dark bool) atmosphereShape {
	local := atmosphereMix(root, 20+uint32(index)*11)
	unit := func(salt uint32) float64 { return atmosphereUnit(atmosphereMix(local, salt)) }
	prominence := atmosphereClamp(slot[2]*(.88+unit(1)*.24), .34, 1.08)
	opacity := atmosphereRound(atmosphereTheme(dark, .085, .065)+prominence*.035+unit(2)*atmosphereTheme(dark, .045, .035), 3)
	hue = atmosphereClamp(hue+[...]float64{-9, 6, -4, 10, 2, -7}[index], 190, 250)
	fill := atmosphereTone(hue, dark, true, atmosphereMix(local, 3)%20).opacity(float32(opacity))
	stroke := atmosphereTone(hue, dark, false, atmosphereMix(local, 4)%20)
	rawX, rawY := slot[0]+(unit(5)*2-1)*slot[3], slot[1]+(unit(6)*2-1)*slot[4]
	mx, my := atmosphereMotionMargins(local, kind, prominence)
	center := func(extent float64) (float32, float32) {
		return float32(atmosphereRound(atmosphereClamp(rawX, 132+extent+mx, 356-extent-mx), 2)), float32(atmosphereRound(atmosphereClamp(rawY, 4+extent+my, 96-extent-my), 2))
	}
	s := atmosphereShape{Kind: kind, Color: fill}
	switch kind {
	case atmosphereDisk:
		r := atmosphereRound((17+unit(7)*11)*prominence, 2)
		s.X, s.Y = center(r)
		s.Radius = float32(r)
	case atmosphereRing:
		r := atmosphereRound((17+unit(7)*10)*prominence, 2)
		w := atmosphereRound(1.15+unit(8)*1.45*prominence, 2)
		s.X, s.Y = center(r + w)
		s.Radius = float32(r)
		s.StrokeWidth = float32(w)
		s.Color = stroke.opacity(float32(opacity + .018))
	case atmosphereRect:
		h := atmosphereRound((18+unit(7)*8)*prominence, 2)
		w := atmosphereRound(h*(1.45+unit(8)*.55), 2)
		cx, cy := center(math.Hypot(w, h) / 2)
		s.X = float32(atmosphereRound(float64(cx)-w/2, 2))
		s.Y = float32(atmosphereRound(float64(cy)-h/2, 2))
		s.Width = float32(w)
		s.Height = float32(h)
		s.Radius = float32(atmosphereRound(min(h*.22, (4+unit(9)*3)*prominence), 2))
		s.Rotate = float32(atmosphereRound(-14+unit(10)*28, 2))
	case atmosphereArc:
		r := atmosphereRound((18+unit(7)*10)*prominence, 2)
		w := atmosphereRound(1.2+unit(10)*1.8*prominence, 2)
		s.X, s.Y = center(r + w)
		s.Radius = float32(r)
		s.StrokeWidth = float32(w)
		s.Start = float32(atmosphereRound(unit(8)*360, 1))
		s.Sweep = float32(atmosphereRound(105+unit(9)*105, 1))
		s.Color = stroke.opacity(float32(opacity + .024))
	case atmosphereTriangle:
		size := atmosphereRound((20+unit(7)*11)*prominence, 2)
		cx, cy := center(size)
		angle := unit(8) * 2 * math.Pi
		for i := range s.Points {
			theta := angle + 2*math.Pi*float64(i)/3
			s.Points[i] = atmospherePoint{float32(atmosphereRound(float64(cx)+math.Cos(theta)*size, 2)), float32(atmosphereRound(float64(cy)+math.Sin(theta)*size, 2))}
		}
	}
	return s
}
func atmosphereMotionMargins(local uint32, kind atmosphereKind, prominence float64) (float64, float64) {
	unit := func(salt uint32) float64 { return atmosphereUnit(atmosphereMix(local, salt)) }
	angle := unit(34) * 2 * math.Pi
	if kind == atmosphereArc {
		scale := 8 + unit(36)*6
		return math.Abs(math.Cos(angle)*scale) * .65, math.Abs(math.Sin(angle)*scale*.68) * .65
	}
	scale := 16 + (1-atmosphereClamp(prominence, .35, 1))*11 + unit(29)*4
	swayAngle := angle + math.Pi*(.72+unit(35)*.56)
	swayScale := scale * (.7 + unit(36)*.24)
	return max(math.Abs(math.Cos(angle)*scale), math.Abs(math.Cos(swayAngle)*swayScale)) * .65, max(math.Abs(math.Sin(angle)*scale*.68), math.Abs(math.Sin(swayAngle)*swayScale*.68)) * .65
}
