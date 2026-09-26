//! Review: the merge requests the forges ask this user to look at.

use groove_controllers::AppState;
use groove_gfx::Rect;
use groove_types::ReviewMr;

use super::row::{Line, aside, named};
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::mark::Mark;
use crate::style::Role;
use crate::widget::{after_mark, ago, elide, icon, leading, row};

/// Every MR the filter lets through, newest first.
pub(super) fn lines<'a>(app: &'a AppState, ui: &Ui) -> Vec<Line<'a>> {
    let query = ui.board.query();
    let asked: Vec<&ReviewMr> = app
        .delivery
        .reviews
        .iter()
        .filter(|mr| query.lets_review(mr))
        .collect();
    if asked.is_empty() {
        return vec![Line::Nothing(match query.is_empty() {
            true => "nothing is waiting on you",
            false => "nothing the filter lets through",
        })];
    }
    asked.into_iter().map(Line::Review).collect()
}

/// One MR: its project and number, its title, its author, and when it last moved.
pub(super) fn item(ctx: &mut Ctx, line: Rect, ui: &Ui, mr: &ReviewMr) {
    let target = Target::Review(mr.project.clone(), mr.iid);
    if ui.hover.as_ref() == Some(&target) {
        ctx.quad(line, ctx.styles.hover());
    }
    ctx.hit(line, target);
    let box_ = leading(ctx, line, line.x + ctx.tokens.md);
    icon(ctx, box_, Mark::Review, role(mr));
    let until = aside(ctx, line, &ago(mr.updated_at.age_at(ctx.now)));
    named(ctx, line, &mr.title, until, after_mark(ctx, ctx.tokens.md));
    under(ctx, line, mr, until);
}

/// The project, the number and the author, under the title.
fn under(ctx: &mut Ctx, line: Rect, mr: &ReviewMr, until: f32) {
    let style = ctx.styles.small(Role::Faint);
    let named = format!("{}{}{}", mr.project, mr.forge.sigil(), mr.iid);
    let text = match mr.author.is_empty() {
        true => named,
        false => format!("{named} · {}", mr.author),
    };
    let room = (until - line.x - after_mark(ctx, ctx.tokens.md)).max(0.0);
    let text = elide(ctx, &text, &style, room);
    let second = Rect::new(line.x, line.y + ctx.tokens.row, line.w, ctx.tokens.row);
    row(ctx, second, after_mark(ctx, ctx.tokens.md), &text, style);
}

fn role(mr: &ReviewMr) -> Role {
    match (mr.draft, mr.approved) {
        (true, _) => Role::Ghost,
        (false, true) => Role::Ok,
        (false, false) => Role::Attention,
    }
}
