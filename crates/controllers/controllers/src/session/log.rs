//! The log of what a session did: its own entries, and those its children send it.

use groove_types::{LogEntry, SessionId, Timestamp};

use crate::asker::Asker;
use crate::{AppState, Continuation, Services, Spawner};

/// What was done, why, and the MR that carries it.
#[derive(Debug, Default)]
pub(crate) struct Logged {
    pub what: String,
    pub why: Option<String>,
    pub mr_url: Option<String>,
}

/// One entry on the session's log, or on its parent's when it has one.
pub(crate) fn log_work(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    from: &SessionId,
    logged: Logged,
    asker: Asker,
) {
    let Some(open) = state.session.get(from) else {
        return asker.refused(state, crate::tools::NO_SESSION);
    };
    let (to, child) = match open.state.parent.clone() {
        Some(parent) => (parent, Some(open.session.title.clone())),
        None => (from.clone(), None),
    };
    let entry = LogEntry {
        from: child,
        what: logged.what,
        why: logged.why,
        mr_url: logged.mr_url,
        at: Timestamp::now(),
    };
    if let Some(open) = state.session.get_mut(&to) {
        open.log.insert(0, entry.clone());
    }
    let service = services.session.clone();
    spawner.spawn(Box::pin(async move {
        let done = service.append_log(&to, &entry).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            asker.answer(state, done, || format!("logged to {to}"));
        }) as Continuation
    }));
}
