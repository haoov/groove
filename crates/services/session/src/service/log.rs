//! What each session did to its own work, as its log holds it.

use groove_types::{Error, SessionId, TimelineEvent};

use super::Service;
use crate::FEED_MAX;

impl Service {
    /// One line written down.
    pub async fn log(&self, event: &TimelineEvent) -> Result<(), Error> {
        self.timeline.append(event).await
    }

    /// The newest lines of one session, at most `limit` of them.
    pub async fn lines(
        &self,
        session: &SessionId,
        limit: usize,
    ) -> Result<Vec<TimelineEvent>, Error> {
        self.timeline.list(session, limit).await
    }

    /// The feed of these sessions: their newest lines together, newest first.
    pub async fn feed(&self, sessions: &[SessionId]) -> Result<Vec<TimelineEvent>, Error> {
        let mut read = Vec::new();
        for session in sessions {
            read.extend(self.lines(session, FEED_MAX).await?);
        }
        read.sort_by_key(|one| std::cmp::Reverse(one.at));
        read.truncate(FEED_MAX);
        Ok(read)
    }
}
