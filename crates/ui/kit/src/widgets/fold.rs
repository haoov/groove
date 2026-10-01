use groove_gfx::Rect;

use crate::base::ctx::{App, Ctx};
use crate::base::mark::Mark;
use crate::base::style::Role;
use crate::shape::square;

/// A directory's folder, open or shut, from the left of `room`.
pub fn folder<A: App>(ctx: &mut Ctx<'_, A>, room: &mut Rect, open: bool, role: Role) -> Rect {
    let size = ctx.tokens.icon;
    let box_ = square(room.take_left(size), size);
    room.take_left(ctx.tokens.xs);
    let mark = if open { Mark::FolderOpen } else { Mark::Folder };
    ctx.icon(box_, mark, 0, ctx.styles.color(role));
    box_
}

/// The caret of what folds: down while open, right while shut, from the left of `room`.
pub fn fold<A: App>(ctx: &mut Ctx<'_, A>, room: &mut Rect, open: bool, role: Role) -> Rect {
    let size = ctx.tokens.small;
    let box_ = square(room.take_left(size), size);
    room.take_left(ctx.tokens.xs);
    let turn = if open { 0 } else { Mark::RIGHTWARDS };
    ctx.icon(box_, Mark::Down, turn, ctx.styles.color(role));
    box_
}
