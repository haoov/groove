//! An explorer's rows moved onto the session of a task.

use groove_types::{Session, SessionId, SessionKind, Task, WorktreeId};

use crate::{Error, Result, Store};

/// What every table that names a session is told: the task's row holds it now.
const MOVES: [&str; 9] = [
    "UPDATE session_repos SET session_id = ? WHERE session_id = ?",
    "UPDATE worktrees SET session_id = ? WHERE session_id = ?",
    "UPDATE annotations SET session_id = ? WHERE session_id = ?",
    "UPDATE time_entries SET session_id = ? WHERE session_id = ?",
    "UPDATE time_logs SET session_id = ? WHERE session_id = ?",
    "UPDATE pending_confirmations SET session_id = ? WHERE session_id = ?",
    "UPDATE timeline SET session_id = ? WHERE session_id = ?",
    "UPDATE read_files SET session_id = ? WHERE session_id = ?",
    "UPDATE session_state SET session_id = ? WHERE session_id = ?",
];

/// Where one worktree went: its new branch and its new path.
pub struct Moved {
    pub worktree: WorktreeId,
    pub branch: String,
    pub path: String,
}

impl Store {
    /// The explorer's row replaced by the task's in one transaction, everything it held moved over.
    pub async fn promote(
        &self,
        explorer: &SessionId,
        session: &Session,
        task: &Task,
        moved: &[Moved],
    ) -> Result<()> {
        let SessionKind::Task { external_id } = &session.kind else {
            return Err(wrong("task", &session.id));
        };
        self.remember(task).await?;
        let mut tx = self.db.pool().begin().await?;
        is_explorer(&mut tx, explorer).await?;
        sqlx::query(
            "INSERT INTO sessions (id, kind, title, external_id, created_at)
             VALUES (?, 'task', ?, ?, ?)",
        )
        .bind(session.id.as_str())
        .bind(&session.title)
        .bind(external_id.as_str())
        .bind(session.created_at.seconds())
        .execute(&mut *tx)
        .await?;
        for statement in MOVES {
            sqlx::query(statement)
                .bind(session.id.as_str())
                .bind(explorer.as_str())
                .execute(&mut *tx)
                .await?;
        }
        for one in moved {
            sqlx::query("UPDATE worktrees SET branch = ?, path = ? WHERE id = ?")
                .bind(&one.branch)
                .bind(&one.path)
                .bind(one.worktree.as_str())
                .execute(&mut *tx)
                .await?;
        }
        sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(explorer.as_str())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}

/// Refuses a session that is not an explorer, or not there at all.
async fn is_explorer(conn: &mut sqlx::SqliteConnection, id: &SessionId) -> Result<()> {
    let kind: Option<String> = sqlx::query_scalar(
        "SELECT kind FROM sessions
         WHERE id = ? AND id NOT IN (SELECT session_id FROM routine_sessions)",
    )
    .bind(id.as_str())
    .fetch_optional(conn)
    .await?;
    match kind.as_deref() {
        Some("explorer") => Ok(()),
        _ => Err(wrong("explorer", id)),
    }
}

/// The refusal of a session that is not of the kind a write needs.
pub(crate) fn wrong(expected: &'static str, id: &SessionId) -> Error {
    Error::WrongKind {
        expected,
        id: id.clone(),
    }
}
