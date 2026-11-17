//! A task marked as maintenance or no longer, and the child it spawns, from the palette.

use groove_controllers::session_service::Open;
use groove_controllers::{Command, session};
use groove_types::SessionKind;

use super::Entry;

pub(super) fn entries(open: &Open) -> Vec<Entry> {
    if !matches!(open.session.kind, SessionKind::Task { .. }) {
        return Vec::new();
    }
    let id = open.session.id.clone();
    let on = !open.state.maintenance;
    let label = match on {
        true => "Mark as maintenance task",
        false => "Unmark maintenance task",
    };
    let mark = session::Command::SetMaintenance {
        session: id.clone(),
        on,
    };
    let mut out = vec![Entry::command("Session", label, Command::Session(mark))];
    if open.state.maintenance {
        let spawn = session::Command::SpawnChild { parent: id };
        out.push(Entry::command(
            "Session",
            "Spawn a child session",
            Command::Session(spawn),
        ));
    }
    out
}
