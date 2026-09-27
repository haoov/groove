//! The notes a session leaves on its own code: one per range of a file's new side.

#[cfg(test)]
mod tests;

use groove_db::Db;
use groove_types::{
    Annotation, AnnotationId, AnnotationStatus, Error, RepoId, Result, SessionId, Timestamp,
};

/// A note as the table holds it.
#[derive(sqlx::FromRow)]
struct Row {
    id: String,
    session_id: String,
    repo_id: String,
    file_path: String,
    start_line: i64,
    end_line: i64,
    content: String,
    author: String,
    status: String,
    created_at: i64,
}

impl From<Row> for Annotation {
    fn from(row: Row) -> Self {
        Annotation {
            id: AnnotationId::new(row.id),
            session: SessionId::new(row.session_id),
            repo: RepoId::new(row.repo_id),
            file_path: row.file_path,
            start_line: line_of(row.start_line),
            end_line: line_of(row.end_line),
            content: row.content,
            author: row.author,
            status: status_of(&row.status),
            created_at: Timestamp::new(row.created_at),
        }
    }
}

/// A note to write, before the store gives it an id and a time.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct New {
    pub session: SessionId,
    pub repo: RepoId,
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub content: String,
    pub author: String,
}

/// The note rows, cheap to clone: a handle on the pool.
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
            .map_err(|e| Error::db(format!("no database: {e}")))?;
        Ok(Self::new(db))
    }

    /// The pool this store writes to, for the modules that share it.
    pub fn db(&self) -> &Db {
        &self.db
    }

    /// Every note of a session, in the order a file reads.
    pub async fn list(&self, session: &SessionId) -> Result<Vec<Annotation>> {
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT id, session_id, repo_id, file_path, start_line, end_line,
                    content, author, status, created_at
             FROM annotations WHERE session_id = ?
             ORDER BY file_path, start_line, created_at",
        )
        .bind(session.as_str())
        .fetch_all(self.db.pool())
        .await
        .map_err(failed)?;
        Ok(rows.into_iter().map(Annotation::from).collect())
    }

    pub async fn get(&self, id: &AnnotationId) -> Result<Option<Annotation>> {
        let row: Option<Row> = sqlx::query_as(
            "SELECT id, session_id, repo_id, file_path, start_line, end_line,
                    content, author, status, created_at
             FROM annotations WHERE id = ?",
        )
        .bind(id.as_str())
        .fetch_optional(self.db.pool())
        .await
        .map_err(failed)?;
        Ok(row.map(Annotation::from))
    }

    /// Writes a note, with the range the right way round.
    pub async fn create(&self, new: New, now: Timestamp) -> Result<Annotation> {
        let id = AnnotationId::new(uuid::Uuid::new_v4().to_string());
        let (start, end) = ordered(new.start_line, new.end_line);
        sqlx::query(
            "INSERT INTO annotations
                (id, session_id, repo_id, file_path, start_line, end_line,
                 content, author, status, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'open', ?)",
        )
        .bind(id.as_str())
        .bind(new.session.as_str())
        .bind(new.repo.as_str())
        .bind(&new.file_path)
        .bind(i64::from(start))
        .bind(i64::from(end))
        .bind(&new.content)
        .bind(&new.author)
        .bind(now.seconds())
        .execute(self.db.pool())
        .await
        .map_err(failed)?;
        self.one(&id).await
    }

    /// New words on a note that stands.
    pub async fn update(&self, id: &AnnotationId, content: &str) -> Result<Annotation> {
        let done = sqlx::query("UPDATE annotations SET content = ? WHERE id = ?")
            .bind(content)
            .bind(id.as_str())
            .execute(self.db.pool())
            .await
            .map_err(failed)?;
        self.changed(id, done.rows_affected()).await
    }

    pub async fn resolve(&self, id: &AnnotationId) -> Result<Annotation> {
        self.status(id, "resolved").await
    }

    pub async fn reopen(&self, id: &AnnotationId) -> Result<Annotation> {
        self.status(id, "open").await
    }

    pub async fn delete(&self, id: &AnnotationId) -> Result<()> {
        sqlx::query("DELETE FROM annotations WHERE id = ?")
            .bind(id.as_str())
            .execute(self.db.pool())
            .await
            .map_err(failed)?;
        Ok(())
    }

    async fn status(&self, id: &AnnotationId, word: &str) -> Result<Annotation> {
        let done = sqlx::query("UPDATE annotations SET status = ? WHERE id = ?")
            .bind(word)
            .bind(id.as_str())
            .execute(self.db.pool())
            .await
            .map_err(failed)?;
        self.changed(id, done.rows_affected()).await
    }

    /// The note as it now stands, or the refusal a write that touched nothing earns.
    async fn changed(&self, id: &AnnotationId, rows: u64) -> Result<Annotation> {
        match rows {
            0 => Err(gone(id)),
            _ => self.one(id).await,
        }
    }

    async fn one(&self, id: &AnnotationId) -> Result<Annotation> {
        self.get(id).await?.ok_or_else(|| gone(id))
    }
}

/// The lower line first, whichever way the selection was made.
fn ordered(start: u32, end: u32) -> (u32, u32) {
    (start.min(end), start.max(end))
}

fn line_of(stored: i64) -> u32 {
    u32::try_from(stored).unwrap_or_default()
}

fn status_of(word: &str) -> AnnotationStatus {
    match word {
        "resolved" => AnnotationStatus::Resolved,
        _ => AnnotationStatus::Open,
    }
}

fn gone(id: &AnnotationId) -> Error {
    Error::not_found(format!("no note {id}"))
}

fn failed(source: sqlx::Error) -> Error {
    Error::db(format!("the notes could not be read or written: {source}"))
}
