//! A word a click acts on, at the end of a row.

use groove_gfx::{Color, Rect, TextStyle};

use crate::ctx::Ctx;
use crate::widget::row;

/// A label with `ground` behind it, inset from the row's edges, at its right end.
/// Returns its box, for the caller to register and to measure what is left.
pub fn button(
    ctx: &mut Ctx,
    line: Rect,
    label: &str,
    style: TextStyle,
    ground: Option<Color>,
) -> Rect {
    let (pad, inset) = (ctx.tokens.sm, ctx.tokens.xs);
    let width = ctx.measure(label, &style) + pad * 2.0;
    let at = line.right() - pad - width;
    let box_ = Rect::new(at, line.y + inset, width, line.h - inset * 2.0);
    if let Some(ground) = ground {
        ctx.quad(box_, ground);
    }
    row(ctx, box_, pad, label, style);
    box_
}
