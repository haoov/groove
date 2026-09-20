//! The panel a menu opens: a row per action, as wide as the widest of them.

use groove_gfx::{Color, Rect};

use crate::ctx::Ctx;
use crate::hit::Target;
use crate::style::Role;
use crate::widget::row;

/// How much room the rows need, padding at both ends included.
pub fn size(ctx: &mut Ctx, labels: &[&str]) -> (f32, f32) {
    let style = ctx.styles.body(Role::Text);
    let widest = labels
        .iter()
        .map(|label| ctx.measure(label, &style))
        .fold(0.0, f32::max);
    let width = (widest + ctx.tokens.md * 2.0).max(ctx.tokens.menu);
    (width, ctx.tokens.row * labels.len() as f32)
}

/// Draws the rows with their top-left at `at`, kept inside `within`. Registers a hit
/// per row.
pub fn menu(
    ctx: &mut Ctx,
    at: (f32, f32),
    within: Rect,
    labels: &[&str],
    hovered: Option<usize>,
    border: Color,
) {
    let (width, tall) = size(ctx, labels);
    let x = at.0.min(within.right() - width).max(within.x);
    let y = at.1.min(within.bottom() - tall).max(within.y);
    let box_ = Rect::new(x, y, width, tall);
    ctx.quad(box_, ctx.styles.band());
    ctx.border(box_, border);
    for (index, label) in labels.iter().enumerate() {
        let line = Rect::new(x, y + ctx.tokens.row * index as f32, width, ctx.tokens.row);
        if hovered == Some(index) {
            ctx.quad(line, ctx.styles.hover());
        }
        let style = ctx.styles.body(Role::Text);
        row(ctx, line, ctx.tokens.md, label, style);
        ctx.hit(line, Target::MenuRow(index));
    }
}
