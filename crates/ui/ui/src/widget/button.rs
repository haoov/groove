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
    let word = ctx.measure(label, &style);
    let box_ = slot(ctx, line, word, ground);
    row(ctx, box_, ctx.tokens.sm, label, style);
    box_
}

/// Room of `content` plus the padding either side, at the row's right end.
pub fn slot(ctx: &mut Ctx, line: Rect, content: f32, ground: Option<Color>) -> Rect {
    let pad = ctx.tokens.sm;
    let at = line.right() - pad - (content + pad * 2.0);
    slot_at(ctx, line, at, content, ground)
}

/// The same room, from `x`, on `ground`, which carries a border. Returns its box, for
/// the caller to fill and to register.
pub fn slot_at(ctx: &mut Ctx, line: Rect, x: f32, content: f32, ground: Option<Color>) -> Rect {
    let pad = ctx.tokens.sm;
    let height = (ctx.tokens.row - ctx.tokens.xs).min(line.h - ctx.tokens.xs);
    let box_ = Rect::new(
        x,
        line.y + (line.h - height) / 2.0,
        content + pad * 2.0,
        height,
    );
    if let Some(ground) = ground {
        ctx.quad(box_, ground);
        ctx.border(box_, ctx.styles.line());
    }
    box_
}
