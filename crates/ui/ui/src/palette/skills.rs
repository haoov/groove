//! The skills the agent's bar offers, sent from the palette.

use groove_controllers::session_service::Open;
use groove_controllers::{AppState, Command, agent};

use super::Entry;

pub(super) fn entries(app: &AppState, open: &Open) -> Vec<Entry> {
    let sources = groove_controllers::task_service::source_ids(app.config.config.as_ref());
    let offers = app.skill.offers(&open.session.kind, &sources).into_iter();
    offers
        .map(|one| {
            let send = agent::Command::SendSkill {
                session: open.session.id.clone(),
                id: one.id,
                args: one.args,
            };
            Entry::command("Skills", one.label, Command::Agent(send))
        })
        .collect()
}
