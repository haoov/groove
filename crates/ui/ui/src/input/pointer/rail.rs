//! A click on the rail's own parts: a session opened on its ask, a window folded, the feed narrowed.

use groove_controllers::{AppState, Command, agent};

use crate::Ui;
use crate::hit::Target;

/// The rail's own targets; anything else is not its to answer.
pub(super) fn acted(target: &Target, ui: &mut Ui, app: &AppState) -> Option<Vec<Command>> {
    match target {
        Target::Feed => Some(folded_feed(ui)),
        Target::SessionsFold => {
            ui.rail.stowed = !ui.rail.stowed;
            Some(Vec::new())
        }
        Target::FeedScope(mine) => Some(narrowed_feed(ui, *mine)),
        Target::FeedRun(key) => {
            ui.rail.open = (ui.rail.open != Some(*key)).then_some(*key);
            Some(Vec::new())
        }
        Target::RoutineRun(id) => {
            let run = agent::Command::RunRoutine { id: id.clone() };
            Some(vec![Command::Agent(run)])
        }
        Target::Routines => {
            ui.rail.routines = !ui.rail.routines;
            Some(Vec::new())
        }
        Target::Session(session) => {
            let opened = super::opened_session(ui, session.clone());
            let ask = app.agent.activity(session).and_then(|one| one.asks.first());
            if let Some(ask) = ask {
                ui.examine(ask.id.clone());
            }
            Some(opened)
        }
        Target::FeedLine(session) => Some(super::opened_session(ui, session.clone())),
        Target::Approve(id) => Some(answered(ui, agent::Command::Approve { id: id.clone() })),
        Target::Refuse(id) => Some(answered(ui, agent::Command::Refuse { id: id.clone() })),
        Target::Sheet => Some(Vec::new()),
        Target::SheetView(view) => {
            if let Some(sheet) = ui.sheet_mut() {
                (sheet.view, sheet.scroll, sheet.across) = (*view, 0.0, 0.0);
            }
            Some(Vec::new())
        }
        _ => None,
    }
}

/// A write decided, and the sheet that showed it put away.
fn answered(ui: &mut Ui, answer: agent::Command) -> Vec<Command> {
    ui.close(|one| matches!(one, crate::Overlay::Examining(_)));
    vec![Command::Agent(answer)]
}

/// The feed folded to its own heading, or opened again.
fn folded_feed(ui: &mut Ui) -> Vec<Command> {
    ui.rail.folded = !ui.rail.folded;
    ui.rail.feed = 0.0;
    Vec::new()
}

/// The feed on every session, or on the one in hand.
fn narrowed_feed(ui: &mut Ui, mine: bool) -> Vec<Command> {
    ui.rail.mine = mine;
    ui.rail.feed = 0.0;
    Vec::new()
}
