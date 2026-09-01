use super::*;

const MINIMUM_TEXT_CONTRAST: f32 = 4.5;

pub(super) fn resolve_color(
    color: Color,
    colors: &alacritty_terminal::term::color::Colors,
    palette: TerminalPalette,
    foreground: bool,
) -> Hsla {
    let rgb = match color {
        Color::Spec(color) => Some(color),
        Color::Indexed(index) => {
            colors[index as usize].or_else(|| Some(terminal_rgb(index as usize, palette.dark)))
        }
        Color::Named(NamedColor::Foreground | NamedColor::BrightForeground) => None,
        Color::Named(NamedColor::Background) => return palette.background,
        Color::Named(NamedColor::Cursor) => return palette.foreground,
        Color::Named(named) => {
            colors[named as usize].or_else(|| Some(terminal_rgb(named as usize, palette.dark)))
        }
    };
    rgb.map(rgb_to_hsla).unwrap_or(if foreground {
        palette.foreground
    } else {
        palette.background
    })
}

pub(super) fn rgb_to_hsla(color: Rgb) -> Hsla {
    rgb((u32::from(color.r) << 16) | (u32::from(color.g) << 8) | u32::from(color.b)).into()
}

pub(super) fn readable_foreground(
    foreground: Hsla,
    background: Hsla,
    palette_foreground: Hsla,
) -> Hsla {
    if contrast_ratio(foreground, background) >= MINIMUM_TEXT_CONTRAST {
        return foreground;
    }

    let black = rgb(0x000000).into();
    let white = rgb(0xffffff).into();
    let target = [palette_foreground, black, white]
        .into_iter()
        .max_by(|left, right| {
            contrast_ratio(*left, background).total_cmp(&contrast_ratio(*right, background))
        })
        .expect("contrast targets are non-empty");
    if contrast_ratio(target, background) <= contrast_ratio(foreground, background) {
        return foreground;
    }

    let foreground_rgb = foreground.to_rgb();
    let target_rgb = target.to_rgb();
    let mut low = 0.0;
    let mut high = 1.0;
    for _ in 0..12 {
        let amount = (low + high) / 2.0;
        let candidate = mix_rgb(foreground_rgb, target_rgb, amount);
        if contrast_ratio(candidate.into(), background) >= MINIMUM_TEXT_CONTRAST {
            high = amount;
        } else {
            low = amount;
        }
    }
    mix_rgb(foreground_rgb, target_rgb, high).into()
}

fn mix_rgb(from: Rgba, to: Rgba, amount: f32) -> Rgba {
    Rgba {
        r: from.r + (to.r - from.r) * amount,
        g: from.g + (to.g - from.g) * amount,
        b: from.b + (to.b - from.b) * amount,
        a: from.a,
    }
}

fn contrast_ratio(foreground: Hsla, background: Hsla) -> f32 {
    let foreground = relative_luminance(foreground.to_rgb());
    let background = relative_luminance(background.to_rgb());
    (foreground.max(background) + 0.05) / (foreground.min(background) + 0.05)
}

fn relative_luminance(color: Rgba) -> f32 {
    let linear = |channel: f32| {
        if channel <= 0.04045 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
}

pub(super) fn terminal_rgb(index: usize, dark: bool) -> Rgb {
    const DARK: [u32; 16] = [
        0x1d1f21, 0xcc6666, 0xb5bd68, 0xf0c674, 0x81a2be, 0xb294bb, 0x8abeb7, 0xc5c8c6, 0x666666,
        0xd54e53, 0xb9ca4a, 0xe7c547, 0x7aa6da, 0xc397d8, 0x70c0b1, 0xeaeaea,
    ];
    const LIGHT: [u32; 16] = [
        0x000000, 0xc82829, 0x718c00, 0xeab700, 0x4271ae, 0x8959a8, 0x3e999f, 0xc7c7c7, 0x8e908c,
        0xc82829, 0x718c00, 0xeab700, 0x4271ae, 0x8959a8, 0x3e999f, 0xffffff,
    ];
    let ansi = if dark { &DARK } else { &LIGHT };
    let value = match index {
        0..=15 => ansi[index],
        16..=231 => {
            let index = index - 16;
            let channel = |value: usize| {
                if value == 0 {
                    0
                } else {
                    55 + value as u32 * 40
                }
            };
            (channel(index / 36) << 16) | (channel((index / 6) % 6) << 8) | channel(index % 6)
        }
        232..=255 => {
            let value = 8 + (index as u32 - 232) * 10;
            (value << 16) | (value << 8) | value
        }
        256 | 258 | 267 => terminal_foreground(dark),
        257 | 268 => terminal_background(dark),
        _ => terminal_foreground(dark),
    };
    Rgb {
        r: (value >> 16) as u8,
        g: (value >> 8) as u8,
        b: value as u8,
    }
}

fn terminal_foreground(dark: bool) -> u32 {
    if dark { 0xe5e5e5 } else { 0x242424 }
}

fn terminal_background(dark: bool) -> u32 {
    if dark { 0x171717 } else { 0xfafafa }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_contrast_foregrounds_are_made_readable_on_light_and_dark_backgrounds() {
        let light_background = Hsla::from(rgb(0xfafafa));
        let dark_background = Hsla::from(rgb(0x171717));

        let light_result =
            readable_foreground(rgb(0xe5e5e5).into(), light_background, rgb(0x242424).into());
        let dark_result =
            readable_foreground(rgb(0x303030).into(), dark_background, rgb(0xe5e5e5).into());

        assert!(contrast_ratio(light_result, light_background) >= MINIMUM_TEXT_CONTRAST);
        assert!(contrast_ratio(dark_result, dark_background) >= MINIMUM_TEXT_CONTRAST);
    }

    #[test]
    fn readable_foregrounds_keep_their_application_color() {
        let foreground: Hsla = rgb(0xc82829).into();
        let background = Hsla::from(rgb(0xfafafa));

        assert_eq!(
            readable_foreground(foreground, background, rgb(0x242424).into()),
            foreground
        );
    }
}
