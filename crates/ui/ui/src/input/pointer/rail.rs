//! What a click on the rail's own parts does: the feed folded, or narrowed.
//! The agent's own asks answer here too.

use groove_controllers::{Command, agent};

use crate::Ui;
use crate::hit::Target;

/// The rail's own targets; anything else is not its to answer.
pub(super) fn acted(target: &Target, ui: &mut Ui) -> Option<Vec<Command>> {
    match target {
        Target::Feed => Some(folded_feed(ui)),
        Target::FeedScope => Some(narrowed_feed(ui)),
        Target::Session(session) | Target::FeedLine(session) => {
            Some(super::opened_session(ui, session.clone()))
        }
        Target::Approve(id) => Some(vec![Command::Agent(agent::Command::Approve {
            id: id.clone(),
        })]),
        Target::Refuse(id) => Some(vec![Command::Agent(agent::Command::Refuse {
            id: id.clone(),
        })]),
        _ => None,
    }
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
