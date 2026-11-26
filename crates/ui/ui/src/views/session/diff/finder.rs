//! The bar of a live search of the rows: glass over them, or with modes a row over the file's band.

use groove_gfx::{Edges, Rect};

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in, row_in};
use groove_ui_kit::shape::{Panel, from_right, from_top};
use groove_ui_kit::text::row;
use groove_ui_kit::widgets::{Search, glass_pane};

/// With modes, the search's own row from the top of `column`, a hairline around it.
pub(super) fn above(ctx: &mut Ctx, column: &mut Rect, ui: &Ui) {
    let Some(find) = ui.session.find.as_ref() else {
        return;
    };
    let bar = from_top(column, ctx.tokens.bar, ctx.tokens.sm);
    Panel::default()
        .border(ctx.styles.panel_edge())
        .draw(ctx, bar);
    let field = bar.pad(Edges::across(ctx.tokens.md, ctx.tokens.sm));
    let search = Search::new(&find.query, Target::Finding, find.typing);
    search.code().faint(Role::Faint).draw(ctx, field);
    count(ctx, field, &find.count());
}

pub(super) fn draw(ctx: &mut Ctx, body: Rect, ui: &Ui) {
    let Some(find) = ui.session.find.as_ref() else {
        return;
    };
    let sm = ctx.tokens.sm;
    let room = body.pad(Edges::xy(sm, sm));
    let [mut top, _] = column_in(room, [Spec::default().height(ctx.tokens.bar), Spec::fill()]);
    let wide = (ctx.tokens.palette / 2.0).min(top.w);
    let bar = from_right(&mut top, wide, 0.0);
    ctx.layer();
    glass_pane(ctx, bar);
    let field = bar.pad(Edges::across(sm, sm));
    let search = Search::new(&find.query, Target::Finding, find.typing);
    search.code().faint(Role::Faint).draw(ctx, field);
    count(ctx, field, &find.count());
}

/// How many matches there are, at the bar's own end.
fn count(ctx: &mut Ctx, bar: Rect, text: &str) {
    let style = ctx.styles.code(Role::Faint);
    let width = ctx.measure(text, &style);
    let wide = |width: f32| Spec::default().width(width);
    let [_, at, _] = row_in(bar, [Spec::fill(), wide(width), wide(ctx.tokens.md)]);
    row(ctx, at, 0.0, text, style);
}
