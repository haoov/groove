use groove_gfx::{Color, Rect, TextStyle};

use crate::ctx::Ctx;

/// One line of text in `rect`, `indent` from its left edge, clipped to it.
pub fn row(ctx: &mut Ctx, rect: Rect, indent: f32, text: &str, style: TextStyle) {
    ctx.clipped(rect, |ctx| {
        ctx.text(text, rect.x + indent, rect.y, rect.h, style);
    });
}

/// A hairline along the bottom of `rect`.
pub fn hairline(ctx: &mut Ctx, rect: Rect, color: Color) {
    let thickness = ctx.tokens.hairline;
    ctx.quad(
        Rect::new(rect.x, rect.bottom() - thickness, rect.w, thickness),
        color,
    );
}
