package main

import (
	"math"

	"github.com/egoist/mygo/ui"
)

// A narrow drawing boundary also lets tests verify submission size without
// opening a native window or inspecting MyGo's private path representation.
type atmospherePainter interface {
	Fill(ui.Rect, ui.Color, float32)
	Stroke(ui.Rect, ui.Color, float32, float32)
	FillGradient(ui.Rect, ui.LinearGradient, float32)
	FillPath(*ui.Path, ui.Color)
	StrokePath(*ui.Path, float32, ui.Color)
}

func atmosphereVerticalFade(y float32) float32 {
	if y <= atmosphereHeight*.7 {
		return 1
	}
	return max(0, min(1, 1-(y-atmosphereHeight*.7)/(atmosphereHeight*.3)))
}
func atmosphereShapeFade(y float32) float32 {
	if y <= 75 {
		return 1
	}
	return max(0, min(1, 1-(y-75)/25))
}
func atmosphereHorizontalFade(x float32) float32 {
	if x <= 122.4 {
		return 0
	}
	if x >= 187.2 {
		return 1
	}
	return (x - 122.4) / 64.8
}
func atmosphereRotate(v, center atmospherePoint, degrees float32) atmospherePoint {
	angle := float64(degrees * float32(math.Pi/180))
	cos, sin := float32(math.Cos(angle)), float32(math.Sin(angle))
	x, y := v.X-center.X, v.Y-center.Y
	return atmospherePoint{center.X + x*cos - y*sin, center.Y + x*sin + y*cos}
}
func atmosphereGeometry(width float32) (origin, scale float32) {
	scale = min(max(0, width-48), noteMaxWidth) / 360
	scale = min(scale, atmosphereShapeHeight/100)
	return (width - 360*scale) / 2, scale
}

// The backdrop always starts at the article origin, not at the panel's inset,
// and is 268 DIPs tall even for a short article. Only shapes use the 360x100
// design coordinate system; texture spacing stays in physical layout DIPs.
func (a noteAtmosphere) draw(p atmospherePainter, r ui.Rect) {
	if r.W <= 0 {
		return
	}
	r.H = atmosphereHeight
	p.FillGradient(r, ui.LinearGradient{From: a.wash.ui(), To: a.wash.opacity(0).ui(), Angle: 180}, 0)
	a.backdrop.draw(p, r)
	origin, scale := atmosphereGeometry(r.W)
	for _, s := range a.shapes {
		s.draw(p, r.X+origin, r.Y, scale)
	}
}
func (b atmosphereBackdrop) draw(p atmospherePainter, r ui.Rect) {
	if b.Kind == backdropNone {
		return
	}
	spacing := b.Spacing
	if b.Kind == backdropDots {
		for y := r.Y - spacing + b.OffsetY; y < r.Y+r.H; y += spacing {
			for x := r.X - spacing + b.OffsetX; x < r.X+r.W; x += spacing {
				var path ui.Path
				path.Circle(x+b.Radius, y+b.Radius, b.Radius)
				p.FillPath(&path, b.Color.opacity(atmosphereVerticalFade(y-r.Y)).ui())
			}
		}
		return
	}
	center := atmospherePoint{r.X + r.W/2, r.Y + r.H/2}
	rotate := func(x, y float32) atmospherePoint { return atmosphereRotate(atmospherePoint{x, y}, center, b.Rotate) }
	for y := r.Y - spacing*3 + b.OffsetY; y < r.Y+r.H+spacing*3; y += spacing {
		fade := atmosphereVerticalFade(y - r.Y)
		if fade == 0 {
			continue
		}
		// Keep each mask cell-sized. A rotated viewport-wide row has a huge
		// bounding box and can evict glyph masks while the frame is painting.
		for x := r.X - spacing*3 + b.OffsetX; x < r.X+r.W+spacing*3; x += spacing {
			var path ui.Path
			if b.Kind == backdropWaves {
				v := [5]atmospherePoint{rotate(x, y+spacing/2), rotate(x+spacing/4, y+spacing/2-b.Amplitude), rotate(x+spacing/2, y+spacing/2), rotate(x+spacing*3/4, y+spacing/2+b.Amplitude), rotate(x+spacing, y+spacing/2)}
				path.MoveTo(v[0].X, v[0].Y).QuadTo(v[1].X, v[1].Y, v[2].X, v[2].Y).QuadTo(v[3].X, v[3].Y, v[4].X, v[4].Y)
				p.StrokePath(&path, b.StrokeWidth, b.Color.opacity(fade).ui())
				continue
			}
			var points []atmospherePoint
			switch b.Kind {
			case backdropGrid:
				points = []atmospherePoint{{x, y + spacing}, {x, y}, {x + spacing, y}}
			case backdropLines:
				points = []atmospherePoint{{x, y}, {x + spacing, y}}
			case backdropDiagonal:
				points = []atmospherePoint{{x, y}, {x + spacing, y + spacing}}
			case backdropDiamonds:
				points = []atmospherePoint{{x, y + spacing/2}, {x + spacing/2, y}, {x + spacing, y + spacing/2}, {x + spacing/2, y + spacing}, {x, y + spacing/2}}
			}
			for i, v := range points {
				v = rotate(v.X, v.Y)
				if i == 0 {
					path.MoveTo(v.X, v.Y)
				} else {
					path.LineTo(v.X, v.Y)
				}
			}
			p.StrokePath(&path, b.StrokeWidth, b.Color.opacity(fade).ui())
		}
	}
}
func (s atmosphereShape) center() atmospherePoint {
	switch s.Kind {
	case atmosphereRect:
		return atmospherePoint{s.X + s.Width/2, s.Y + s.Height/2}
	case atmosphereTriangle:
		return atmospherePoint{(s.Points[0].X + s.Points[1].X + s.Points[2].X) / 3, (s.Points[0].Y + s.Points[1].Y + s.Points[2].Y) / 3}
	default:
		return atmospherePoint{s.X, s.Y}
	}
}
func (s atmosphereShape) draw(p atmospherePainter, ox, oy, scale float32) {
	center := s.center()
	color := s.Color.opacity(atmosphereShapeFade(center.Y) * atmosphereHorizontalFade(center.X)).ui()
	mapped := func(x, y float32) atmospherePoint { return atmospherePoint{ox + x*scale, oy + y*scale} }
	switch s.Kind {
	case atmosphereDisk, atmosphereRing:
		v := mapped(s.X-s.Radius, s.Y-s.Radius)
		radius := s.Radius * scale
		box := ui.Rect{X: v.X, Y: v.Y, W: radius * 2, H: radius * 2}
		if s.Kind == atmosphereDisk {
			p.Fill(box, color, radius)
		} else {
			p.Stroke(box, color, radius, s.StrokeWidth*scale)
		}
	case atmosphereRect:
		v := mapped(s.X, s.Y)
		center := atmospherePoint{v.X + s.Width*scale/2, v.Y + s.Height*scale/2}
		corners := [4]atmospherePoint{v, {v.X + s.Width*scale, v.Y}, {v.X + s.Width*scale, v.Y + s.Height*scale}, {v.X, v.Y + s.Height*scale}}
		var path ui.Path
		for i, v := range corners {
			v = atmosphereRotate(v, center, s.Rotate)
			if i == 0 {
				path.MoveTo(v.X, v.Y)
			} else {
				path.LineTo(v.X, v.Y)
			}
		}
		// The reference deliberately ignores its generated radius and fills a
		// rotated four-corner polygon. Rounding it would not match the old renderer.
		path.Close()
		p.FillPath(&path, color)
	case atmosphereArc:
		center := mapped(s.X, s.Y)
		path := atmosphereArcPath(center, s.Radius*scale, s.Start, s.Sweep)
		p.StrokePath(&path, s.StrokeWidth*scale, color)
	case atmosphereTriangle:
		var path ui.Path
		for i, v := range s.Points {
			v = mapped(v.X, v.Y)
			if i == 0 {
				path.MoveTo(v.X, v.Y)
			} else {
				path.LineTo(v.X, v.Y)
			}
		}
		path.Close()
		p.FillPath(&path, color)
	}
}

// MyGo has no SVG arc command. Split the same circular arc into <=90 degree
// cubic Beziers, preserving its center, sweep, endpoints, and open stroke.
func atmosphereArcPath(center atmospherePoint, radius, start, sweep float32) ui.Path {
	var path ui.Path
	angle := float64(start) * math.Pi / 180
	count := int(math.Ceil(math.Abs(float64(sweep)) / 90))
	if count == 0 {
		return path
	}
	step := float64(sweep) * math.Pi / 180 / float64(count)
	point := func(x, y float64) (float32, float32) {
		return center.X + radius*float32(x), center.Y + radius*float32(y)
	}
	x, y := point(math.Cos(angle), math.Sin(angle))
	path.MoveTo(x, y)
	for i := 0; i < count; i++ {
		next := angle + step
		k := 4.0 / 3 * math.Tan(step/4)
		c1x, c1y := point(math.Cos(angle)-k*math.Sin(angle), math.Sin(angle)+k*math.Cos(angle))
		c2x, c2y := point(math.Cos(next)+k*math.Sin(next), math.Sin(next)-k*math.Cos(next))
		x, y = point(math.Cos(next), math.Sin(next))
		path.CubeTo(c1x, c1y, c2x, c2y, x, y)
		angle = next
	}
	return path
}
