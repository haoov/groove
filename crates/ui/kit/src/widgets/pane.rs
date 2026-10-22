//! A pane beside the work: its ground, and the hairline along the edge it shares.

use groove_gfx::{Edges, Rect};

use crate::base::ctx::{App, Ctx};
use crate::base::ground::Ground;
use crate::shape::{ground, side_rule};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

pub fn pane<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, fill: Ground, edge: Side) {
    ground(ctx, rect, fill);
    let x = match edge {
        Side::Left => rect.x,
        Side::Right => rect.right() - ctx.tokens.hairline,
    };
    side_rule(ctx, rect, x, ctx.styles.line());
}

/// A pane on glass: no ground, a hairline down its left edge, short of the ends.
pub fn glass_aside<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect) {
    let sm = ctx.tokens.sm;
    let span = rect.pad(Edges {
        top: sm,
        bottom: sm,
        ..Edges::all(0.0)
    });
    side_rule(ctx, span, rect.x, ctx.styles.line());
}
