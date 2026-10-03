//! What the agent tells the task's own source: the hours it took, and that it is done.

use groove_types::{ExternalId, SessionKind};

use super::Write;
use crate::asker::Asker;
use crate::{AppState, Services, Spawner};

/// The hours the clock measured and the source has not heard about.
pub(super) fn log_hours(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    write: Write,
) {
    let Some(id) = worked(state, &write) else {
        return write.reply.failed(crate::task::NOT_A_TASK);
    };
    let asker = Asker::Agent(write.reply);
    crate::task::log_hours(state, services, spawner, &id, asker);
}

/// The task done at its source, and its session torn down here.
pub(super) fn finish(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    write: Write,
) {
    let (session, asker) = (write.session, Asker::Agent(write.reply));
    crate::task::finish(state, services, spawner, &session, asker);
}

/// The explorer made the session of the task the write names.
pub(super) fn adopt(state: &mut AppState, spawner: &dyn Spawner, write: Write) {
    let key = match groove_task_service::referenced(write.text("task").unwrap_or_default()) {
        Ok(key) => key,
        Err(e) => return write.reply.failed(e.to_string()),
    };
    let session = write.session.clone();
    crate::task::adopt(state, spawner, &session, key, Asker::Agent(write.reply));
}

/// The task the write's own session works.
fn worked(state: &AppState, write: &Write) -> Option<ExternalId> {
    let open = state.session.get(&write.session)?;
    match &open.session.kind {
        SessionKind::Task { external_id } => Some(external_id.clone()),
        _ => None,
    }
}
