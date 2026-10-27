//! The glass bar that floats over the rows while a search of them is live.

use groove_gfx::{Edges, Rect};

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in, row_in};
use groove_ui_kit::text::row;
use groove_ui_kit::widgets::{Search, glass_pane};

pub(super) fn draw(ctx: &mut Ctx, body: Rect, ui: &Ui) {
    let Some(find) = ui.session.find.as_ref() else {
        return;
    };
    let sm = ctx.tokens.sm;
    let room = body.pad(Edges::xy(sm, sm));
    let [bar, _] = column_in(room, [Spec::default().height(ctx.tokens.bar), Spec::fill()]);
    ctx.layer();
    glass_pane(ctx, bar);
    let field = bar.pad(Edges::across(sm, sm));
    let search = Search::new(&find.query, Target::Finding, find.typing);
    search
        .code()
        .glass(false)
        .faint(Role::Faint)
        .draw(ctx, field);
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
