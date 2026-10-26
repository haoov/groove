//! The panel a menu opens: a row per action, as wide as the widest of them.

use groove_gfx::{Edges, Rect};

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

/// How much room rows on glass need, the pane's inset included.
pub fn glass_size<A: App>(ctx: &mut Ctx<'_, A>, labels: &[&str]) -> (f32, f32) {
    let (wide, _) = size(ctx, labels);
    let sm = ctx.tokens.sm;
    let tall = ctx.tokens.glass_row * labels.len() as f32;
    (wide + sm * 2.0, tall + sm * 2.0)
}

/// The rows on a pane of glass with its top-left at `at`, kept inside `within`, each in its role or the text's.
pub fn glass_menu<A: App>(
    ctx: &mut Ctx<'_, A>,
    at: (f32, f32),
    within: Rect,
    (labels, roles): (&[&str], &[Role]),
    target: impl Fn(usize) -> A::Target,
) {
    let (width, tall) = glass_size(ctx, labels);
    let pane = crate::shape::kept_in(at, (width, tall), within);
    crate::widgets::glass_pane(ctx, pane);
    let rows = pane.pad(Edges::xy(0.0, ctx.tokens.sm));
    let high = ctx.tokens.glass_row;
    for (index, label) in labels.iter().enumerate() {
        let role = roles.get(index).copied().unwrap_or(Role::Text);
        let style = ctx.styles.body(role);
        let line = Rect::new(rows.x, rows.y + high * index as f32, rows.w, high);
        let inset = crate::widgets::inset_row(ctx, line, target(index), (false, None));
        row(ctx, inset, ctx.tokens.md, label, style);
    }
}
