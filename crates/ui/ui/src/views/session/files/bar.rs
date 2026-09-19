//! The rows the search types into: one per term, under the one mark they share.

use groove_gfx::Rect;

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::mark::Mark;
use crate::style::Role;
use crate::views::session::{Bar, Term};
use crate::widget::{hairline, row};

/// A row for each term the search narrows by. Returns what they took.
pub(super) fn draw(ctx: &mut Ctx, rect: Rect, ui: &Ui) -> Rect {
    let bar = &ui.session.bar;
    let height = ctx.tokens.row * Term::ALL.len() as f32;
    let whole = Rect::new(rect.x, rect.y, rect.w, height);
    ctx.quad(whole, ctx.styles.ground());
    hairline(ctx, whole, ctx.styles.line());
    for (at, term) in Term::ALL.into_iter().enumerate() {
        let y = whole.y + ctx.tokens.row * at as f32;
        let line = Rect::new(whole.x, y, whole.w, ctx.tokens.row);
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
    let at = match first {
        true => glass(ctx, line, role),
        false => past_glass(ctx),
    };
    let label = term.label();
    let quiet = ctx.styles.code(Role::Ghost);
    let width = ctx.measure(label, &quiet);
    row(
        ctx,
        Rect::new(line.x + at, line.y, width, line.h),
        0.0,
        label,
        quiet,
    );
    let text = match held {
        true => field.shown(),
        false => field.text().to_string(),
    };
    let at = at + width + ctx.tokens.sm;
    row(ctx, line, at, &text, ctx.styles.code(role));
}

/// The mark the bar carries, at the left of its first row. Returns where the text
/// starts, which every row shares.
fn glass(ctx: &mut Ctx, line: Rect, role: Role) -> f32 {
    let size = ctx.tokens.icon;
    let box_ = Rect::new(
        line.x + ctx.tokens.md,
        line.y + (line.h - size) / 2.0,
        size,
        size,
    );
    ctx.icon(box_, Mark::Search, 0, ctx.styles.color(role));
    past_glass(ctx)
}

fn past_glass(ctx: &Ctx) -> f32 {
    ctx.tokens.md + ctx.tokens.icon + ctx.tokens.sm
}
