//! Up next: the tasks in the user's own order, and the divider they stand either side of.

use groove_controllers::AppState;
use groove_controllers::session_service::Living;
use groove_controllers::task_service::Planned;
use groove_gfx::Rect;
use groove_types::{ExternalId, SessionKind, Task};

use super::column::Line;
use crate::Ui;
use crate::ctx::Ctx;
use crate::style::Role;
use crate::widget::row;

const LATER: &str = "LATER";

/// What Up next holds: the tasks over the divider, the divider, the ones under it.
pub(super) fn lines<'a>(app: &'a AppState, ui: &Ui) -> Vec<Line<'a>> {
    let mut lines = Vec::new();
    for (at, planned) in waiting(app, ui).into_iter().enumerate() {
        if planned.later && !lines.iter().any(is_divider) {
            lines.push(Line::Divider);
        }
        lines.push(Line::Task(at + 1, planned.task));
    }
    if !lines.iter().any(is_divider) {
        lines.push(Line::Divider);
    }
    lines
}

/// Where every line of Up next starts, and where the last one ends.
fn edges(tokens: &crate::tokens::Tokens, app: &AppState, ui: &Ui) -> Vec<f32> {
    let lines = lines(app, ui);
    let mut at = 0.0;
    let mut edges = vec![0.0];
    for height in super::column::heights(tokens, app, &lines) {
        at += height;
        edges.push(at);
    }
    edges
}

/// The tasks the filter lets through that no session works, in the user's order, the
/// ones that need the user first.
fn waiting<'a>(app: &'a AppState, ui: &Ui) -> Vec<Planned<'a>> {
    let query = ui.board.query();
    let left: Vec<&Task> = app
        .task
        .tasks
        .iter()
        .filter(|task| !app.session.living.iter().any(|living| holds(living, task)))
        .filter(|task| query.lets_task(task))
        .collect();
    let planned = app.task.planned(&left);
    let (asking, rest): (Vec<Planned<'_>>, Vec<Planned<'_>>) = planned
        .into_iter()
        .partition(|one| !app.task.needs(&one.task.external_id).is_empty());
    asking.into_iter().chain(rest).collect()
}

fn holds(living: &Living, task: &Task) -> bool {
    match &living.session.kind {
        SessionKind::Task { external_id } => *external_id == task.external_id,
        _ => false,
    }
}

fn is_divider(line: &Line<'_>) -> bool {
    matches!(line, Line::Divider)
}

/// The divider a task is dragged under to leave the plan for later.
pub(super) fn divider(ctx: &mut Ctx, line: Rect) {
    let style = ctx.styles.heading(Role::Ghost);
    let pad = ctx.tokens.md;
    let from = line.x + pad * 2.0 + ctx.measure(LATER, &style);
    row(ctx, line, pad, LATER, style);
    let rule = Rect::new(
        from,
        line.y + (line.h / 2.0).floor(),
        (line.right() - pad - from).max(0.0),
        ctx.tokens.hairline,
    );
    ctx.quad(rule, ctx.styles.line());
}

/// The rule where a dragged row would land, over the rows it moves between.
pub(super) fn dragging(ctx: &mut Ctx, body: Rect, app: &AppState, ui: &Ui, scroll: f32) {
    let Some(at) = ui.board.drop else {
        return;
    };
    let edges = edges(&ctx.tokens, app, ui);
    let Some(edge) = edges.get(at) else {
        return;
    };
    let thick = ctx.tokens.hairline * 2.0;
    let y = body.y - scroll + edge - thick / 2.0;
    ctx.quad(
        Rect::new(body.x, y, body.w, thick),
        ctx.styles.color(Role::Working),
    );
}

/// Which insertion point a drag lands on, counted in lines from the column's top.
pub fn dropped(
    tokens: &crate::tokens::Tokens,
    app: &AppState,
    ui: &Ui,
    body: Rect,
    scroll: f32,
    y: f32,
) -> usize {
    let at = y - body.y + scroll;
    let edges = edges(tokens, app, ui);
    edges
        .iter()
        .enumerate()
        .min_by(|(_, one), (_, other)| (*one - at).abs().total_cmp(&(*other - at).abs()))
        .map(|(index, _)| index)
        .unwrap_or_default()
}

/// What the drop asks of the plan: the task, the one it lands above, and its side.
pub fn moving(app: &AppState, ui: &Ui) -> Option<(ExternalId, Option<ExternalId>, bool)> {
    let id = ui.board.dragging.clone()?;
    let at = ui.board.drop?;
    let lines = lines(app, ui);
    let later = lines
        .iter()
        .position(is_divider)
        .is_some_and(|at_| at_ < at);
    let before = lines
        .get(at..)
        .unwrap_or_default()
        .iter()
        .find_map(|line| match line {
            Line::Task(_, task) => Some(task.external_id.clone()),
            _ => None,
        });
    match before.as_ref() == Some(&id) {
        true => None,
        false => Some((id, before, later)),
    }
}
