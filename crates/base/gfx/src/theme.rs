use crate::Color;

/// Catppuccin, the names it uses.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Palette {
    pub crust: Color,
    pub mantle: Color,
    pub base: Color,
    pub surface0: Color,
    pub surface1: Color,
    pub surface2: Color,
    pub overlay0: Color,
    pub subtext0: Color,
    pub subtext1: Color,
    pub text: Color,
    pub blue: Color,
    pub lavender: Color,
    pub green: Color,
    pub red: Color,
    pub peach: Color,
    pub yellow: Color,
}

impl Palette {
    pub const MOCHA: Palette = Palette {
        crust: Color::hex(0x11111b),
        mantle: Color::hex(0x181825),
        base: Color::hex(0x1e1e2e),
        surface0: Color::hex(0x313244),
        surface1: Color::hex(0x45475a),
        surface2: Color::hex(0x585b70),
        overlay0: Color::hex(0x6c7086),
        subtext0: Color::hex(0xa6adc8),
        subtext1: Color::hex(0xbac2de),
        text: Color::hex(0xcdd6f4),
        blue: Color::hex(0x89b4fa),
        lavender: Color::hex(0xb4befe),
        green: Color::hex(0xa6e3a1),
        red: Color::hex(0xf38ba8),
        peach: Color::hex(0xfab387),
        yellow: Color::hex(0xf9e2af),
    };

    pub const FRAPPE: Palette = Palette {
        crust: Color::hex(0x232634),
        mantle: Color::hex(0x292c3c),
        base: Color::hex(0x303446),
        surface0: Color::hex(0x414559),
        surface1: Color::hex(0x51576d),
        surface2: Color::hex(0x626880),
        overlay0: Color::hex(0x737994),
        subtext0: Color::hex(0xa5adce),
        subtext1: Color::hex(0xb5bfe2),
        text: Color::hex(0xc6d0f5),
        blue: Color::hex(0x8caaee),
        lavender: Color::hex(0xbabbf1),
        green: Color::hex(0xa6d189),
        red: Color::hex(0xe78284),
        peach: Color::hex(0xef9f76),
        yellow: Color::hex(0xe5c890),
    };

    pub const MACCHIATO: Palette = Palette {
        crust: Color::hex(0x181926),
        mantle: Color::hex(0x1e2030),
        base: Color::hex(0x24273a),
        surface0: Color::hex(0x363a4f),
        surface1: Color::hex(0x494d64),
        surface2: Color::hex(0x5b6078),
        overlay0: Color::hex(0x6e738d),
        subtext0: Color::hex(0xa5adcb),
        subtext1: Color::hex(0xb8c0e0),
        text: Color::hex(0xcad3f5),
        blue: Color::hex(0x8aadf4),
        lavender: Color::hex(0xb7bdf8),
        green: Color::hex(0xa6da95),
        red: Color::hex(0xed8796),
        peach: Color::hex(0xf5a97f),
        yellow: Color::hex(0xeed49f),
    };

    pub const LATTE: Palette = Palette {
        crust: Color::hex(0xdce0e8),
        mantle: Color::hex(0xe6e9ef),
        base: Color::hex(0xeff1f5),
        surface0: Color::hex(0xccd0da),
        surface1: Color::hex(0xbcc0cc),
        surface2: Color::hex(0xacb0be),
        overlay0: Color::hex(0x9ca0b0),
        subtext0: Color::hex(0x6c6f85),
        subtext1: Color::hex(0x5c5f77),
        text: Color::hex(0x4c4f69),
        blue: Color::hex(0x1e66f5),
        lavender: Color::hex(0x7287fd),
        green: Color::hex(0x40a02b),
        red: Color::hex(0xd20f39),
        peach: Color::hex(0xfe640b),
        yellow: Color::hex(0xdf8e1d),
    };
}

/// The palette and the type scale, in pixels.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Theme {
    pub palette: Palette,
    pub text: f32,
    pub small: f32,
    pub title: f32,
    pub mono: f32,
}

impl Theme {
    pub const fn dark() -> Self {
        Self::with(Palette::MOCHA)
    }

    pub const fn light() -> Self {
        Self::with(Palette::LATTE)
    }

    pub const fn frappe() -> Self {
        Self::with(Palette::FRAPPE)
    }

    pub const fn macchiato() -> Self {
        Self::with(Palette::MACCHIATO)
    }

    const fn with(palette: Palette) -> Self {
        Self {
            palette,
            text: 12.0,
            small: 11.0,
            title: 14.0,
            mono: 12.5,
        }
    }
}
