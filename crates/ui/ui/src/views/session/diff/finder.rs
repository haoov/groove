//! The bar that pops over the rows while a search of them is live.

use groove_gfx::Rect;

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::Panel;
use groove_ui_kit::text::row;
use groove_ui_kit::widgets::Search;

pub(super) fn draw(ctx: &mut Ctx, body: Rect, ui: &Ui) {
    let Some(find) = ui.session.find.as_ref() else {
        return;
    };
    let bar = Rect::new(body.x, body.y, body.w, ctx.tokens.row);
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
    let at = bar.right() - ctx.tokens.md - width;
    row(ctx, Rect::new(at, bar.y, width, bar.h), 0.0, text, style);
}
