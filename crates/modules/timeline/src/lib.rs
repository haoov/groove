//! What happened on a session: one line appended as it happens, read back newest first.

#[cfg(test)]
mod tests;

use groove_db::Db;
pub use groove_db::Store as Stored;
use groove_types::{Error, Result, SessionId, TimelineEvent, TimelineKind, Timestamp};

/// One line as the table holds it.
#[derive(sqlx::FromRow)]
struct Row {
    session_id: String,
    at: i64,
    kind: String,
    subject: String,
    payload: String,
}

impl Row {
    /// The line it holds, or nothing for a kind this version does not know.
    fn event(self) -> Option<TimelineEvent> {
        Some(TimelineEvent {
            session: SessionId::new(self.session_id),
            at: Timestamp::new(self.at),
            kind: TimelineKind::parse(&self.kind)?,
            subject: self.subject,
            payload: serde_json::from_str(&self.payload).unwrap_or(serde_json::Value::Null),
        })
    }
}

/// The lines of the log, cheap to clone: a handle on the pool.
#[derive(Clone)]
pub struct Timeline {
    db: Db,
}

impl groove_db::Store for Timeline {
    fn new(db: Db) -> Self {
        Self { db }
    }

    fn db(&self) -> &Db {
        &self.db
    }
}

impl Timeline {
    /// One line written down, whatever else the session has.
    pub async fn append(&self, event: &TimelineEvent) -> Result<()> {
        sqlx::query(
            "INSERT INTO timeline (session_id, at, kind, subject, payload)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(event.session.as_str())
        .bind(event.at.seconds())
        .bind(event.kind.word())
        .bind(&event.subject)
        .bind(event.payload.to_string())
        .execute(self.db.pool())
        .await
        .map_err(failed)?;
        Ok(())
    }

    /// The newest lines of one session, at most `limit` of them.
    pub async fn list(&self, session: &SessionId, limit: usize) -> Result<Vec<TimelineEvent>> {
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT session_id, at, kind, subject, payload FROM timeline
             WHERE session_id = ? ORDER BY at DESC, id DESC LIMIT ?",
        )
        .bind(session.as_str())
        .bind(i64::try_from(limit).unwrap_or(i64::MAX))
        .fetch_all(self.db.pool())
        .await
        .map_err(failed)?;
        Ok(rows.into_iter().filter_map(Row::event).collect())
    }
}

fn failed(source: sqlx::Error) -> Error {
    Error::store("log lines", source)
}
