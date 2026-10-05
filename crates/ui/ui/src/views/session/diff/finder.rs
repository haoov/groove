//! The bar that pops over the rows while a search of them is live.

use groove_gfx::Rect;

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in, row_in};
use groove_ui_kit::shape::Panel;
use groove_ui_kit::text::row;
use groove_ui_kit::widgets::Search;

pub(super) fn draw(ctx: &mut Ctx, body: Rect, ui: &Ui) {
    let Some(find) = ui.session.find.as_ref() else {
        return;
    };
    let [bar, _] = column_in(body, [Spec::default().height(ctx.tokens.row), Spec::fill()]);
    ctx.layer();
    let panel = Panel::default().ground(ctx.styles.action());
    panel.border(ctx.styles.border()).draw(ctx, bar);
    let search = Search::new(&find.query, Target::Finding, find.typing);
    search.code().faint(Role::Faint).draw(ctx, bar);
    count(ctx, bar, &find.count());
}

/// How many matches there are, at the bar's own end.
fn count(ctx: &mut Ctx, bar: Rect, text: &str) {
    let style = ctx.styles.code(Role::Faint);
    let width = ctx.measure(text, &style);
    let wide = |width: f32| Spec::default().width(width);
    let [_, at, _] = row_in(bar, [Spec::fill(), wide(width), wide(ctx.tokens.md)]);
    row(ctx, at, 0.0, text, style);
}
