//! The order the user gave the tasks waiting.

#[cfg(test)]
mod tests;

use groove_db::Db;
pub use groove_db::Store as Stored;
use groove_types::{Error, ExternalId, Result};

/// One task's place in the order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placed {
    pub external_id: ExternalId,
}

impl Placed {
    pub fn new(external_id: ExternalId) -> Self {
        Self { external_id }
    }
}

/// The plan on disk, cheap to clone: a handle on the pool.
#[derive(Clone)]
pub struct Plan {
    db: Db,
}

impl groove_db::Store for Plan {
    fn new(db: Db) -> Self {
        Self { db }
    }

    fn db(&self) -> &Db {
        &self.db
    }
}

impl Plan {
    /// Every placed task, in the order the user gave it.
    pub async fn order(&self) -> Result<Vec<Placed>> {
        let rows: Vec<(String,)> =
            sqlx::query_as("SELECT external_id FROM plan ORDER BY at, external_id")
                .fetch_all(self.db.pool())
                .await
                .map_err(failed)?;
        Ok(rows
            .into_iter()
            .map(|(id,)| Placed::new(ExternalId::new(id)))
            .collect())
    }

    /// The whole order again, in one transaction.
    pub async fn save(&self, order: &[Placed]) -> Result<()> {
        let mut tx = self.db.pool().begin().await.map_err(failed)?;
        sqlx::query("DELETE FROM plan")
            .execute(&mut *tx)
            .await
            .map_err(failed)?;
        for (at, placed) in order.iter().enumerate() {
            sqlx::query("INSERT INTO plan (external_id, at) VALUES (?, ?)")
                .bind(placed.external_id.as_str())
                .bind(at as i64)
                .execute(&mut *tx)
                .await
                .map_err(failed)?;
        }
        tx.commit().await.map_err(failed)
    }
}

fn failed(source: sqlx::Error) -> Error {
    Error::store("plan", source)
}
