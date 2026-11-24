//! The `session_state` row: what a session remembers between runs.

use groove_types::{Section, Session, SessionId, SessionState, Timestamp, WorktreeId};

use crate::rows::SessionRow;
use crate::{Result, Store};

/// A `session_state` row, defaults where the row is missing.
#[derive(sqlx::FromRow)]
struct StateRow {
    opened_at: Option<i64>,
    seen_at: Option<i64>,
    auto_approve: bool,
    selected_worktree_id: Option<String>,
    maintenance: bool,
    parent_id: Option<String>,
    section: String,
    rank: Option<i64>,
}

impl From<StateRow> for SessionState {
    fn from(row: StateRow) -> Self {
        SessionState {
            opened_at: row.opened_at.map(Timestamp::new),
            seen_at: row.seen_at.map(Timestamp::new),
            auto_approve: row.auto_approve,
            selected_worktree: row.selected_worktree_id.map(WorktreeId::new),
            maintenance: row.maintenance,
            parent: row.parent_id.map(SessionId::new),
            section: Section::parse(&row.section),
            rank: row.rank,
        }
    }
}

impl Store {
    pub async fn state(&self, id: &SessionId) -> Result<SessionState> {
        let row: Option<StateRow> = sqlx::query_as(
            "SELECT opened_at, seen_at, auto_approve, selected_worktree_id, maintenance, parent_id,
                    section, rank
             FROM session_state WHERE session_id = ?",
        )
        .bind(id.as_str())
        .fetch_optional(self.db.pool())
        .await?;
        Ok(row.map(SessionState::from).unwrap_or_default())
    }

    /// `Some(now)` puts the session on top of the rail's active section; `None` takes it off.
    pub async fn set_opened(&self, id: &SessionId, at: Option<Timestamp>) -> Result<()> {
        self.upsert_state(Column::OpenedAt, at.map(|t| t.seconds()), id)
            .await?;
        if at.is_some() {
            self.set_places(&[(id.clone(), Section::Active, None)])
                .await?;
        }
        Ok(())
    }

    /// Each session's section and rank, written together.
    pub async fn set_places(&self, places: &[(SessionId, Section, Option<i64>)]) -> Result<()> {
        let mut tx = self.db.pool().begin().await?;
        for (id, section, rank) in places {
            sqlx::query(
                "INSERT INTO session_state (session_id, section, rank) VALUES (?, ?, ?)
                 ON CONFLICT(session_id) DO UPDATE SET section = excluded.section, rank = excluded.rank",
            )
            .bind(id.as_str())
            .bind(section.as_str())
            .bind(rank)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn set_seen(&self, id: &SessionId, at: Timestamp) -> Result<()> {
        self.upsert_state(Column::SeenAt, Some(at.seconds()), id)
            .await
    }

    pub async fn set_auto_approve(&self, id: &SessionId, on: bool) -> Result<()> {
        self.upsert_state(Column::AutoApprove, Some(i64::from(on)), id)
            .await
    }

    pub async fn set_maintenance(&self, id: &SessionId, on: bool) -> Result<()> {
        self.upsert_state(Column::Maintenance, Some(i64::from(on)), id)
            .await
    }

    pub async fn set_parent(&self, id: &SessionId, parent: &SessionId) -> Result<()> {
        sqlx::query(
            "INSERT INTO session_state (session_id, parent_id) VALUES (?, ?)
             ON CONFLICT(session_id) DO UPDATE SET parent_id = excluded.parent_id",
        )
        .bind(id.as_str())
        .bind(parent.as_str())
        .execute(self.db.pool())
        .await?;
        Ok(())
    }

    pub async fn set_selected_worktree(
        &self,
        id: &SessionId,
        worktree: Option<&WorktreeId>,
    ) -> Result<()> {
        let sql = "INSERT INTO session_state (session_id, selected_worktree_id) VALUES (?, ?)
                   ON CONFLICT(session_id) DO UPDATE SET selected_worktree_id = excluded.selected_worktree_id";
        sqlx::query(sql)
            .bind(id.as_str())
            .bind(worktree.map(|w| w.as_str()))
            .execute(self.db.pool())
            .await?;
        Ok(())
    }

    /// The sessions on the rail, in the order they were opened.
    pub async fn opened(&self) -> Result<Vec<(Session, SessionState)>> {
        let rows: Vec<SessionRow> = sqlx::query_as(
            "SELECT s.id, s.kind, s.title, s.external_id, s.review_project, s.review_iid,
                    s.created_at
             FROM sessions s JOIN session_state st ON st.session_id = s.id
             WHERE st.opened_at IS NOT NULL ORDER BY st.opened_at, s.id",
        )
        .fetch_all(self.db.pool())
        .await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let session = Session::from(row);
            let state = self.state(&session.id).await?;
            out.push((session, state));
        }
        Ok(out)
    }

    /// Every session on disk, the most recently opened first.
    pub async fn living(&self) -> Result<Vec<(Session, SessionState)>> {
        let rows: Vec<SessionRow> = sqlx::query_as(
            "SELECT s.id, s.kind, s.title, s.external_id, s.review_project, s.review_iid,
                    s.created_at
             FROM sessions s
             LEFT JOIN session_state st ON st.session_id = s.id
             ORDER BY COALESCE(st.opened_at, s.created_at) DESC, s.id",
        )
        .fetch_all(self.db.pool())
        .await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let session = Session::from(row);
            let state = self.state(&session.id).await?;
            out.push((session, state));
        }
        Ok(out)
    }

    /// One integer column, the row created when missing.
    async fn upsert_state(&self, column: Column, value: Option<i64>, id: &SessionId) -> Result<()> {
        let sql = match column {
            Column::OpenedAt => {
                "INSERT INTO session_state (session_id, opened_at) VALUES (?, ?)
                 ON CONFLICT(session_id) DO UPDATE SET opened_at = excluded.opened_at"
            }
            Column::SeenAt => {
                "INSERT INTO session_state (session_id, seen_at) VALUES (?, ?)
                 ON CONFLICT(session_id) DO UPDATE SET seen_at = excluded.seen_at"
            }
            Column::AutoApprove => {
                "INSERT INTO session_state (session_id, auto_approve) VALUES (?, ?)
                 ON CONFLICT(session_id) DO UPDATE SET auto_approve = excluded.auto_approve"
            }
            Column::Maintenance => {
                "INSERT INTO session_state (session_id, maintenance) VALUES (?, ?)
                 ON CONFLICT(session_id) DO UPDATE SET maintenance = excluded.maintenance"
            }
        };
        sqlx::query(sql)
            .bind(id.as_str())
            .bind(value)
            .execute(self.db.pool())
            .await?;
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum Column {
    OpenedAt,
    SeenAt,
    AutoApprove,
    Maintenance,
}
