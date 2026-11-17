//! A session's log: what it did, written by its agent or by its children.

use groove_types::{LogEntry, SessionId, Timestamp};

use crate::{Result, Store};

/// A `session_log` row as SQLite holds it.
#[derive(sqlx::FromRow)]
struct EntryRow {
    from_child: Option<String>,
    what: String,
    why: Option<String>,
    mr_url: Option<String>,
    at: i64,
}

impl From<EntryRow> for LogEntry {
    fn from(row: EntryRow) -> Self {
        LogEntry {
            from: row.from_child,
            what: row.what,
            why: row.why,
            mr_url: row.mr_url,
            at: Timestamp::new(row.at),
        }
    }
}

impl Store {
    pub async fn append_log(&self, id: &SessionId, entry: &LogEntry) -> Result<()> {
        sqlx::query(
            "INSERT INTO session_log (session_id, from_child, what, why, mr_url, at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id.as_str())
        .bind(&entry.from)
        .bind(&entry.what)
        .bind(&entry.why)
        .bind(&entry.mr_url)
        .bind(entry.at.seconds())
        .execute(self.db.pool())
        .await?;
        Ok(())
    }

    /// The session's log, the newest entry first.
    pub async fn log_of(&self, id: &SessionId) -> Result<Vec<LogEntry>> {
        let rows: Vec<EntryRow> = sqlx::query_as(
            "SELECT from_child, what, why, mr_url, at FROM session_log
             WHERE session_id = ? ORDER BY at DESC, id DESC",
        )
        .bind(id.as_str())
        .fetch_all(self.db.pool())
        .await?;
        Ok(rows.into_iter().map(LogEntry::from).collect())
    }
}
