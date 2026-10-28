//! A pane on glass beside the work, and the side a thing stands at.

use groove_gfx::{Edges, Rect};

use crate::base::ctx::{App, Ctx};
use crate::shape::side_rule;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

/// A pane on glass: no ground, a hairline down its left edge, short of the ends.
pub fn glass_aside<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect) {
    let lg = ctx.tokens.lg;
    let span = rect.pad(Edges {
        top: lg,
        bottom: lg,
        ..Edges::all(0.0)
    });
    side_rule(ctx, span, rect.x, ctx.styles.line());
}
