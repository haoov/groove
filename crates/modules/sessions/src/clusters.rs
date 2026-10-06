//! The cluster contexts a session holds, each on a namespace or the whole cluster.

use groove_types::{Attached, SessionId, Timestamp};

use crate::{Result, Store};

impl Store {
    /// Attaching the same context and namespace again changes nothing.
    pub async fn attach_cluster(
        &self,
        id: &SessionId,
        attached: &Attached,
        now: Timestamp,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO session_clusters (session_id, context, namespace, added_at)
             VALUES (?, ?, ?, ?)
             ON CONFLICT(session_id, context, namespace) DO NOTHING",
        )
        .bind(id.as_str())
        .bind(&attached.context)
        .bind(attached.namespace.as_deref().unwrap_or_default())
        .bind(now.seconds())
        .execute(self.db.pool())
        .await?;
        Ok(())
    }

    pub async fn detach_cluster(&self, id: &SessionId, attached: &Attached) -> Result<()> {
        sqlx::query(
            "DELETE FROM session_clusters WHERE session_id = ? AND context = ? AND namespace = ?",
        )
        .bind(id.as_str())
        .bind(&attached.context)
        .bind(attached.namespace.as_deref().unwrap_or_default())
        .execute(self.db.pool())
        .await?;
        Ok(())
    }

    /// In the order they were attached.
    pub async fn clusters_of(&self, id: &SessionId) -> Result<Vec<Attached>> {
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT context, namespace FROM session_clusters
             WHERE session_id = ? ORDER BY added_at, context, namespace",
        )
        .bind(id.as_str())
        .fetch_all(self.db.pool())
        .await?;
        Ok(rows
            .into_iter()
            .map(|(context, namespace)| Attached {
                context,
                namespace: (!namespace.is_empty()).then_some(namespace),
            })
            .collect())
    }
}
