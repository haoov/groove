//! Placement and rules: where a box sits in a row, and the lines drawn around one.

use groove_gfx::{Color, Rect};

use crate::base::ctx::Ctx;

/// A square box of `size`, centred vertically in `row`, at `x`.
pub fn box_in(row: Rect, x: f32, size: f32) -> Rect {
    Rect::new(x, row.y + (row.h - size) / 2.0, size, size)
}

/// A leading icon in `row` at `x`, the icon size.
pub fn leading(ctx: &Ctx, row: Rect, x: f32) -> Rect {
    box_in(row, x, ctx.tokens.icon)
}

/// Where a row's text starts when a mark leads it at `indent`.
pub fn after_mark(ctx: &Ctx, indent: f32) -> f32 {
    indent + ctx.tokens.icon + ctx.tokens.sm
}

/// A hairline above `rect` and one below it.
pub fn ruled(ctx: &mut Ctx, rect: Rect, color: Color) {
    let thickness = ctx.tokens.hairline;
    ctx.quad(Rect::new(rect.x, rect.y, rect.w, thickness), color);
    let under = rect.bottom() - thickness;
    ctx.quad(Rect::new(rect.x, under, rect.w, thickness), color);
}

/// A hairline along the bottom of `rect`.
pub fn hairline(ctx: &mut Ctx, rect: Rect, color: Color) {
    let thickness = ctx.tokens.hairline;
    ctx.quad(
        Rect::new(rect.x, rect.bottom() - thickness, rect.w, thickness),
        color,
    );
}
