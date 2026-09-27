//! The panel a menu opens: a row per action, as wide as the widest of them.

use groove_gfx::{Color, Rect};

use crate::base::ctx::{App, Ctx};
use crate::base::style::Role;
use crate::text::row;

/// How much room the rows need, padding at both ends included.
pub fn size<A: App>(ctx: &mut Ctx<'_, A>, labels: &[&str]) -> (f32, f32) {
    let style = ctx.styles.body(Role::Text);
    let widest = labels
        .iter()
        .map(|label| ctx.measure(label, &style))
        .fold(0.0, f32::max);
    let width = (widest + ctx.tokens.md * 2.0).max(ctx.tokens.menu);
    (width, ctx.tokens.row * labels.len() as f32)
}

/// The rows with their top-left at `at`, kept inside `within`, a hit each.
pub fn menu<A: App>(
    ctx: &mut Ctx<'_, A>,
    at: (f32, f32),
    within: Rect,
    labels: &[&str],
    border: Color,
    target: impl Fn(usize) -> A::Target,
) {
    let (width, tall) = size(ctx, labels);
    let x = at.0.min(within.right() - width).max(within.x);
    let y = at.1.min(within.bottom() - tall).max(within.y);
    let box_ = Rect::new(x, y, width, tall);
    ctx.quad(box_, ctx.styles.band());
    ctx.border(box_, border);
    for (index, label) in labels.iter().enumerate() {
        let line = Rect::new(x, y + ctx.tokens.row * index as f32, width, ctx.tokens.row);
        if ctx.hovered(&target(index)) {
            ctx.quad(line, ctx.styles.hover());
        }
        let style = ctx.styles.body(Role::Text);
        row(ctx, line, ctx.tokens.md, label, style);
        ctx.hit(line, target(index));
    }
}
