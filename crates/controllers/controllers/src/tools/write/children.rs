//! What a maintenance task's agent spawns, and what a session writes to its log.

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
    };
    let (parent, asker) = (write.session, Asker::Agent(write.reply));
    crate::session::spawn_child(state, services, spawner, &parent, child, asker);
}

/// What the write's own session did, on its log or its parent's.
pub(super) fn log(state: &mut AppState, services: &Services, spawner: &dyn Spawner, write: Write) {
    let Some(what) = write.text("what").map(str::to_string) else {
        return write.reply.failed("log_work needs what was done");
    };
    let logged = crate::session::Logged {
        what,
        why: write.text("why").map(str::to_string),
        mr_url: write.text("mr_url").map(str::to_string),
    };
    let (from, asker) = (write.session, Asker::Agent(write.reply));
    crate::session::log_work(state, services, spawner, &from, logged, asker);
}
