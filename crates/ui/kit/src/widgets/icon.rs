use groove_gfx::Rect;

use crate::base::ctx::{App, Ctx};
use crate::base::mark::Mark;
use crate::base::style::Role;

pub fn icon<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, mark: Mark, role: Role) {
    turned(ctx, rect, mark, 0, role);
}

/// A mark turned by eighths of a turn.
pub fn turned<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, mark: Mark, turn: u8, role: Role) {
    let color = ctx.styles.color(role);
    ctx.icon(rect, mark, turn, color);
}

pub fn busy<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, role: Role) {
    let turn = crate::base::motion::turn(ctx.tick);
    turned(ctx, rect, Mark::Busy, turn, role);
}

/// A mark at the left of `room`, which gives up the mark and the gap after it; returns its box.
pub fn lead<A: App>(ctx: &mut Ctx<'_, A>, room: &mut Rect, mark: Mark, role: Role) -> Rect {
    let size = ctx.tokens.icon;
    let at = crate::shape::square(room.take_left(size), size);
    room.take_left(ctx.tokens.sm);
    icon(ctx, at, mark, role);
    at
}
