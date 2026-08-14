use sqlx::SqlitePool;

use crate::db::schema::Annotation;

// A command's parameters are its JS argument names — not foldable into a struct.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn create_annotation(
    task_id: String,
    repo_id: String,
    file_path: String,
    start_line: i64,
    end_line: i64,
    content: String,
    author: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Annotation, String> {
    create_annotation_impl(task_id, repo_id, file_path, start_line, end_line, content, author, &pool)
        .await
        .map_err(|e| e.to_string())
}

#[allow(clippy::too_many_arguments)]
pub async fn create_annotation_impl(
    task_id: String,
    repo_id: String,
    file_path: String,
    start_line: i64,
    end_line: i64,
    content: String,
    author: String,
    pool: &SqlitePool,
) -> anyhow::Result<Annotation> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp();
    // Normalize so start <= end regardless of drag direction; anchor at start_line
    // (the first line of the range) so the marker/comment sits at the top.
    let (start_line, end_line) = (start_line.min(end_line), start_line.max(end_line));
    let line_num = start_line;

    sqlx::query(
        "INSERT INTO annotations
         (id, task_id, repo_id, file_path, line_num, start_line, end_line, content, author, status, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'open', ?)",
    )
    .bind(&id)
    .bind(&task_id)
    .bind(&repo_id)
    .bind(&file_path)
    .bind(line_num)
    .bind(start_line)
    .bind(end_line)
    .bind(&content)
    .bind(&author)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(Annotation {
        id,
        task_id,
        repo_id,
        file_path,
        line_num,
        start_line,
        end_line,
        content,
        author,
        status: "open".to_string(),
        created_at: now,
    })
}

#[tauri::command]
pub async fn resolve_annotation(
    id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<(), String> {
    resolve_annotation_impl(serde_json::json!({ "id": id }), &pool)
        .await
        .map_err(|e| e.to_string())
}

pub async fn resolve_annotation_impl(
    payload: serde_json::Value,
    pool: &SqlitePool,
) -> anyhow::Result<()> {
    let id = payload["id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing id"))?;
    sqlx::query("UPDATE annotations SET status = 'resolved' WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Delete an annotation outright. Resolving keeps a record; deleting is for notes
/// that shouldn't have been made at all (a wrong line, an agent's false positive).
#[tauri::command]
pub async fn delete_annotation(
    id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<(), String> {
    sqlx::query("DELETE FROM annotations WHERE id = ?")
        .bind(&id)
        .execute(&*pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_annotations(
    task_id: String,
    repo_id: Option<String>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Vec<Annotation>, String> {
    let rows: Result<Vec<Annotation>, _> = if let Some(repo) = repo_id {
        sqlx::query_as(
            "SELECT * FROM annotations WHERE task_id = ? AND repo_id = ?
             ORDER BY file_path, line_num",
        )
        .bind(&task_id)
        .bind(&repo)
        .fetch_all(&*pool)
        .await
    } else {
        sqlx::query_as(
            "SELECT * FROM annotations WHERE task_id = ?
             ORDER BY file_path, line_num",
        )
        .bind(&task_id)
        .fetch_all(&*pool)
        .await
    };

    rows.map_err(|e| e.to_string())
}
