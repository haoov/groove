//! The bar of a live search of the rows: a row over the file's band.

use groove_gfx::{Edges, Rect};

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, row_in};
use groove_ui_kit::shape::{Panel, from_top};
use groove_ui_kit::text::row;
use groove_ui_kit::widgets::Search;

/// The search's own row from the top of `column`, a hairline around it.
pub(crate) fn above(ctx: &mut Ctx, column: &mut Rect, ui: &Ui) {
    let Some(find) = ui.session.find.as_ref() else {
        return;
    };
    let search = Search::new(&find.query, Target::Finding, find.typing);
    row_over(ctx, column, search, Some(&find.count()));
}

/// A search's own row from the top of `column`, a hairline around it, its count at its end.
pub(crate) fn row_over(
    ctx: &mut Ctx,
    column: &mut Rect,
    search: Search<'_, Target>,
    counted: Option<&str>,
) {
    let bar = from_top(column, ctx.tokens.bar, ctx.tokens.sm);
    Panel::default()
        .border(ctx.styles.panel_edge())
        .draw(ctx, bar);
    let field = bar.pad(Edges::across(ctx.tokens.md, ctx.tokens.sm));
    search.code().faint(Role::Faint).draw(ctx, field);
    if let Some(text) = counted {
        count(ctx, field, text);
    }
}

/// How many matches there are, at the bar's own end.
fn count(ctx: &mut Ctx, bar: Rect, text: &str) {
    let style = ctx.styles.code(Role::Faint);
    let width = ctx.measure(text, &style);
    let wide = |width: f32| Spec::default().width(width);
    let [_, at, _] = row_in(bar, [Spec::fill(), wide(width), wide(ctx.tokens.md)]);
    row(ctx, at, 0.0, text, style);
}
