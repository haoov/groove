/// An sRGB colour with straight alpha.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const TRANSPARENT: Color = Color::rgba(0, 0, 0, 0);
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const BLACK: Color = Color::rgb(0, 0, 0);

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// `t` of the way from this colour to `other`, opaque.
    pub fn mix(self, other: Color, t: f32) -> Color {
        let at = |a: u8, b: u8| {
            let (a, b) = (f32::from(a), f32::from(b));
            (a + (b - a) * t.clamp(0.0, 1.0)).round() as u8
        };
        Color::rgb(
            at(self.r, other.r),
            at(self.g, other.g),
            at(self.b, other.b),
        )
    }

    /// `0xRRGGBB`.
    pub const fn hex(rgb: u32) -> Self {
        Self::rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
    }

    pub const fn with_alpha(self, a: u8) -> Self {
        Self { a, ..self }
    }

    pub const fn is_transparent(self) -> bool {
        self.a == 0
    }

    /// Linear RGB with alpha in `0.0..=1.0`, the form the sRGB render target expects.
    pub(crate) fn linear(self) -> [f32; 4] {
        [
            channel_to_linear(self.r),
            channel_to_linear(self.g),
            channel_to_linear(self.b),
            f32::from(self.a) / 255.0,
        ]
    }

    pub(crate) fn glyphon(self) -> glyphon::Color {
        glyphon::Color::rgba(self.r, self.g, self.b, self.a)
    }
}

fn channel_to_linear(c: u8) -> f32 {
    let s = f32::from(c) / 255.0;
    if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}
