//! Up next: the tasks in the user's own order.

use groove_controllers::AppState;
use groove_controllers::task::Landing;
use groove_controllers::task_service::Planned;
use groove_gfx::Rect;
use groove_types::Task;

use super::row::Line;
use crate::Ui;
use crate::ctx::Ctx;
use groove_ui_kit::base::ground::Ground;
use groove_ui_kit::base::style::Role;

/// What Up next holds: the tasks in the user's order.
pub(super) fn lines(app: &AppState) -> Vec<Line<'_>> {
    upcoming(app)
        .into_iter()
        .map(|planned| Line::Task(planned.task))
        .collect()
}

/// Where every line of Up next starts, and where the last one ends.
fn edges(tokens: &groove_ui_kit::base::tokens::Tokens, app: &AppState) -> Vec<f32> {
    let lines = lines(app);
    let mut at = 0.0;
    let mut edges = vec![0.0];
    for height in super::row::heights(tokens, app, &lines) {
        at += height;
        edges.push(at);
    }
    edges
}

/// The tasks no session works, in the user's order, those that need the user first on each side.
fn upcoming(app: &AppState) -> Vec<Planned<'_>> {
    let left: Vec<&Task> = app.task.waiting(&app.session.worked());
    let mut planned = app.task.planned(&left);
    planned.sort_by_key(|one| !app.task.asks(&one.task.external_id));
    planned
}

/// The rule where a dragged row would land, over the rows it moves between.
pub(super) fn dragging(ctx: &mut Ctx, body: Rect, app: &AppState, ui: &Ui) {
    let Some(at) = ui.board.drop else {
        return;
    };
    let scroll = ui.board.scroll(super::List::Next);
    let edges = edges(&ctx.tokens, app);
    let Some(edge) = edges.get(at) else {
        return;
    };
    let thick = ctx.tokens.hairline * 2.0;
    let bar = groove_ui_kit::shape::band(body, edge - scroll - thick / 2.0, thick);
    groove_ui_kit::shape::ground(ctx, bar, Ground::Solid(Role::Working));
}

/// Which insertion point a drag lands on, counted in lines from the column's top.
pub fn dropped(
    tokens: &groove_ui_kit::base::tokens::Tokens,
    app: &AppState,
    ui: &Ui,
    body: Rect,
    y: f32,
) -> usize {
    let at = y - body.y + ui.board.scroll(super::List::Next);
    let edges = edges(tokens, app);
    edges
        .iter()
        .enumerate()
        .min_by(|(_, one), (_, other)| (*one - at).abs().total_cmp(&(*other - at).abs()))
        .map(|(index, _)| index)
        .unwrap_or_default()
}

/// What the drop asks of the plan, or nothing when the row would not move.
pub fn landing(app: &AppState, ui: &Ui) -> Option<Landing> {
    let external_id = ui.carried()?.clone();
    let at = ui.board.drop?;
    let lines = lines(app);
    let before = lines
        .get(at..)
        .unwrap_or_default()
        .iter()
        .map(|Line::Task(task)| task.external_id.clone())
        .next();
    match before.as_ref() == Some(&external_id) {
        true => None,
        false => Some(Landing {
            external_id,
            before,
        }),
    }
}
