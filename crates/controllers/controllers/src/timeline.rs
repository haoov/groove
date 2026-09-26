//! What a session did to its own work, written down as it happens.

use groove_types::{SessionId, TimelineEvent, TimelineKind, Timestamp};

use crate::{AppState, Continuation, Services, Spawner};

/// One line on a session's log.
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
        Box::new(
            move |state: &mut AppState, services: &Services, spawner: &dyn Spawner| match written {
                Err(e) => state.failed(e),
                Ok(()) => crate::session::feed::read(state, services, spawner),
            },
        ) as Continuation
    }));
}
