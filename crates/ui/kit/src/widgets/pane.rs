//! A pane on glass beside the work, and the side a thing stands at.

use groove_gfx::{Edges, Rect};

use crate::base::ctx::{App, Ctx};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

/// The sidebar's box: its room, a hairline in from each side, out to its border and up to the
/// rule `sm` over it. Its own border is the separator, and what a focus ring goes around.
pub fn aside_bounds(tokens: &crate::base::tokens::Tokens, rect: Rect) -> Rect {
    let edge = tokens.hairline;
    rect.pad(Edges {
        top: -tokens.sm,
        left: -edge,
        right: -edge,
        bottom: 0.0,
    })
}

/// The sidebar set into its panel: a box a step below the panel's base, edged by a hairline that
/// is its separator on the left and lies on the panel's own lines elsewhere.
pub fn glass_aside<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect) {
    let (edge, round) = (ctx.tokens.hairline, ctx.tokens.glass_round);
    let bounds = aside_bounds(&ctx.tokens, rect);
    let ground = ctx.styles.band();
    ctx.rounded(
        bounds.pad(Edges::all(edge)),
        ground,
        (round - edge).max(0.0),
    );
    ctx.ring(bounds, ctx.styles.line(), round, edge);
}
