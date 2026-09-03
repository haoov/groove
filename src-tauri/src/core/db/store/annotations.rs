use sqlx::SqliteExecutor;

use super::super::error::StoreResult;
use super::super::models::Annotation;

const COLUMNS: &str =
    "id, session_id, repo_id, file_path, start_line, end_line, content, author, status, created_at";

#[allow(clippy::too_many_arguments)]
pub async fn create(
    exec: impl SqliteExecutor<'_>,
    session_id: &str,
    repo_id: &str,
    file_path: &str,
    start_line: i64,
    end_line: i64,
    content: &str,
    author: &str,
) -> StoreResult<Annotation> {
    let annotation = Annotation {
        id: uuid::Uuid::new_v4().to_string(),
        session_id: session_id.to_string(),
        repo_id: repo_id.to_string(),
        file_path: file_path.to_string(),
        start_line: start_line.min(end_line),
        end_line: start_line.max(end_line),
        content: content.to_string(),
        author: author.to_string(),
        status: "open".to_string(),
        created_at: chrono::Utc::now().timestamp(),
    };
    sqlx::query(
        "INSERT INTO annotations
           (id, session_id, repo_id, file_path, start_line, end_line,
            content, author, status, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&annotation.id)
    .bind(&annotation.session_id)
    .bind(&annotation.repo_id)
    .bind(&annotation.file_path)
    .bind(annotation.start_line)
    .bind(annotation.end_line)
    .bind(&annotation.content)
    .bind(&annotation.author)
    .bind(&annotation.status)
    .bind(annotation.created_at)
    .execute(exec)
    .await?;
    Ok(annotation)
}

pub async fn for_session(
    exec: impl SqliteExecutor<'_>,
    session_id: &str,
    repo_id: Option<&str>,
) -> StoreResult<Vec<Annotation>> {
    let rows = match repo_id {
        Some(repo_id) => {
            sqlx::query_as(&format!(
                "SELECT {COLUMNS} FROM annotations
                 WHERE session_id = ? AND repo_id = ?
                 ORDER BY file_path, start_line"
            ))
            .bind(session_id)
            .bind(repo_id)
            .fetch_all(exec)
            .await?
        }
        None => {
            sqlx::query_as(&format!(
                "SELECT {COLUMNS} FROM annotations
                 WHERE session_id = ?
                 ORDER BY file_path, start_line"
            ))
            .bind(session_id)
            .fetch_all(exec)
            .await?
        }
    };
    Ok(rows)
}

/// Rewrite a note's body, leaving its author, range and status alone.
pub async fn update(
    exec: impl SqliteExecutor<'_>,
    id: &str,
    content: &str,
) -> StoreResult<Annotation> {
    let row = sqlx::query_as(&format!(
        "UPDATE annotations SET content = ? WHERE id = ? RETURNING {COLUMNS}"
    ))
    .bind(content)
    .bind(id)
    .fetch_one(exec)
    .await?;
    Ok(row)
}

pub async fn resolve(exec: impl SqliteExecutor<'_>, id: &str) -> StoreResult<()> {
    sqlx::query("UPDATE annotations SET status = 'resolved' WHERE id = ?")
        .bind(id)
        .execute(exec)
        .await?;
    Ok(())
}

pub async fn delete(exec: impl SqliteExecutor<'_>, id: &str) -> StoreResult<()> {
    sqlx::query("DELETE FROM annotations WHERE id = ?")
        .bind(id)
        .execute(exec)
        .await?;
    Ok(())
}
