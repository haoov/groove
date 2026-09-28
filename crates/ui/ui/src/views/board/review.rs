//! Review: the merge requests the forges ask this user to look at.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_types::ReviewMr;

use super::row::{Line, aside, named};
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{hoverable, square};
use groove_ui_kit::text::ago;
use groove_ui_kit::widgets::icon;

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

/// One MR: its title and when it last moved, then its project, number, author and review.
pub(super) fn item(ctx: &mut Ctx, line: Rect, mr: &ReviewMr) {
    hoverable(ctx, line, Target::Review(mr.project.clone(), mr.iid));
    let mut rest = line;
    let first = rest.take_top(super::row::item(&ctx.tokens));
    let size = ctx.tokens.icon;
    let mut room = first.pad(Edges::across(ctx.tokens.md, 0.0));
    let mark = square(room.take_left(size), size);
    room.take_left(ctx.tokens.sm);
    icon(ctx, mark, Mark::Review, role(mr));
    let start = room.x;
    aside(ctx, &mut room, &ago(mr.updated_at.age_at(ctx.now)));
    named(ctx, room, &mr.title);
    let second = rest.take_top(ctx.tokens.line);
    under(ctx, second.pad(Edges::across(start - second.x, 0.0)), mr);
}

/// The project, the number and the author, then where its reviewers stand.
fn under(ctx: &mut Ctx, line: Rect, mr: &ReviewMr) {
    let named = format!("{}{}{}", mr.project, mr.forge.sigil(), mr.iid);
    let text = match mr.author.is_empty() {
        true => named,
        false => format!("{named} · {}", mr.author),
    };
    let style = ctx.styles.small(Role::Faint);
    let room = line.w - ctx.tokens.md;
    let text = groove_ui_kit::text::elide(ctx, &text, &style, room);
    let end = line.x + ctx.measure(&text, &style);
    groove_ui_kit::text::row(ctx, line, 0.0, &text, style);
    if let Some(review) = mr.review {
        crate::components::review_word(ctx, line, end + ctx.tokens.sm, review);
    }
}

fn role(mr: &ReviewMr) -> Role {
    match (mr.draft, mr.approved) {
        (true, _) => Role::Ghost,
        (false, true) => Role::Ok,
        (false, false) => Role::Attention,
    }
}
