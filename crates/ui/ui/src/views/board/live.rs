//! Live: every session on disk, and what each one holds.

use groove_controllers::AppState;
use groove_controllers::session_service::Living;
use groove_gfx::Rect;

use super::row::{Line, aside, named};
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{hoverable, square};
use groove_ui_kit::widgets::icon;

/// Every session the filter lets through, with its worktrees under it while it is open.
pub(super) fn lines<'a>(app: &'a AppState, ui: &Ui) -> Vec<Line<'a>> {
    let query = ui.board.query();
    let living: Vec<&Living> = app
        .session
        .living
        .iter()
        .filter(|living| query.lets_session(living, app.task.worked(&living.session)))
        .collect();
    if living.is_empty() {
        return vec![Line::Nothing(match query.is_empty() {
            true => "nothing here. Ctrl+Shift+N starts an explorer",
            false => "nothing the filter lets through",
        })];
    }
    let mut lines = Vec::new();
    for living in living {
        lines.push(Line::Session(living));
        if !ui.board.is_open(&living.session.id) {
            continue;
        }
        let open = app.session.get(&living.session.id);
        for worktree in &living.worktrees {
            let delivery =
                open.map(|open| app.delivery.row(&worktree.id, open.status_of(&worktree.id)));
            lines.push(Line::Worktree(worktree, delivery));
        }
    }
    lines
}

/// One session: a twisty for its worktrees, its kind, its title, what it holds.
pub(super) fn session(ctx: &mut Ctx, line: Rect, app: &AppState, ui: &Ui, living: &Living) {
    let id = &living.session.id;
    hoverable(ctx, line, Target::Session(id.clone()));
    let (sm, size) = (ctx.tokens.sm, ctx.tokens.icon);
    let mut room = line;
    twisty(ctx, &mut room, ui, living);
    let kind = square(room.take_left(size), size);
    room.take_left(sm);
    let role = match app.session.get(id).is_some() {
        true => Role::Working,
        false => Role::Ghost,
    };
    icon(ctx, kind, Mark::of_kind(&living.session.kind), role);
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

/// What opens a session's worktrees under it, from the left of `room`.
fn twisty(ctx: &mut Ctx, room: &mut Rect, ui: &Ui, living: &Living) {
    let (sm, size) = (ctx.tokens.sm, ctx.tokens.icon);
    room.take_left(sm);
    let box_ = square(room.take_left(size), size);
    room.take_left(sm);
    let turn = match ui.board.is_open(&living.session.id) {
        true => 0,
        false => Mark::RIGHTWARDS,
    };
    ctx.icon(box_, Mark::Down, turn, ctx.styles.color(Role::Ghost));
    ctx.hit(box_, Target::Unfold(living.session.id.clone()));
}
