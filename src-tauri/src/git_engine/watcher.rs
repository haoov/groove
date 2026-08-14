use std::path::Path;
use sqlx::SqlitePool;
use tauri::Emitter;
use crate::db::schema::Worktree;
use super::State;

/// Start watching one worktree's working tree for filesystem changes, emitting
/// `file_changed` so the frontend can auto-refresh the diff. Idempotent per path:
/// a second call for the same worktree is a no-op (watchers are keyed by path).
///
/// Events inside `.git` are filtered out — the frontend refreshes git status on
/// its own git actions (UI/MCP go through the confirmation bridge), so we only
/// need to react to working-tree content changes here (and avoid a refresh loop
/// where our own `git status` touches the index and re-triggers the watcher).
pub fn watch_worktree(path: &str, app: &tauri::AppHandle, git_state: &State) -> anyhow::Result<()> {
    use notify::Watcher;

    if let Ok(guard) = git_state.inner.watchers.lock() {
        if guard.contains_key(path) {
            return Ok(());
        }
    }
    if !Path::new(path).is_dir() {
        return Ok(());
    }

    let handle = app.clone();
    let mut watcher = notify::RecommendedWatcher::new(
        move |res: notify::Result<notify::Event>| {
            let Ok(event) = res else { return };
            // Pure access events don't change content — ignore them.
            if matches!(event.kind, notify::EventKind::Access(_)) {
                return;
            }
            let paths: Vec<String> = event
                .paths
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .filter(|p| !p.split(std::path::MAIN_SEPARATOR).any(|seg| seg == ".git"))
                .collect();
            if paths.is_empty() {
                return;
            }
            let _ = handle.emit(crate::events::FILE_CHANGED, serde_json::json!({ "paths": paths }));
        },
        notify::Config::default(),
    )?;

    watcher.watch(Path::new(path), notify::RecursiveMode::Recursive)?;

    if let Ok(mut guard) = git_state.inner.watchers.lock() {
        guard.insert(path.to_string(), watcher);
    }
    Ok(())
}

/// Start (or ensure) filesystem watchers for every active worktree of a task, so
/// edits made by the agent, the terminal, or external tools auto-refresh the diff.
/// Called by the frontend when a workspace becomes ready and after adding a repo.
#[tauri::command]
pub async fn watch_task_worktrees(
    app: tauri::AppHandle,
    task_id: String,
    pool: tauri::State<'_, SqlitePool>,
    git_state: tauri::State<'_, State>,
) -> Result<(), String> {
    let worktrees: Vec<Worktree> =
        crate::db::load::active_worktrees(&pool, &task_id)
            .await
            .map_err(|e| e.to_string())?;

    // Sweep entries whose worktree dir no longer exists on disk: a closed or
    // re-provisioned worktree leaves a watcher bound to a dead path. Dropping it
    // re-arms watching on the next pass and keeps the map from growing unbounded.
    if let Ok(mut guard) = git_state.inner.watchers.lock() {
        guard.retain(|path, _| Path::new(path).exists());
    }

    for wt in worktrees {
        if let Err(e) = watch_worktree(&wt.path, &app, &git_state) {
            tracing::warn!("[watcher] failed to watch {}: {e}", wt.path);
        }
    }
    Ok(())
}
