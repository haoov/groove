use sqlx::SqlitePool;

use crate::core::db::models::Annotation;
use crate::core::db::store;

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
) -> Result<Annotation, String> {
    store::annotations::create(
        &*pool, &session_id, &repo_id, &file_path, start_line, end_line, &content, &author,
    )
    .await
    .map_err(|e| e.to_string())
}

/// Rewrite a note's body. A note is a draft until it is posted to the MR, so both
/// the human's and the agent's are editable up to that point.
#[tauri::command]
pub async fn update_annotation(
    id: String,
    content: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Annotation, String> {
    store::annotations::update(&*pool, &id, &content).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn resolve_annotation(
    id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<(), String> {
    store::annotations::resolve(&*pool, &id).await.map_err(|e| e.to_string())
}

/// Delete an annotation outright. Resolving keeps a record; deleting is for notes
/// that shouldn't have been made at all (a wrong line, an agent's false positive).
#[tauri::command]
pub async fn delete_annotation(
    id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<(), String> {
    store::annotations::delete(&*pool, &id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_annotations(
    session_id: String,
    repo_id: Option<String>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Vec<Annotation>, String> {
    store::annotations::for_session(&*pool, &session_id, repo_id.as_deref())
        .await
        .map_err(|e| e.to_string())
}
