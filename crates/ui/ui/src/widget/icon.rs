use groove_gfx::{Icon, Rect};

use crate::ctx::Ctx;
use crate::mark::Mark;
use crate::style::Role;
use crate::tokens::SPINNER_MS;

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

/// An icon in `rect`, upright, in the role's colour.
pub fn icon(ctx: &mut Ctx, rect: Rect, mark: Mark, role: Role) {
    let color = ctx.styles.color(role);
    ctx.icon(rect, mark, 0, color);
}

/// Which eighth of a turn a busy mark is at.
pub fn turn(tick: u64) -> u8 {
    (tick / SPINNER_MS % u64::from(Icon::TURNS)) as u8
}
