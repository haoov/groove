//! The feed: the lines of every session on the rail, newest first.

use groove_types::SessionId;

use crate::{AppState, Continuation, Services, Spawner};

/// The feed read again for the sessions now on the rail.
pub(crate) fn read(state: &AppState, services: &Services, spawner: &dyn Spawner) {
    let sessions: Vec<SessionId> = state
        .session
        .open
        .iter()
        .map(|open| open.session.id.clone())
        .collect();
    let service = services.session.clone();
    spawner.spawn(Box::pin(async move {
        let read = service.feed(&sessions).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.session.feed = read;
        }) as Continuation
    }));
}
