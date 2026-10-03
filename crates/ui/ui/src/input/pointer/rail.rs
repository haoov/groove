//! A click on the rail's own parts: the feed folded or narrowed, and the agent's asks answered.

use groove_controllers::{Command, agent};

use crate::Ui;
use crate::hit::Target;

/// The rail's own targets; anything else is not its to answer.
pub(super) fn acted(target: &Target, ui: &mut Ui) -> Option<Vec<Command>> {
    match target {
        Target::Feed => Some(folded_feed(ui)),
        Target::FeedScope => Some(narrowed_feed(ui)),
        Target::RoutineRun(id) => {
            let run = agent::Command::RunRoutine { id: id.clone() };
            Some(vec![Command::Agent(run)])
        }
        Target::Routines => {
            ui.rail.routines = !ui.rail.routines;
            Some(Vec::new())
        }
        Target::Session(session) | Target::FeedLine(session) => {
            Some(super::opened_session(ui, session.clone()))
        }
        Target::Approve(id) => Some(answered(ui, agent::Command::Approve { id: id.clone() })),
        Target::Refuse(id) => Some(answered(ui, agent::Command::Refuse { id: id.clone() })),
        Target::Sheet => Some(Vec::new()),
        Target::Examine(id) => {
            ui.overlay = Some(crate::Overlay::Examining(id.clone()));
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
fn narrowed_feed(ui: &mut Ui) -> Vec<Command> {
    ui.rail.mine = !ui.rail.mine;
    ui.rail.feed = 0.0;
    Vec::new()
}
