//! The flavour the config names, and the colour of each hue a context can take.

use groove_gfx::{Color, Palette};
use groove_types::{Hue, ThemeName};

pub(crate) fn flavour(theme: ThemeName) -> Palette {
    match theme {
        ThemeName::Latte => Palette::LATTE,
        ThemeName::Frappe => Palette::FRAPPE,
        ThemeName::Macchiato => Palette::MACCHIATO,
        ThemeName::Mocha => Palette::MOCHA,
    }
}

pub(crate) fn hue(palette: &Palette, hue: Hue) -> Color {
    match hue {
        Hue::Sapphire => palette.sapphire,
        Hue::Mauve => palette.mauve,
        Hue::Pink => palette.pink,
        Hue::Teal => palette.teal,
        Hue::Flamingo => palette.flamingo,
        Hue::Sky => palette.sky,
    }
}
