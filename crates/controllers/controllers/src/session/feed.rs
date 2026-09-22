//! The log of every opened session, newest first.

use groove_session_service::FEED_MAX;
use groove_types::{SessionId, TimelineEvent};

use crate::{AppState, Continuation, Services, Spawner};

/// The lines of every session on the rail, read again from the start.
pub(crate) fn read(state: &mut AppState, services: &Services, spawner: &dyn Spawner) {
    let sessions: Vec<SessionId> = state
        .session
        .open
        .iter()
        .map(|open| open.session.id.clone())
        .collect();
    if sessions.is_empty() {
        return state.session.feed.clear();
    }
    let timeline = services.timeline.clone();
    spawner.spawn(Box::pin(async move {
        let mut read: Vec<TimelineEvent> = Vec::new();
        for session in sessions {
            read.extend(timeline.list(&session, FEED_MAX).await.unwrap_or_default());
        }
        read.sort_by_key(|one| std::cmp::Reverse(one.at));
        read.truncate(FEED_MAX);
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.session.feed = read;
        }) as Continuation
    }));
}
