//! The sessions a maintenance task spawned, and the alert each one was spawned for.

use groove_types::{AlertChild, Session, SessionId};

use crate::rows::SessionRow;
use crate::{Result, Store};

impl Store {
    /// The sessions this one spawned, the oldest first.
    pub async fn children(&self, parent: &SessionId) -> Result<Vec<Session>> {
        let rows: Vec<SessionRow> = sqlx::query_as(
            "SELECT s.id, s.kind, s.title, s.external_id, s.review_project, s.review_iid,
                    s.created_at
             FROM sessions s JOIN session_state st ON st.session_id = s.id
             WHERE st.parent_id = ? ORDER BY s.created_at, s.id",
        )
        .bind(parent.as_str())
        .fetch_all(self.db.pool())
        .await?;
        Ok(rows.into_iter().map(Session::from).collect())
    }
}

/// An `alert_sessions` row as SQLite holds it.
#[derive(sqlx::FromRow)]
struct AlertRow {
    session_id: String,
    parent_id: String,
    dedup_key: String,
    title: String,
    priority: Option<String>,
    cluster: Option<String>,
    namespace: Option<String>,
}

impl From<AlertRow> for AlertChild {
    fn from(row: AlertRow) -> Self {
        AlertChild {
            session: SessionId::new(row.session_id),
            parent: SessionId::new(row.parent_id),
            dedup_key: row.dedup_key,
            title: row.title,
            priority: row.priority,
            cluster: row.cluster,
            namespace: row.namespace,
        }
    }
}

impl Store {
    pub async fn remember_alert(&self, child: &AlertChild) -> Result<()> {
        sqlx::query(
            "INSERT INTO alert_sessions
                (session_id, parent_id, dedup_key, title, priority, cluster, namespace)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(child.session.as_str())
        .bind(child.parent.as_str())
        .bind(&child.dedup_key)
        .bind(&child.title)
        .bind(&child.priority)
        .bind(&child.cluster)
        .bind(&child.namespace)
        .execute(self.db.pool())
        .await?;
        Ok(())
    }

    /// The children this session spawned for alerts.
    pub async fn alert_children(&self, parent: &SessionId) -> Result<Vec<AlertChild>> {
        let rows: Vec<AlertRow> = sqlx::query_as(
            "SELECT session_id, parent_id, dedup_key, title, priority, cluster, namespace
             FROM alert_sessions WHERE parent_id = ? ORDER BY rowid",
        )
        .bind(parent.as_str())
        .fetch_all(self.db.pool())
        .await?;
        Ok(rows.into_iter().map(AlertChild::from).collect())
    }
}
