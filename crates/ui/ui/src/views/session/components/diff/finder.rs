//! The bar that pops over the rows while a search of them is live.

use groove_gfx::Rect;

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::mark::Mark;
use crate::style::Role;
use crate::widget::row;

pub(super) fn draw(ctx: &mut Ctx, body: Rect, ui: &Ui) {
    let Some(find) = ui.session.find.as_ref() else {
        return;
    };
    let bar = Rect::new(body.x, body.y, body.w, ctx.tokens.row);
    ctx.layer();
    ctx.quad(bar, ctx.styles.action());
    ctx.border(bar, ctx.styles.here());
    ctx.hit(bar, Target::Finding);
    let role = match find.typing {
        true => Role::Text,
        false => Role::Faint,
    };
    let size = ctx.tokens.icon;
    let glass = Rect::new(
        bar.x + ctx.tokens.md,
        bar.y + (bar.h - size) / 2.0,
        size,
        size,
    );
    ctx.icon(glass, Mark::Search, 0, ctx.styles.color(role));
    let typed = match find.typing {
        true => find.query.shown(),
        false => find.query.text().to_string(),
    };
    let at = glass.right() - bar.x + ctx.tokens.sm;
    row(ctx, bar, at, &typed, ctx.styles.code(role));
    count(ctx, bar, &find.count());
}

/// How many matches there are, at the bar's own end.
fn count(ctx: &mut Ctx, bar: Rect, text: &str) {
    let style = ctx.styles.code(Role::Faint);
    let width = ctx.measure(text, &style);
    let at = bar.right() - ctx.tokens.md - width;
    row(ctx, Rect::new(at, bar.y, width, bar.h), 0.0, text, style);
}
