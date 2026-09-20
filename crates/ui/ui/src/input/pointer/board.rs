//! What a click on the board does: its rows, its filter, its button.

use groove_controllers::{AppState, Command, session, task};
use groove_types::SessionId;

use crate::{Surface, Ui};

/// The board, with a read of the sessions and the sources behind it.
pub(super) fn board(ui: &mut Ui) -> Vec<Command> {
    ui.surface = Surface::Board;
    vec![
        Command::Session(session::Command::List),
        Command::Task(task::Command::Load),
    ]
}

/// A task picked on the board: its session opens, and the window goes to it.
pub(super) fn task(ui: &mut Ui, short_id: String) -> Vec<Command> {
    ui.surface = Surface::Session;
    vec![Command::Task(task::Command::Open { short_id })]
}

/// The keyboard into the board's filter.
pub(super) fn filtering(ui: &mut Ui) -> Vec<Command> {
    ui.board.focus();
    Vec::new()
}

/// One row the filter offered, taken into what it holds.
pub(super) fn offered(ui: &mut Ui, app: &AppState, at: usize) -> Vec<Command> {
    let offers = crate::views::board::complete::offers(app, ui.board.filter.text());
    let Some(pick) = offers.get(at) else {
        return Vec::new();
    };
    ui.board.take(pick);
    Vec::new()
}

/// A new explorer, which is how a task is started from the board.
pub(super) fn explorer(ui: &mut Ui) -> Vec<Command> {
    ui.surface = Surface::Session;
    vec![Command::Session(session::Command::OpenExplorer {
        title: None,
    })]
}

/// A live item's worktrees shown under it, or hidden again.
pub(super) fn unfolded(ui: &mut Ui, session: &groove_types::SessionId) -> Vec<Command> {
    ui.board.fold(session);
    Vec::new()
}

/// A session picked, wherever it was picked from, with the window back on it.
pub(super) fn opened_session(ui: &mut Ui, session: SessionId) -> Vec<Command> {
    ui.surface = Surface::Session;
    vec![Command::Session(session::Command::Open { session })]
}
