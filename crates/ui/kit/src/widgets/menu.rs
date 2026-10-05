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
    let box_ = crate::shape::kept_in(at, (width, tall), within);
    let (band, round) = (ctx.styles.band(), ctx.tokens.round);
    ctx.rounded(box_, band, round);
    for (index, label) in labels.iter().enumerate() {
        let line = Rect::new(
            box_.x,
            box_.y + ctx.tokens.row * index as f32,
            width,
            ctx.tokens.row,
        );
        if ctx.hovered(&target(index)) {
            ctx.rounded(line, ctx.styles.hover(), round);
        }
        let style = ctx.styles.body(Role::Text);
        row(ctx, line, ctx.tokens.md, label, style);
        ctx.hit(line, target(index));
    }
    ctx.ring(box_, border, round, 1.0);
}
