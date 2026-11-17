//! The sessions a maintenance task spawned.

use groove_types::{Session, SessionId};

use crate::rows::SessionRow;
use crate::{Result, Store};

impl Store {
    /// The sessions this one spawned, the oldest first.
    pub async fn children(&self, parent: &SessionId) -> Result<Vec<Session>> {
        let rows: Vec<SessionRow> = sqlx::query_as(
            "SELECT s.id, s.kind, s.title, s.external_id, s.review_project, s.review_iid,
                    s.created_at, r.routine
             FROM sessions s JOIN session_state st ON st.session_id = s.id
             LEFT JOIN routine_sessions r ON r.session_id = s.id
             WHERE st.parent_id = ? ORDER BY s.created_at, s.id",
        )
        .bind(parent.as_str())
        .fetch_all(self.db.pool())
        .await?;
        Ok(rows.into_iter().map(Session::from).collect())
    }
}
