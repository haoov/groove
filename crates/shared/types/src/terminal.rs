/// A terminal colour, resolved.
#[derive(
    Clone, Copy, PartialEq, Eq, Hash, Debug, Default, serde::Serialize, serde::Deserialize,
)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn hex(rgb: u32) -> Self {
        Self {
            r: (rgb >> 16) as u8,
            g: (rgb >> 8) as u8,
            b: rgb as u8,
        }
    }

    /// Two thirds of the light: the `dim` attribute.
    pub const fn dimmed(self) -> Self {
        Self {
            r: (self.r as u16 * 2 / 3) as u8,
            g: (self.g as u16 * 2 / 3) as u8,
            b: (self.b as u16 * 2 / 3) as u8,
        }
    }
}

/// One cell of a terminal screen. `bg` is `None` on the default background.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ScreenCell {
    pub ch: char,
    pub fg: Rgb,
    pub bg: Option<Rgb>,
    pub bold: bool,
    /// The second half of a wide character.
    pub spacer: bool,
}

/// The visible grid of a terminal, cursor in `(col, row)`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Screen {
    pub cols: usize,
    pub rows: usize,
    pub cells: Vec<ScreenCell>,
    pub cursor: Option<(usize, usize)>,
}

impl Screen {
    pub fn cell(&self, col: usize, row: usize) -> &ScreenCell {
        &self.cells[row * self.cols + col]
    }

    /// One row's characters, trailing blanks trimmed.
    pub fn line(&self, row: usize) -> String {
        let text: String = (0..self.cols)
            .map(|c| self.cell(c, row))
            .filter(|c| !c.spacer)
            .map(|c| c.ch)
            .collect();
        text.trim_end().to_string()
    }
}

/// The sixteen ANSI colours and the three defaults.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AnsiPalette {
    pub colors: [Rgb; 16],
    pub foreground: Rgb,
    pub background: Rgb,
    pub cursor: Rgb,
}

impl AnsiPalette {
    pub const MOCHA: AnsiPalette = AnsiPalette {
        colors: [
            Rgb::hex(0x45475a),
            Rgb::hex(0xf38ba8),
            Rgb::hex(0xa6e3a1),
            Rgb::hex(0xf9e2af),
            Rgb::hex(0x89b4fa),
            Rgb::hex(0xf5c2e7),
            Rgb::hex(0x94e2d5),
            Rgb::hex(0xbac2de),
            Rgb::hex(0x585b70),
            Rgb::hex(0xf38ba8),
            Rgb::hex(0xa6e3a1),
            Rgb::hex(0xf9e2af),
            Rgb::hex(0x89b4fa),
            Rgb::hex(0xf5c2e7),
            Rgb::hex(0x94e2d5),
            Rgb::hex(0xa6adc8),
        ],
        foreground: Rgb::hex(0xcdd6f4),
        background: Rgb::hex(0x1e1e2e),
        cursor: Rgb::hex(0xf5e0dc),
    };

    pub const LATTE: AnsiPalette = AnsiPalette {
        colors: [
            Rgb::hex(0x5c5f77),
            Rgb::hex(0xd20f39),
            Rgb::hex(0x40a02b),
            Rgb::hex(0xdf8e1d),
            Rgb::hex(0x1e66f5),
            Rgb::hex(0xea76cb),
            Rgb::hex(0x179299),
            Rgb::hex(0xacb0be),
            Rgb::hex(0x6c6f85),
            Rgb::hex(0xd20f39),
            Rgb::hex(0x40a02b),
            Rgb::hex(0xdf8e1d),
            Rgb::hex(0x1e66f5),
            Rgb::hex(0xea76cb),
            Rgb::hex(0x179299),
            Rgb::hex(0xbcc0cc),
        ],
        foreground: Rgb::hex(0x4c4f69),
        background: Rgb::hex(0xeff1f5),
        cursor: Rgb::hex(0xdc8a78),
    };
}
