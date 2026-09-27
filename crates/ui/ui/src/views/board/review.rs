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
use groove_ui_kit::text::{Label, ago};
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

/// One MR: its project and number, its title, its author, and when it last moved.
pub(super) fn item(ctx: &mut Ctx, line: Rect, mr: &ReviewMr) {
    hoverable(ctx, line, Target::Review(mr.project.clone(), mr.iid));
    let size = ctx.tokens.icon;
    let mut room = line.pad(Edges::across(ctx.tokens.md, 0.0));
    let mark = square(room.take_left(size), size);
    room.take_left(ctx.tokens.sm);
    icon(ctx, mark, Mark::Review, role(mr));
    aside(ctx, &mut room, &ago(mr.updated_at.age_at(ctx.now)));
    under(ctx, room, mr);
    named(ctx, room, &mr.title);
}

/// The project, the number and the author, under the title.
fn under(ctx: &mut Ctx, room: Rect, mr: &ReviewMr) {
    let named = format!("{}{}{}", mr.project, mr.forge.sigil(), mr.iid);
    let text = match mr.author.is_empty() {
        true => named,
        false => format!("{named} · {}", mr.author),
    };
    let second = Rect {
        y: room.y + ctx.tokens.row,
        h: ctx.tokens.row,
        ..room
    };
    Label::new(&text, ctx.styles.small(Role::Faint)).draw(ctx, second);
}

fn role(mr: &ReviewMr) -> Role {
    match (mr.draft, mr.approved) {
        (true, _) => Role::Ghost,
        (false, true) => Role::Ok,
        (false, false) => Role::Attention,
    }
}
