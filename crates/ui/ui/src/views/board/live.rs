//! Live: every session on disk, and what each one holds.

use groove_controllers::AppState;
use groove_controllers::session_service::Living;
use groove_gfx::{Edges, Rect};

use super::row::{Line, aside, named};
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::hoverable;
use groove_ui_kit::widgets::lead;

/// Every session the filter lets through.
pub(super) fn lines<'a>(app: &'a AppState, ui: &Ui) -> Vec<Line<'a>> {
    let query = ui.board.query();
    let living: Vec<&Living> = app
        .session
        .living
        .iter()
        .filter(|living| living.session.kind.routine().is_none())
        .filter(|living| query.lets_session(living, app.task.worked(&living.session)))
        .collect();
    if living.is_empty() {
        return vec![Line::Nothing(match query.is_empty() {
            true => "nothing here. + explorer starts one",
            false => "nothing the filter lets through",
        })];
    }
    living.into_iter().map(Line::Session).collect()
}

/// One session: its kind, its title, what it holds.
pub(super) fn session(ctx: &mut Ctx, line: Rect, app: &AppState, living: &Living) {
    let id = &living.session.id;
    hoverable(ctx, line, Target::Session(id.clone()));
    let (_sm, _size) = (ctx.tokens.sm, ctx.tokens.icon);
    let mut room = line.pad(Edges::across(ctx.tokens.md, 0.0));
    let role = match app.session.get(id).is_some() {
        true => Role::Working,
        false => Role::Ghost,
    };
    lead(ctx, &mut room, Mark::of_kind(&living.session.kind), role);
    aside(ctx, &mut room, &held(living));
    named(ctx, room, &living.session.title);
}

/// What a session holds, as the row's right-hand text.
fn held(living: &Living) -> String {
    let repos = match living.repos {
        1 => "1 repo".to_string(),
        n => format!("{n} repos"),
    };
    match living.worktrees.len() {
        1 => repos,
        n => format!("{repos} · {n} worktrees"),
    }
}
