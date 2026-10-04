//! The rows the search types into: one per term, under the one mark they share.

use groove_gfx::Rect;

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::views::session::{Bar, Term};
use groove_ui_kit::base::ground::Ground;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::hairline;
use groove_ui_kit::widgets::Search;

/// A row for each term the search narrows by. Returns what they took.
pub(super) fn draw(ctx: &mut Ctx, rect: Rect, ui: &Ui) -> Rect {
    let bar = &ui.session.bar;
    if !bar.in_use() {
        return Rect { h: 0.0, ..rect };
    }
    let shown: &[Term] = &Term::ALL;
    let whole = Rect {
        h: ctx.tokens.row * shown.len() as f32,
        ..rect
    };
    groove_ui_kit::shape::ground(ctx, whole, Ground::Work);
    hairline(ctx, whole, ctx.styles.line());
    let mut rows = whole;
    for (at, term) in shown.iter().copied().enumerate() {
        let line = rows.take_top(ctx.tokens.row);
        narrowing(ctx, line, bar, term, at == 0);
    }
    whole
}

/// One term of the bar: what it narrows by, and what is typed into it.
fn narrowing(ctx: &mut Ctx, line: Rect, bar: &Bar, term: Term, first: bool) {
    let field = match term {
        Term::Path => &bar.path,
        Term::Text => &bar.text,
    };
    let search = Search::new(field, Target::Term(term), bar.typing == Some(term));
    let search = search
        .prefix(term.label())
        .code()
        .glass(first)
        .faint(Role::Faint);
    search.draw(ctx, line);
}
