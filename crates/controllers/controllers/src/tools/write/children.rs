//! What a maintenance task's agent spawns.

use super::Write;
use crate::asker::Asker;
use crate::session::Child;
use crate::{AppState, Services, Spawner};

/// An explorer spawned from the write's own session, started on the brief.
pub(super) fn spawn(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    write: Write,
) {
    let Some(brief) = write.text("brief").map(str::to_string) else {
        return write.reply.failed("spawn_session needs a brief");
    };
    let child = Child {
        title: write.text("title").map(str::to_string),
        brief: Some(brief),
        beside: true,
        ..Child::default()
    };
    let (parent, asker) = (write.session, Asker::Agent(write.reply));
    crate::session::spawn_child(state, services, spawner, &parent, child, asker);
}
