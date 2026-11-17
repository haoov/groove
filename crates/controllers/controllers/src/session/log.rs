//! The log of what a session did: its own entries, and those its closed children handed it.

use groove_types::{LogEntry, SessionId, Timestamp};

use crate::asker::Asker;
use crate::{AppState, Continuation, Services, Spawner};

/// What was done, why, and the MR it opened.
#[derive(Debug, Default)]
pub(crate) struct Logged {
    pub what: String,
    pub why: Option<String>,
    pub mr_url: Option<String>,
}

/// One entry on the session's own log.
pub(crate) fn log_work(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    from: &SessionId,
    logged: Logged,
    asker: Asker,
) {
    if state.session.get(from).is_none() {
        return asker.refused(state, crate::tools::NO_SESSION);
    }
    let mut entry = LogEntry {
        id: 0,
        from: None,
        what: logged.what,
        why: logged.why,
        mr_url: logged.mr_url,
        at: Timestamp::now(),
    };
    let (service, to) = (services.session.clone(), from.clone());
    spawner.spawn(Box::pin(async move {
        let done = service.append_log(&to, &entry).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            if let (Ok(id), Some(open)) = (&done, state.session.get_mut(&to)) {
                entry.id = *id;
                open.log.insert(0, entry);
            }
            asker.answer(state, done.map(|_| ()), || format!("logged to {to}"));
        }) as Continuation
    }));
}

/// One entry off the session's log.
pub(crate) fn remove(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    id: &SessionId,
    entry: i64,
) {
    if let Some(open) = state.session.get_mut(id) {
        open.log.retain(|one| one.id != entry);
    }
    let (service, id) = (services.session.clone(), id.clone());
    crate::spawn::record(spawner, async move { service.remove_log(&id, entry).await });
}

/// A closing child's newest entry on its parent's log, named after the child.
pub(crate) fn hand_over(state: &mut AppState, child: &SessionId) {
    let Some(open) = state.session.get(child) else {
        return;
    };
    let Some(parent) = open.state.parent.clone() else {
        return;
    };
    let Some(mut entry) = open.log.first().cloned() else {
        return;
    };
    entry.from.get_or_insert_with(|| open.session.title.clone());
    if let Some(parent) = state.session.get_mut(&parent) {
        let at = parent.log.partition_point(|one| one.at >= entry.at);
        parent.log.insert(at, entry);
    }
}
