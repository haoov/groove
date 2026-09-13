use alacritty_terminal::term::color::Colors;
use alacritty_terminal::vte::ansi::{Color, NamedColor, Rgb as VteRgb};
use groove_types::{AnsiPalette, Rgb};

/// A cell colour to RGB: the program's own overrides first, then the palette.
pub(crate) fn resolve(color: Color, overrides: &Colors, palette: &AnsiPalette) -> Rgb {
    match color {
        Color::Spec(rgb) => from_vte(rgb),
        Color::Indexed(i) => overrides[usize::from(i)]
            .map(from_vte)
            .unwrap_or_else(|| indexed(usize::from(i), palette)),
        Color::Named(name) => overrides[name as usize]
            .map(from_vte)
            .unwrap_or_else(|| named(name, palette)),
    }
}

/// The 256-colour table: the palette, the 6x6x6 cube, the 24 greys.
pub(crate) fn indexed(index: usize, palette: &AnsiPalette) -> Rgb {
    match index {
        0..=15 => palette.colors[index],
        16..=231 => {
            let i = index - 16;
            let level = |n: usize| if n == 0 { 0 } else { (55 + 40 * n) as u8 };
            Rgb {
                r: level(i / 36),
                g: level(i / 6 % 6),
                b: level(i % 6),
            }
        }
        232..=255 => {
            let grey = (8 + 10 * (index - 232)) as u8;
            Rgb {
                r: grey,
                g: grey,
                b: grey,
            }
        }
        _ => named_index(index, palette),
    }
}

fn named(name: NamedColor, palette: &AnsiPalette) -> Rgb {
    named_index(name as usize, palette)
}

fn named_index(index: usize, palette: &AnsiPalette) -> Rgb {
    const DIM_BLACK: usize = NamedColor::DimBlack as usize;
    match index {
        0..=15 => palette.colors[index],
        i if i == NamedColor::Foreground as usize => palette.foreground,
        i if i == NamedColor::BrightForeground as usize => palette.foreground,
        i if i == NamedColor::DimForeground as usize => palette.foreground.dimmed(),
        i if i == NamedColor::Background as usize => palette.background,
        i if i == NamedColor::Cursor as usize => palette.cursor,
        i if (DIM_BLACK..DIM_BLACK + 8).contains(&i) => palette.colors[i - DIM_BLACK].dimmed(),
        _ => palette.foreground,
    }
}

pub(crate) fn from_vte(rgb: VteRgb) -> Rgb {
    Rgb {
        r: rgb.r,
        g: rgb.g,
        b: rgb.b,
    }
}

pub(crate) fn to_vte(rgb: Rgb) -> VteRgb {
    VteRgb {
        r: rgb.r,
        g: rgb.g,
        b: rgb.b,
    }
}
