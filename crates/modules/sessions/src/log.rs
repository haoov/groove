//! A session's log: what it did, written by its agent or by its children.

use groove_types::{LogEntry, SessionId, Timestamp};

use crate::{Result, Store};

/// A `session_log` row as SQLite holds it.
#[derive(sqlx::FromRow)]
struct EntryRow {
    id: i64,
    from_child: Option<String>,
    what: String,
    why: Option<String>,
    mr_url: Option<String>,
    at: i64,
}

impl From<EntryRow> for LogEntry {
    fn from(row: EntryRow) -> Self {
        LogEntry {
            id: row.id,
            from: row.from_child,
            what: row.what,
            why: row.why,
            mr_url: row.mr_url,
            at: Timestamp::new(row.at),
        }
    }
}

impl Store {
    /// The entry stored; its row id.
    pub async fn append_log(&self, id: &SessionId, entry: &LogEntry) -> Result<i64> {
        let done = sqlx::query(
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
        Ok(done.last_insert_rowid())
    }

    pub async fn remove_log(&self, id: &SessionId, entry: i64) -> Result<()> {
        sqlx::query("DELETE FROM session_log WHERE session_id = ? AND id = ?")
            .bind(id.as_str())
            .bind(entry)
            .execute(self.db.pool())
            .await?;
        Ok(())
    }

    /// The session's log, the newest entry first.
    pub async fn log_of(&self, id: &SessionId) -> Result<Vec<LogEntry>> {
        let rows: Vec<EntryRow> = sqlx::query_as(
            "SELECT id, from_child, what, why, mr_url, at FROM session_log
             WHERE session_id = ? ORDER BY at DESC, id DESC",
        )
        .bind(id.as_str())
        .fetch_all(self.db.pool())
        .await?;
        Ok(rows.into_iter().map(LogEntry::from).collect())
    }

    /// A child's newest entry moved to its parent's log, named after the child.
    pub async fn hand_log_over(&self, child: &SessionId) -> Result<()> {
        sqlx::query(
            "UPDATE session_log
             SET session_id = (SELECT parent_id FROM session_state WHERE session_id = ?1),
                 from_child = COALESCE(from_child, (SELECT title FROM sessions WHERE id = ?1))
             WHERE id = (SELECT id FROM session_log WHERE session_id = ?1
                         ORDER BY at DESC, id DESC LIMIT 1)
               AND EXISTS (SELECT 1 FROM session_state
                           WHERE session_id = ?1 AND parent_id IS NOT NULL)",
        )
        .bind(child.as_str())
        .execute(self.db.pool())
        .await?;
        Ok(())
    }
}
