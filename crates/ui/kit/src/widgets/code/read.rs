//! The box that says whether a file is read.

use groove_gfx::{Edges, Rect};

use crate::base::ctx::{App, Ctx};
use crate::base::mark::Mark;
use crate::base::style::Role;

/// The box that says whether a file is read, ticked once it is.
pub fn read_box<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, read: bool) {
    let square = rect.pad(Edges::all(ctx.tokens.xs / 2.0));
    let role = match read {
        true => Role::Text,
        false => Role::Faint,
    };
    ctx.border(square, ctx.styles.color(role));
    if read {
        ctx.icon(rect, Mark::Read, 0, ctx.styles.color(role));
    }
}
