//! The one MR a worktree has, as the database holds it.

use groove_db::Db;
use groove_types::{Forge, Mr, MrId, MrState, WorktreeId};

use crate::{Error, Result, Snapshot};

#[derive(sqlx::FromRow)]
struct Row {
    id: String,
    worktree_id: String,
    platform: String,
    remote_id: String,
    url: String,
    state: String,
}

impl From<Row> for Mr {
    fn from(row: Row) -> Self {
        Mr {
            id: MrId::new(row.id),
            worktree: WorktreeId::new(row.worktree_id),
            forge: Forge::parse(&row.platform).unwrap_or(Forge::Github),
            remote_id: row.remote_id,
            url: row.url,
            state: state_of(&row.state),
        }
    }
}

/// The MR rows, cheap to clone: a handle on the pool.
#[derive(Clone)]
pub struct Store {
    db: Db,
}

impl Store {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// A store on a private in-memory database, for tests up the stack.
    pub async fn in_memory() -> Result<Self> {
        let db = Db::in_memory()
            .await
            .map_err(|e| Error::Invalid(format!("no database: {e}")))?;
        Ok(Self::new(db))
    }

    /// The pool this store writes to, for the modules that share it.
    pub fn db(&self) -> &Db {
        &self.db
    }

    /// The MR this worktree has, if it has one.
    pub async fn get(&self, worktree: &WorktreeId) -> Result<Option<Mr>> {
        let row: Option<Row> = sqlx::query_as(
            "SELECT id, worktree_id, platform, remote_id, url, state
             FROM mrs WHERE worktree_id = ?",
        )
        .bind(worktree.as_str())
        .fetch_optional(self.db.pool())
        .await?;
        Ok(row.map(Mr::from))
    }

    /// What the forge answered, written down. A worktree keeps one MR, so any other
    /// row of it goes.
    pub async fn save(&self, worktree: &WorktreeId, forge: Forge, read: &Snapshot) -> Result<Mr> {
        let mut tx = self.db.pool().begin().await?;
        sqlx::query("DELETE FROM mrs WHERE worktree_id = ? AND remote_id != ?")
            .bind(worktree.as_str())
            .bind(&read.number)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO mrs (id, worktree_id, platform, remote_id, url, state)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(worktree_id, remote_id)
               DO UPDATE SET url = excluded.url, state = excluded.state",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(worktree.as_str())
        .bind(forge.as_str())
        .bind(&read.number)
        .bind(&read.details.web_url)
        .bind(word_of(read.details.state))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        self.get(worktree)
            .await?
            .ok_or_else(|| Error::Invalid(format!("{worktree} lost its merge request")))
    }

    /// Forgets the worktree's MR, for one closed or a branch that lost it.
    pub async fn remove(&self, worktree: &WorktreeId) -> Result<()> {
        sqlx::query("DELETE FROM mrs WHERE worktree_id = ?")
            .bind(worktree.as_str())
            .execute(self.db.pool())
            .await?;
        Ok(())
    }
}

/// The one word a state is stored as.
fn word_of(state: MrState) -> &'static str {
    match state {
        MrState::Open => "open",
        MrState::Merged => "merged",
        MrState::Closed => "closed",
    }
}

fn state_of(word: &str) -> MrState {
    match word {
        "merged" => MrState::Merged,
        "closed" => MrState::Closed,
        _ => MrState::Open,
    }
}
