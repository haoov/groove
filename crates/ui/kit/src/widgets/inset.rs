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

/// A row-high field on a rounded glass ground, centred in `rect`; returns the field.
pub fn glass_field<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect) -> Rect {
    let down = ((rect.h - ctx.tokens.row) / 2.0).max(0.0);
    let field = rect.pad(Edges::xy(ctx.tokens.sm, down));
    ctx.rounded(field, ctx.styles.glass_hover(), ctx.tokens.sm);
    field
}

/// The room inside a card above its content and below it.
pub fn card_inset(tokens: &crate::base::tokens::Tokens) -> f32 {
    tokens.sm
}

/// A list row lit as the pointer lights it.
pub fn glass_lit<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect) {
    glass_row(ctx, rect, ctx.styles.glass_hover());
}

/// A list row on a rounded ground of `color`, as a row in a role's tint stands.
pub fn glass_tinted<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, color: Color) {
    glass_row(ctx, rect, color);
}

/// The row a list stands on, held on a rounded ground.
pub fn glass_chosen<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect) {
    glass_row(ctx, rect, ctx.styles.glass_chosen());
}

/// One row's slice of a rounded box drawn row by row: rounded only where the box opens or closes.
pub fn rounded_slice<A: App>(
    ctx: &mut Ctx<'_, A>,
    rect: Rect,
    (top, bottom): (bool, bool),
    (ground, edge): (Color, Option<Color>),
) {
    let round = ctx.tokens.sm;
    let up = if top { 0.0 } else { round };
    let down = if bottom { 0.0 } else { round };
    let whole = Rect::new(rect.x, rect.y - up, rect.w, rect.h + up + down);
    ctx.clipped(rect, |ctx| {
        ctx.rounded(whole, ground, round);
        if let Some(edge) = edge {
            ctx.ring(whole, edge, round, ctx.tokens.hairline);
        }
    });
}

/// A row as a card on glass: its rounded ground, held when selected, lit under the pointer.
pub fn glass_card_row<A: App>(
    ctx: &mut Ctx<'_, A>,
    rect: Rect,
    target: Option<A::Target>,
    selected: bool,
) -> Rect {
    let card = rect.pad(Edges::xy(0.0, ctx.tokens.xs));
    let round = ctx.tokens.glass_round;
    ctx.rounded(card, ctx.styles.glass_card(), round);
    let lit = target.is_some_and(|target| ctx.interact(rect, target));
    let tint = match (selected, lit) {
        (true, _) => Some(ctx.styles.glass_chosen()),
        (false, true) => Some(ctx.styles.glass_hover()),
        (false, false) => None,
    };
    if let Some(color) = tint {
        ctx.rounded(card, color, round);
    }
    card
}
