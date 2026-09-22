//! What a session did to its own work, written down as it happens.

use groove_types::{SessionId, TimelineEvent, TimelineKind, Timestamp};

use crate::{AppState, Continuation, Services, Spawner};

/// One line on the selected session's log; a write with no session behind it is none.
pub(crate) fn log(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    kind: TimelineKind,
    subject: impl Into<String>,
) {
    let Some(session) = state.session.selected.clone() else {
        return;
    };
    logged(services, spawner, session, kind, subject, payload(state));
}

/// The same, for a session the caller names.
pub(crate) fn logged(
    services: &Services,
    spawner: &dyn Spawner,
    session: SessionId,
    kind: TimelineKind,
    subject: impl Into<String>,
    payload: serde_json::Value,
) {
    let event = TimelineEvent {
        session,
        at: Timestamp::now(),
        kind,
        subject: subject.into(),
        payload,
    };
    let timeline = services.timeline.clone();
    spawner.spawn(Box::pin(async move {
        let written = timeline.append(&event).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            if let Err(e) = written {
                state.errors.push(e);
            }
        }) as Continuation
    }));
}

/// Which worktree the line belongs to, for a reader that wants to go there.
fn payload(state: &AppState) -> serde_json::Value {
    match crate::workspace::selected(state) {
        Some(worktree) => serde_json::json!({ "worktree": worktree.as_str() }),
        None => serde_json::Value::Null,
    }
}
