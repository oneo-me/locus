use gpui::{Hsla, hsla};

#[derive(Clone)]
pub(super) struct NoteAtmosphere {
    pub wash: Hsla,
    pub panel: Hsla,
    pub backdrop: AtmosphereBackdrop,
    pub shapes: Vec<AtmosphereShape>,
}

#[derive(Clone)]
pub(super) enum AtmosphereBackdrop {
    None,
    Grid(Pattern),
    Lines(Pattern),
    Dots {
        spacing: f32,
        offset_x: f32,
        offset_y: f32,
        radius: f32,
        color: Hsla,
    },
    Diagonal(Pattern),
    Diamonds(Pattern),
    Waves {
        pattern: Pattern,
        amplitude: f32,
    },
}

#[derive(Clone)]
pub(super) struct Pattern {
    pub spacing: f32,
    pub rotate: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub stroke_width: f32,
    pub color: Hsla,
}

#[derive(Clone)]
pub(super) enum AtmosphereShape {
    Disk {
        cx: f32,
        cy: f32,
        radius: f32,
        color: Hsla,
    },
    Ring {
        cx: f32,
        cy: f32,
        radius: f32,
        stroke_width: f32,
        color: Hsla,
    },
    RoundedRect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        radius: f32,
        rotate: f32,
        color: Hsla,
    },
    Arc {
        cx: f32,
        cy: f32,
        radius: f32,
        start: f32,
        sweep: f32,
        stroke_width: f32,
        color: Hsla,
    },
    Triangle {
        points: [(f32, f32); 3],
        color: Hsla,
    },
}

#[derive(Clone, Copy)]
struct Slot(f64, f64, f64, f64, f64);

#[derive(Clone, Copy)]
enum Kind {
    Disk,
    Ring,
    RoundedRect,
    Arc,
    Triangle,
}

const LAYOUTS: [[Slot; 6]; 6] = [
    [
        Slot(286., 38., 1., 15., 9.),
        Slot(214., 74., 0.74, 13., 8.),
        Slot(328., 74., 0.55, 9., 7.),
        Slot(243., 19., 0.48, 11., 6.),
        Slot(170., 54., 0.42, 8., 10.),
        Slot(333., 18., 0.38, 8., 5.),
    ],
    [
        Slot(270., 66., 1., 16., 9.),
        Slot(328., 30., 0.72, 10., 8.),
        Slot(205., 27., 0.58, 13., 8.),
        Slot(238., 79., 0.48, 12., 6.),
        Slot(176., 65., 0.4, 9., 8.),
        Slot(315., 82., 0.36, 9., 5.),
    ],
    [
        Slot(305., 50., 1., 15., 10.),
        Slot(229., 24., 0.7, 13., 8.),
        Slot(210., 78., 0.58, 12., 7.),
        Slot(333., 20., 0.48, 9., 6.),
        Slot(272., 82., 0.42, 11., 6.),
        Slot(172., 48., 0.36, 8., 10.),
    ],
    [
        Slot(246., 45., 1., 16., 10.),
        Slot(321., 65., 0.74, 11., 8.),
        Slot(296., 18., 0.58, 12., 6.),
        Slot(189., 74., 0.48, 10., 7.),
        Slot(343., 26., 0.42, 7., 7.),
        Slot(170., 28., 0.36, 8., 8.),
    ],
    [
        Slot(294., 68., 1., 16., 9.),
        Slot(244., 24., 0.74, 13., 8.),
        Slot(333., 45., 0.58, 9., 9.),
        Slot(204., 72., 0.5, 11., 7.),
        Slot(175., 34., 0.42, 8., 8.),
        Slot(276., 84., 0.36, 11., 5.),
    ],
    [
        Slot(263., 31., 1., 16., 9.),
        Slot(320., 75., 0.72, 11., 7.),
        Slot(208., 67., 0.58, 12., 8.),
        Slot(341., 29., 0.48, 8., 7.),
        Slot(176., 52., 0.42, 9., 10.),
        Slot(250., 82., 0.36, 11., 5.),
    ],
];

const FAMILIES: [[Kind; 6]; 10] = [
    [
        Kind::Disk,
        Kind::Ring,
        Kind::Arc,
        Kind::Disk,
        Kind::Ring,
        Kind::Triangle,
    ],
    [
        Kind::RoundedRect,
        Kind::Arc,
        Kind::Disk,
        Kind::Ring,
        Kind::RoundedRect,
        Kind::Triangle,
    ],
    [
        Kind::Triangle,
        Kind::Arc,
        Kind::RoundedRect,
        Kind::Ring,
        Kind::Disk,
        Kind::Arc,
    ],
    [
        Kind::Ring,
        Kind::RoundedRect,
        Kind::Disk,
        Kind::Triangle,
        Kind::Arc,
        Kind::RoundedRect,
    ],
    [
        Kind::Arc,
        Kind::Disk,
        Kind::Triangle,
        Kind::Ring,
        Kind::RoundedRect,
        Kind::Disk,
    ],
    [
        Kind::Disk,
        Kind::Triangle,
        Kind::RoundedRect,
        Kind::Arc,
        Kind::Ring,
        Kind::Triangle,
    ],
    [
        Kind::Ring,
        Kind::Arc,
        Kind::RoundedRect,
        Kind::Disk,
        Kind::Triangle,
        Kind::Ring,
    ],
    [
        Kind::RoundedRect,
        Kind::Ring,
        Kind::Triangle,
        Kind::Disk,
        Kind::Arc,
        Kind::RoundedRect,
    ],
    [
        Kind::Triangle,
        Kind::Disk,
        Kind::Arc,
        Kind::RoundedRect,
        Kind::Ring,
        Kind::Disk,
    ],
    [
        Kind::Arc,
        Kind::RoundedRect,
        Kind::Ring,
        Kind::Triangle,
        Kind::Disk,
        Kind::Arc,
    ],
];

pub(super) fn derive_note_atmosphere(title: &str, dark: bool) -> NoteAtmosphere {
    let seed = hash_value(title);
    let hue = round(190. + unit(mix(seed, 1)) * 60., 1);
    NoteAtmosphere {
        wash: color(
            hue,
            if dark { 36. } else { 40. },
            if dark { 48. } else { 62. },
            if dark { 0.16 } else { 0.12 },
        ),
        panel: color(
            hue,
            if dark { 34. } else { 42. },
            if dark { 28. } else { 90. },
            if dark { 0.42 } else { 0.58 },
        ),
        backdrop: derive_backdrop(seed, hue, dark),
        shapes: derive_shapes(seed, hue, dark),
    }
}

fn derive_backdrop(seed: u32, hue: f64, dark: bool) -> AtmosphereBackdrop {
    let local = mix(seed, 0xbd01);
    let opacity = round(
        (if dark { 0.035 } else { 0.06 }) + unit(mix(local, 2)) * if dark { 0.025 } else { 0.04 },
        3,
    );
    let stroke = tone(hue, dark, false, mix(local, 3) % 16).opacity(opacity as f32);
    let spacing = round(18. + unit(mix(local, 4)) * 16., 2) as f32;
    let pattern = Pattern {
        spacing,
        rotate: round(-8. + unit(mix(local, 5)) * 16., 2) as f32,
        offset_x: round(unit(mix(local, 6)) * spacing as f64, 2) as f32,
        offset_y: round(unit(mix(local, 7)) * spacing as f64, 2) as f32,
        stroke_width: round(0.5 + unit(mix(local, 8)) * 0.45, 2) as f32,
        color: stroke,
    };
    match mix(local, 1) % 7 {
        0 => AtmosphereBackdrop::None,
        1 => AtmosphereBackdrop::Grid(Pattern {
            color: stroke.opacity(0.72),
            ..pattern
        }),
        2 => AtmosphereBackdrop::Lines(pattern),
        3 => AtmosphereBackdrop::Dots {
            spacing,
            offset_x: pattern.offset_x,
            offset_y: pattern.offset_y,
            radius: round(0.55 + unit(mix(local, 9)) * 0.55, 2) as f32,
            color: stroke,
        },
        4 => AtmosphereBackdrop::Diagonal(Pattern {
            spacing: round(spacing as f64 * 1.08, 2) as f32,
            rotate: round(pattern.rotate as f64 + 24., 2) as f32,
            ..pattern
        }),
        5 => AtmosphereBackdrop::Diamonds(Pattern {
            spacing: round(spacing as f64 * 1.16, 2) as f32,
            rotate: round(pattern.rotate as f64 * 0.45, 2) as f32,
            stroke_width: round(pattern.stroke_width as f64 * 0.9, 2) as f32,
            color: stroke.opacity(0.78),
            ..pattern
        }),
        _ => {
            let spacing = round(spacing as f64 * 1.24, 2) as f32;
            AtmosphereBackdrop::Waves {
                pattern: Pattern {
                    spacing,
                    rotate: round(pattern.rotate as f64 * 0.32, 2) as f32,
                    stroke_width: round(pattern.stroke_width as f64 * 0.92, 2) as f32,
                    color: stroke.opacity(0.84),
                    ..pattern
                },
                amplitude: round(
                    (spacing as f64 * (0.11 + unit(mix(local, 10)) * 0.05)).clamp(2.4, 5.8),
                    2,
                ) as f32,
            }
        }
    }
}

fn derive_shapes(seed: u32, hue: f64, dark: bool) -> Vec<AtmosphereShape> {
    let root = mix(seed, 0x5a9e);
    let count = 4 + (mix(root, 1) % 3) as usize;
    let layout = &LAYOUTS[mix(root, 2) as usize % LAYOUTS.len()];
    let family = &FAMILIES[mix(root, 3) as usize % FAMILIES.len()];
    (0..count)
        .map(|index| shape(root, index, layout[index], family[index], hue, dark))
        .collect()
}

fn shape(root: u32, index: usize, slot: Slot, kind: Kind, hue: f64, dark: bool) -> AtmosphereShape {
    let local = mix(root, 20 + index as u32 * 11);
    let prominence = (slot.2 * (0.88 + unit(mix(local, 1)) * 0.24)).clamp(0.34, 1.08);
    let opacity = round(
        (if dark { 0.085 } else { 0.065 })
            + prominence * 0.035
            + unit(mix(local, 2)) * if dark { 0.045 } else { 0.035 },
        3,
    );
    let shape_hue = (hue + [-9., 6., -4., 10., 2., -7.][index]).clamp(190., 250.);
    let fill = tone(shape_hue, dark, true, mix(local, 3) % 20).opacity(opacity as f32);
    let stroke = tone(shape_hue, dark, false, mix(local, 4) % 20);
    let raw_x = slot.0 + (unit(mix(local, 5)) * 2. - 1.) * slot.3;
    let raw_y = slot.1 + (unit(mix(local, 6)) * 2. - 1.) * slot.4;
    let (margin_x, margin_y) = motion_margins(local, kind, prominence);
    let center = |extent: f64| {
        (
            round(
                raw_x.clamp(132. + extent + margin_x, 356. - extent - margin_x),
                2,
            ) as f32,
            round(
                raw_y.clamp(4. + extent + margin_y, 96. - extent - margin_y),
                2,
            ) as f32,
        )
    };
    match kind {
        Kind::Disk => {
            let r = round((17. + unit(mix(local, 7)) * 11.) * prominence, 2);
            let (cx, cy) = center(r);
            AtmosphereShape::Disk {
                cx,
                cy,
                radius: r as f32,
                color: fill,
            }
        }
        Kind::Ring => {
            let r = round((17. + unit(mix(local, 7)) * 10.) * prominence, 2);
            let w = round(1.15 + unit(mix(local, 8)) * 1.45 * prominence, 2);
            let (cx, cy) = center(r + w);
            AtmosphereShape::Ring {
                cx,
                cy,
                radius: r as f32,
                stroke_width: w as f32,
                color: stroke.opacity((opacity + 0.018) as f32),
            }
        }
        Kind::RoundedRect => {
            let h = round((18. + unit(mix(local, 7)) * 8.) * prominence, 2);
            let w = round(h * (1.45 + unit(mix(local, 8)) * 0.55), 2);
            let (cx, cy) = center(w.hypot(h) / 2.);
            AtmosphereShape::RoundedRect {
                x: round(cx as f64 - w / 2., 2) as f32,
                y: round(cy as f64 - h / 2., 2) as f32,
                width: w as f32,
                height: h as f32,
                radius: round(
                    (h * 0.22).min((4. + unit(mix(local, 9)) * 3.) * prominence),
                    2,
                ) as f32,
                rotate: round(-14. + unit(mix(local, 10)) * 28., 2) as f32,
                color: fill,
            }
        }
        Kind::Arc => {
            let r = round((18. + unit(mix(local, 7)) * 10.) * prominence, 2);
            let w = round(1.2 + unit(mix(local, 10)) * 1.8 * prominence, 2);
            let (cx, cy) = center(r + w);
            AtmosphereShape::Arc {
                cx,
                cy,
                radius: r as f32,
                start: round(unit(mix(local, 8)) * 360., 1) as f32,
                sweep: round(105. + unit(mix(local, 9)) * 105., 1) as f32,
                stroke_width: w as f32,
                color: stroke.opacity((opacity + 0.024) as f32),
            }
        }
        Kind::Triangle => {
            let size = round((20. + unit(mix(local, 7)) * 11.) * prominence, 2);
            let (cx, cy) = center(size);
            let angle = unit(mix(local, 8)) * std::f64::consts::TAU;
            let mut points = [(0., 0.); 3];
            for (corner, point) in points.iter_mut().enumerate() {
                let theta = angle + std::f64::consts::TAU * corner as f64 / 3.;
                *point = (
                    round(cx as f64 + theta.cos() * size, 2) as f32,
                    round(cy as f64 + theta.sin() * size, 2) as f32,
                );
            }
            AtmosphereShape::Triangle {
                points,
                color: fill,
            }
        }
    }
}

fn motion_margins(local: u32, kind: Kind, prominence: f64) -> (f64, f64) {
    let angle = unit(mix(local, 34)) * std::f64::consts::TAU;
    if matches!(kind, Kind::Arc) {
        let scale = 8. + unit(mix(local, 36)) * 6.;
        return (
            (angle.cos() * scale).abs() * 0.65,
            (angle.sin() * scale * 0.68).abs() * 0.65,
        );
    }
    let scale = 16. + (1. - prominence.clamp(0.35, 1.)) * 11. + unit(mix(local, 29)) * 4.;
    let sway_angle = angle + std::f64::consts::PI * (0.72 + unit(mix(local, 35)) * 0.56);
    let sway_scale = scale * (0.7 + unit(mix(local, 36)) * 0.24);
    (
        (angle.cos() * scale)
            .abs()
            .max((sway_angle.cos() * sway_scale).abs())
            * 0.65,
        (angle.sin() * scale * 0.68)
            .abs()
            .max((sway_angle.sin() * sway_scale * 0.68).abs())
            * 0.65,
    )
}

fn tone(hue: f64, dark: bool, fill: bool, salt: u32) -> Hsla {
    if fill {
        color(
            hue,
            if dark {
                32. + (salt % 10) as f64
            } else {
                36. + (salt % 10) as f64
            },
            if dark {
                52. + (salt % 10) as f64
            } else {
                56. + (salt % 12) as f64
            },
            1.,
        )
    } else {
        color(
            hue,
            if dark {
                28. + (salt % 8) as f64
            } else {
                32. + (salt % 8) as f64
            },
            if dark {
                68. + (salt % 8) as f64
            } else {
                40. + (salt % 8) as f64
            },
            1.,
        )
    }
}
fn color(h: f64, s: f64, l: f64, a: f64) -> Hsla {
    hsla(
        (h / 360.) as f32,
        (s / 100.) as f32,
        (l / 100.) as f32,
        a as f32,
    )
}
fn hash_value(value: &str) -> u32 {
    value.chars().fold(2_166_136_261, |hash, ch| {
        let scalar = ch as u32;
        let first = if scalar > 0xffff {
            0xd800 + ((scalar - 0x10000) >> 10)
        } else {
            scalar
        };
        (hash ^ first).wrapping_mul(16_777_619)
    })
}
fn mix(hash: u32, salt: u32) -> u32 {
    let mut next = hash ^ salt;
    next = (next ^ (next >> 16)).wrapping_mul(2_246_822_507);
    next = (next ^ (next >> 13)).wrapping_mul(3_266_489_909);
    next ^ (next >> 16)
}
fn unit(value: u32) -> f64 {
    value as f64 / 4_294_967_296.
}
fn round(value: f64, digits: i32) -> f64 {
    let factor = 10_f64.powi(digits);
    (value * factor).round() / factor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atmosphere_is_stable() {
        let first = derive_note_atmosphere("Locus", false);
        let repeated = derive_note_atmosphere("Locus", false);
        assert_eq!(first.wash, repeated.wash);
        assert_eq!(hash_value("Locus"), 528_513_201);
        assert_eq!(
            round(190. + unit(mix(hash_value("Locus"), 1)) * 60., 1),
            241.6
        );
        assert_eq!(first.shapes.len(), 4);
        assert!(matches!(first.backdrop, AtmosphereBackdrop::Waves { .. }));
    }

    #[test]
    fn theme_contrast_keeps_light_details_stronger_than_dark_texture() {
        let light = derive_note_atmosphere("Locus", false);
        let dark = derive_note_atmosphere("Locus", true);
        assert!(light.panel.a > dark.panel.a);
        assert!(backdrop_alpha(&light.backdrop) > backdrop_alpha(&dark.backdrop));
    }

    fn backdrop_alpha(backdrop: &AtmosphereBackdrop) -> f32 {
        match backdrop {
            AtmosphereBackdrop::None => 0.0,
            AtmosphereBackdrop::Grid(pattern)
            | AtmosphereBackdrop::Lines(pattern)
            | AtmosphereBackdrop::Diagonal(pattern)
            | AtmosphereBackdrop::Diamonds(pattern) => pattern.color.a,
            AtmosphereBackdrop::Dots { color, .. } => color.a,
            AtmosphereBackdrop::Waves { pattern, .. } => pattern.color.a,
        }
    }
}
