use groove_gfx::{Color, Frame, Rect, TextStyle};

/// One line of text, vertically centred in `rect`, clipped to it.
pub fn row(frame: &mut Frame, rect: Rect, pad: f32, text: &str, style: TextStyle) {
    frame.clipped(rect, |f| f.text(text, rect.x + pad, rect.y, rect.h, style));
}

/// A one-pixel line along the bottom of `rect`.
pub fn hairline(frame: &mut Frame, rect: Rect, color: Color) {
    frame.quad(Rect::new(rect.x, rect.bottom() - 1.0, rect.w, 1.0), color);
}
