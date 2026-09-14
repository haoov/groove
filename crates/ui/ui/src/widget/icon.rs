use groove_gfx::{Icon, Rect};

use crate::ctx::Ctx;
use crate::mark::Mark;
use crate::style::Role;
use crate::tokens::SPINNER_MS;

/// An icon box of the token size, centred vertically in `row`, at `x`.
pub fn box_in(ctx: &Ctx, row: Rect, x: f32) -> Rect {
    let size = ctx.tokens.icon;
    Rect::new(x, row.y + (row.h - size) / 2.0, size, size)
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
