//! What a session did to its own work, written down as it happens.

use groove_types::{SessionId, TimelineEvent, TimelineKind, Timestamp, WorktreeId};

use crate::{AppState, Continuation, Services, Spawner};

/// One line on a session's log, then at the top of its feed.
pub(crate) fn log(
    services: &Services,
    spawner: &dyn Spawner,
    session: &SessionId,
    kind: TimelineKind,
    subject: impl Into<String>,
    worktree: &WorktreeId,
) {
    let payload = serde_json::json!({ "worktree": worktree.as_str() });
    wrote(services, spawner, session, (kind, subject.into()), payload);
}

/// One line on a session's log about the session as a whole.
pub(crate) fn said(
    services: &Services,
    spawner: &dyn Spawner,
    session: &SessionId,
    kind: TimelineKind,
    subject: impl Into<String>,
) {
    let payload = serde_json::json!({});
    wrote(services, spawner, session, (kind, subject.into()), payload);
}

fn wrote(
    services: &Services,
    spawner: &dyn Spawner,
    session: &SessionId,
    (kind, subject): (TimelineKind, String),
    payload: serde_json::Value,
) {
    let line = TimelineEvent {
        session: session.clone(),
        at: Timestamp::now(),
        kind,
        subject,
        payload,
    };
    let service = services.session.clone();
    spawner.spawn(Box::pin(async move {
        let written = service.log(&line).await;
        Box::new(
            move |state: &mut AppState, _: &Services, _: &dyn Spawner| match written {
                Ok(()) => state.session.logged(line),
                Err(e) => state.failed(e),
            },
        ) as Continuation
    }));
}
