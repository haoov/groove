//! A row inset from its list's sides, on a rounded ground, as the rail draws its sessions.

use groove_gfx::{Edges, Rect};

use crate::base::ctx::{App, Ctx};
use crate::base::ground::Ground;

/// `rect` inset, on `tint` when given, else see-through: stronger when chosen, lighter under
/// the pointer. A chosen row on a tint is edged. Returns the inset row.
pub fn inset_row<A: App>(
    ctx: &mut Ctx<'_, A>,
    rect: Rect,
    target: A::Target,
    (chosen, tint): (bool, Option<Ground>),
) -> Rect {
    let row = rect.pad(Edges::xy(ctx.tokens.sm, ctx.tokens.xs / 2.0));
    let on = ctx.interact(row, target);
    let ground = match (tint, chosen, on) {
        (Some(tint), _, _) => Some(tint.color(&ctx.styles)),
        (None, true, _) => Some(ctx.styles.glass_chosen()),
        (None, false, true) => Some(ctx.styles.glass_hover()),
        (None, false, false) => None,
    };
    let round = ctx.tokens.sm;
    if let Some(color) = ground {
        ctx.rounded(row, color, round);
    }
    if chosen && tint.is_some() {
        let edge = ctx.styles.glass_chosen();
        ctx.ring(row, edge, round, ctx.tokens.hairline);
    }
    row
}
