//! Live: every session on disk, and what each one holds.

use groove_controllers::AppState;
use groove_controllers::session_service::Living;
use groove_gfx::Rect;

use super::row::{Line, aside, named};
use crate::Ui;
use crate::base::ctx::Ctx;
use crate::base::hit::Target;
use crate::base::mark::Mark;
use crate::base::style::Role;
use crate::shape::{after_mark, hoverable, leading};
use crate::widgets::icon;

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
    let target = Target::Session(id.clone());
    hoverable(ctx, line, target);
    let kind = twisty(ctx, line, ui, living);
    let box_ = leading(ctx, line, line.x + kind);
    let role = match app.session.get(id).is_some() {
        true => Role::Working,
        false => Role::Ghost,
    };
    icon(ctx, box_, Mark::of_kind(&living.session.kind), role);
    let until = aside(ctx, line, &held(living));
    named(
        ctx,
        line,
        &living.session.title,
        until,
        after_mark(ctx, kind),
    );
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

/// What opens a session's worktrees under it. Returns where the kind icon goes.
fn twisty(ctx: &mut Ctx, line: Rect, ui: &Ui, living: &Living) -> f32 {
    let box_ = leading(ctx, line, line.x + ctx.tokens.sm);
    let turn = match ui.board.is_open(&living.session.id) {
        true => 0,
        false => Mark::RIGHTWARDS,
    };
    ctx.icon(box_, Mark::Down, turn, ctx.styles.color(Role::Ghost));
    ctx.hit(box_, Target::Unfold(living.session.id.clone()));
    after_mark(ctx, ctx.tokens.sm)
}
