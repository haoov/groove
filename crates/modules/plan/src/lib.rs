//! The order the user gave the tasks waiting, and the divider they stand either side of.

#[cfg(test)]
mod tests;

use groove_db::Db;
pub use groove_db::Store as Stored;
use groove_types::{Error, ExternalId, Result};

/// One task's place: where it stands, and whether it stands under the divider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placed {
    pub external_id: ExternalId,
    pub later: bool,
}

impl Placed {
    pub fn now(external_id: ExternalId) -> Self {
        Self {
            external_id,
            later: false,
        }
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
        let rows: Vec<(String, i64)> =
            sqlx::query_as("SELECT external_id, later FROM plan ORDER BY at, external_id")
                .fetch_all(self.db.pool())
                .await
                .map_err(failed)?;
        Ok(rows
            .into_iter()
            .map(|(id, later)| Placed {
                external_id: ExternalId::new(id),
                later: later != 0,
            })
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
            sqlx::query("INSERT INTO plan (external_id, at, later) VALUES (?, ?, ?)")
                .bind(placed.external_id.as_str())
                .bind(at as i64)
                .bind(i64::from(placed.later))
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
