//! The one MR a worktree has, as the database holds it.

#[cfg(test)]
mod tests;

use groove_db::Db;
pub use groove_db::Store as Stored;
use groove_types::{Error, Forge, Mr, MrId, MrState, Result, WorktreeId};

/// What a forge answered of an MR, as the row keeps it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answered {
    pub forge: Forge,
    pub number: String,
    pub url: String,
    pub state: MrState,
}

#[derive(sqlx::FromRow)]
struct Row {
    id: String,
    worktree_id: String,
    platform: String,
    remote_id: String,
    url: String,
    state: String,
}

impl TryFrom<Row> for Mr {
    type Error = Error;

    /// A row whose forge or state no version wrote is an error.
    fn try_from(row: Row) -> Result<Self> {
        let state = MrState::parse(&row.state)
            .ok_or_else(|| Error::db(format!("an MR row holds an unknown state {}", row.state)))?;
        Ok(Mr {
            id: MrId::new(row.id),
            worktree: WorktreeId::new(row.worktree_id),
            forge: Forge::parse(&row.platform)?,
            remote_id: row.remote_id,
            url: row.url,
            state,
        })
    }
}

/// The MR rows, cheap to clone: a handle on the pool.
#[derive(Clone)]
pub struct Store {
    db: Db,
}

impl groove_db::Store for Store {
    fn new(db: Db) -> Self {
        Self { db }
    }

    fn db(&self) -> &Db {
        &self.db
    }
}

impl Store {
    /// The MR this worktree has, if it has one.
    pub async fn get(&self, worktree: &WorktreeId) -> Result<Option<Mr>> {
        let row: Option<Row> = sqlx::query_as(
            "SELECT id, worktree_id, platform, remote_id, url, state
             FROM mrs WHERE worktree_id = ?",
        )
        .bind(worktree.as_str())
        .fetch_optional(self.db.pool())
        .await
        .map_err(failed)?;
        row.map(Mr::try_from).transpose()
    }

    /// Every open MR, whatever worktree it belongs to.
    pub async fn open(&self) -> Result<Vec<Mr>> {
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT id, worktree_id, platform, remote_id, url, state
             FROM mrs WHERE state = 'open' ORDER BY worktree_id",
        )
        .fetch_all(self.db.pool())
        .await
        .map_err(failed)?;
        rows.into_iter().map(Mr::try_from).collect()
    }

    /// What the forge answered, written down as the worktree's one MR.
    pub async fn save(&self, worktree: &WorktreeId, answered: &Answered) -> Result<Mr> {
        let mut tx = self.db.pool().begin().await.map_err(failed)?;
        sqlx::query("DELETE FROM mrs WHERE worktree_id = ? AND remote_id != ?")
            .bind(worktree.as_str())
            .bind(&answered.number)
            .execute(&mut *tx)
            .await
            .map_err(failed)?;
        sqlx::query(
            "INSERT INTO mrs (id, worktree_id, platform, remote_id, url, state)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(worktree_id, remote_id)
               DO UPDATE SET url = excluded.url, state = excluded.state",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(worktree.as_str())
        .bind(answered.forge.as_str())
        .bind(&answered.number)
        .bind(&answered.url)
        .bind(answered.state.as_str())
        .execute(&mut *tx)
        .await
        .map_err(failed)?;
        tx.commit().await.map_err(failed)?;
        self.get(worktree)
            .await?
            .ok_or_else(|| Error::db(format!("{worktree} lost its merge request")))
    }

    /// Forgets the worktree's MR, for one closed or a branch that lost it.
    pub async fn remove(&self, worktree: &WorktreeId) -> Result<()> {
        sqlx::query("DELETE FROM mrs WHERE worktree_id = ?")
            .bind(worktree.as_str())
            .execute(self.db.pool())
            .await
            .map_err(failed)?;
        Ok(())
    }
}

fn failed(source: sqlx::Error) -> Error {
    Error::store("MR rows", source)
}
