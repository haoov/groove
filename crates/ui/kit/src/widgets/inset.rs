//! A row inset from its list's sides, on a rounded ground, as the rail draws its sessions.

use groove_gfx::{Color, Edges, Rect};

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

/// A list row's rounded ground on glass, inset as the rail's rows are.
fn glass_row<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, color: Color) {
    let inset = rect.pad(Edges::xy(ctx.tokens.sm, ctx.tokens.xs / 2.0));
    ctx.rounded(inset, color, ctx.tokens.sm);
}

/// Registers `target` at `rect`, lit on a rounded ground while the pointer rests on it.
pub fn glass_hoverable<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, target: A::Target) -> bool {
    let on = ctx.interact(rect, target);
    if on {
        glass_lit(ctx, rect);
    }
    on
}

/// A list row lit as the pointer lights it.
pub fn glass_lit<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect) {
    glass_row(ctx, rect, ctx.styles.glass_hover());
}

/// The row a list stands on, held on a rounded ground.
pub fn glass_chosen<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect) {
    glass_row(ctx, rect, ctx.styles.glass_chosen());
}
