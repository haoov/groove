//! The panel a menu opens: a row per action, as wide as the widest of them.

use groove_gfx::{Color, Edges, Rect, TextStyle};

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

/// The rows with their top-left at `at`, kept inside `within`, a hit each, each in its role or the text's.
pub fn menu<A: App>(
    ctx: &mut Ctx<'_, A>,
    at: (f32, f32),
    within: Rect,
    (labels, roles): (&[&str], &[Role]),
    border: Color,
    target: impl Fn(usize) -> A::Target,
) {
    let (width, tall) = size(ctx, labels);
    let box_ = crate::shape::kept_in(at, (width, tall), within);
    let (band, round) = (ctx.styles.band(), ctx.tokens.round);
    ctx.rounded(box_, band, round);
    for (index, label) in labels.iter().enumerate() {
        let line = row_of(ctx, box_, index);
        let style = ctx
            .styles
            .body(roles.get(index).copied().unwrap_or(Role::Text));
        let ground = ctx.hovered(&target(index)).then(|| ctx.styles.hover());
        listed(ctx, (line, ctx.tokens.md), (label, style), ground);
        ctx.hit(line, target(index));
    }
    ctx.ring(box_, border, round, 1.0);
}

/// Row `index` of a panel's list of rows.
pub(crate) fn row_of<A: App>(ctx: &Ctx<'_, A>, panel: Rect, index: usize) -> Rect {
    let tall = ctx.tokens.row;
    Rect::new(panel.x, panel.y + tall * index as f32, panel.w, tall)
}

/// One row of a panel's list: its ground when it stands out, then its label from `pad`.
pub(crate) fn listed<A: App>(
    ctx: &mut Ctx<'_, A>,
    (line, pad): (Rect, f32),
    (label, style): (&str, TextStyle),
    ground: Option<Color>,
) {
    if let Some(ground) = ground {
        ctx.rounded(line, ground, ctx.tokens.round);
    }
    row(ctx, line, pad, label, style);
}

/// How much room rows on glass need, the pane's inset included.
pub fn glass_size<A: App>(ctx: &mut Ctx<'_, A>, labels: &[&str]) -> (f32, f32) {
    let (wide, _) = size(ctx, labels);
    let sm = ctx.tokens.sm;
    let tall = ctx.tokens.glass_row * labels.len() as f32;
    (wide + sm * 2.0, tall + sm * 2.0)
}

/// The rows on a pane of glass with its top-left at `at`, kept inside `within`, as the pickers stand.
pub fn glass_menu<A: App>(
    ctx: &mut Ctx<'_, A>,
    at: (f32, f32),
    within: Rect,
    labels: &[&str],
    target: impl Fn(usize) -> A::Target,
) {
    let (width, tall) = glass_size(ctx, labels);
    let pane = crate::shape::kept_in(at, (width, tall), within);
    crate::widgets::glass_pane(ctx, pane);
    let rows = pane.pad(Edges::xy(0.0, ctx.tokens.sm));
    let (high, style) = (ctx.tokens.glass_row, ctx.styles.body(Role::Text));
    for (index, label) in labels.iter().enumerate() {
        let line = Rect::new(rows.x, rows.y + high * index as f32, rows.w, high);
        let inset = crate::widgets::inset_row(ctx, line, target(index), (false, None));
        row(ctx, inset, ctx.tokens.md, label, style);
    }
}
