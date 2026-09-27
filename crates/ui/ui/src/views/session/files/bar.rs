//! The rows the search types into: one per term, under the one mark they share.

use groove_gfx::{Edges, Rect};

use crate::Ui;
use crate::base::ctx::Ctx;
use crate::base::hit::Target;
use crate::base::mark::Mark;
use crate::base::style::Role;
use crate::shape::{hairline, square};
use crate::text::{Label, row};
use crate::views::session::{Bar, Term};

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
    ctx.quad(whole, ctx.styles.ground());
    hairline(ctx, whole, ctx.styles.line());
    let mut rows = whole;
    for (at, term) in shown.iter().copied().enumerate() {
        let line = rows.take_top(ctx.tokens.row);
        narrowing(ctx, line, bar, term, at == 0);
        ctx.hit(line, Target::Term(term));
    }
    whole
}

/// One term of the bar: what it narrows by, and what is typed into it.
fn narrowing(ctx: &mut Ctx, line: Rect, bar: &Bar, term: Term, first: bool) {
    let held = bar.typing == Some(term);
    let role = match held {
        true => Role::Text,
        false => Role::Faint,
    };
    let field = match term {
        Term::Path => &bar.path,
        Term::Text => &bar.text,
    };
    let (sm, size) = (ctx.tokens.sm, ctx.tokens.icon);
    let mut room = line.pad(Edges::across(ctx.tokens.md, 0.0));
    let glass = square(room.take_left(size), size);
    room.take_left(sm);
    if first {
        ctx.icon(glass, Mark::Search, 0, ctx.styles.color(role));
    }
    Label::new(term.label(), ctx.styles.code(Role::Ghost)).left(ctx, &mut room, sm);
    let text = match held {
        true => field.shown(),
        false => field.text().to_string(),
    };
    row(ctx, room, 0.0, &text, ctx.styles.code(role));
}
