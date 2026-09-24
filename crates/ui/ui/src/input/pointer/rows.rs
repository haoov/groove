//! What a row of another surface answers for: the board, the rail, a task's own menu.

use groove_controllers::{AppState, Command, session};

use super::{agent, board, rail};
use crate::Ui;
use crate::ctx::Metrics;
use crate::hit::{Hits, Target};

/// One MR of the review column, opened as a session of its own.
pub(super) fn review(project: String, iid: u64) -> Vec<Command> {
    vec![Command::Session(session::Command::OpenReview {
        project,
        iid,
    })]
}

/// What another surface answers for: the board's own rows, or the rail's.
pub(super) fn elsewhere(
    target: Option<&Target>,
    point: (f32, f32),
    ui: &mut Ui,
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
) -> Option<Vec<Command>> {
    let target = target?;
    board::acted(target, ui, app)
        .or_else(|| rail::acted(target, ui))
        .or_else(|| agent::acted(target, point, ui, app, hits, metrics))
}

/// The rest of the task's actions, under the caret that opened them.
pub(super) fn task_menu(
    ui: &mut Ui,
    hits: &Hits,
    session: groove_types::SessionId,
) -> Vec<Command> {
    let at = hits
        .rect_of(&Target::TaskActions(session.clone()))
        .map(|caret| (caret.x, caret.bottom()))
        .unwrap_or_default();
    ui.menu = Some(crate::Menu {
        at,
        corner: crate::Corner::TopLeft,
        of: crate::Of::Session(session),
    });
    Vec::new()
}

/// The task done at its source, and its session taken away.
pub(super) fn finishing(session: groove_types::SessionId) -> Vec<Command> {
    let finish = groove_controllers::task::Command::Finish { session };
    vec![Command::Task(finish)]
}
