use sqlx::SqlitePool;

use crate::core::db::models::Annotation;
use crate::core::db::store;
use crate::core::error::AppResult;

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn create_annotation(
    session_id: String,
    repo_id: String,
    file_path: String,
    start_line: i64,
    end_line: i64,
    content: String,
    author: String,
    pool: tauri::State<'_, SqlitePool>,
) -> AppResult<Annotation> {
    Ok(store::annotations::create(
        &*pool,
        &session_id,
        &repo_id,
        &file_path,
        start_line,
        end_line,
        &content,
        &author,
    )
    .await?)
}

/// Rewrite a note's body.
#[tauri::command]
pub async fn update_annotation(
    id: String,
    content: String,
    pool: tauri::State<'_, SqlitePool>,
) -> AppResult<Annotation> {
    Ok(store::annotations::update(&*pool, &id, &content).await?)
}

#[tauri::command]
pub async fn resolve_annotation(id: String, pool: tauri::State<'_, SqlitePool>) -> AppResult<()> {
    Ok(store::annotations::resolve(&*pool, &id).await?)
}

/// Delete an annotation outright.
#[tauri::command]
pub async fn delete_annotation(id: String, pool: tauri::State<'_, SqlitePool>) -> AppResult<()> {
    Ok(store::annotations::delete(&*pool, &id).await?)
}

#[tauri::command]
pub async fn get_annotations(
    session_id: String,
    repo_id: Option<String>,
    pool: tauri::State<'_, SqlitePool>,
) -> AppResult<Vec<Annotation>> {
    Ok(store::annotations::for_session(&*pool, &session_id, repo_id.as_deref()).await?)
}
