//! What a click on the board does: its rows, its filter, its button.

use groove_controllers::{AppState, Command, delivery, session, task};
use groove_gfx::Rect;
use groove_types::{ExternalId, SessionId};

use crate::hit::Target;
use crate::layout::Layout;
use crate::views::board::{List, plan};
use crate::{Held, Surface, Ui};
use groove_ui_kit::base::ctx::Metrics;
use groove_ui_kit::base::tokens::Tokens;

/// What a click on one of the board's own targets does, if it is one.
pub(super) fn acted(target: &Target, ui: &mut Ui, app: &AppState) -> Option<Vec<Command>> {
    Some(match target {
        Target::Board => board(ui),
        Target::Task(short_id) => task(ui, short_id.clone()),
        Target::Filter => filtering(ui),
        Target::Offer(at) => offered(ui, app, *at),
        Target::AddTask => explorer(ui),
        Target::Place(_) => Vec::new(),
        _ => return None,
    })
}

/// The board, with a read of the sessions and the sources behind it.
pub(super) fn board(ui: &mut Ui) -> Vec<Command> {
    ui.surface = Surface::Board;
    reads()
}

/// What the board stands on, read again whenever it is opened.
pub(crate) fn reads() -> Vec<Command> {
    vec![
        Command::Session(session::Command::List),
        Command::Task(task::Command::Load),
        Command::Delivery(delivery::Command::ReviewQueue),
        Command::Agent(groove_controllers::agent::Command::ListSkills),
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

/// A session picked, wherever it was picked from, with the window back on it.
pub(super) fn opened_session(ui: &mut Ui, session: SessionId) -> Vec<Command> {
    ui.surface = Surface::Session;
    vec![Command::Session(session::Command::Open { session })]
}

/// A press on a task's place takes hold of it.
pub(super) fn takes(ui: &mut Ui, id: ExternalId) -> Vec<Command> {
    ui.held = Some(Held::Task(id));
    ui.board.drop = None;
    Vec::new()
}

/// The dragged row follows the pointer while it stands over the column.
pub(super) fn carried(x: f32, y: f32, ui: &mut Ui, app: &AppState, metrics: Metrics) {
    let ctx = Tokens::new(metrics.scale);
    let layout = Layout::of(metrics, ui);
    let body = next_column(&ctx, layout);
    if !body.contains(x, y) {
        ui.board.drop = None;
        return;
    }
    ui.board.drop = Some(plan::dropped(&ctx, app, ui, body, y));
}

/// The drag let go: the plan takes the row where it landed.
pub fn dropped(ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let landing = plan::landing(app, ui);
    ui.board.drop = None;
    match landing {
        Some(landing) => vec![Command::Task(task::Command::Plan(landing))],
        None => Vec::new(),
    }
}

/// Up next's own room, which a drag is measured against.
fn next_column(tokens: &Tokens, layout: Layout) -> Rect {
    let body = crate::views::board::columns(tokens, layout.board);
    let width = (body.w / List::ALL.len() as f32).floor();
    let top = body.y + tokens.header;
    Rect::new(body.x + width, top, width, body.bottom() - top)
}

/// One MR of the review column, opened as a session of its own.
pub(super) fn review(project: String, iid: u64) -> Vec<Command> {
    vec![Command::Session(session::Command::OpenReview {
        project,
        iid,
    })]
}
