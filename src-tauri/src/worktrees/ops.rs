use sqlx::SqlitePool;
use tauri::Emitter;

use crate::core::git;

/// The fields every git-op confirmation carries. `repo` is the project name for
/// display: worktree paths embed the branch's slashes now, so the path's last
/// segment is the branch leaf, not the repo.
pub(crate) async fn op_payload(
    pool: &SqlitePool,
    wt: &crate::core::db::models::Worktree,
) -> serde_json::Value {
    let repo = crate::core::db::store::repos::get_opt(pool, &wt.repo_id)
        .await
        .ok()
        .flatten()
        .map(|r| r.project)
        .unwrap_or_default();
    serde_json::json!({
        "worktree_id": wt.id,
        "worktree_path": wt.path,
        "branch": wt.branch,
        "repo": repo,
    })
}

#[tauri::command]
pub async fn commit(
    worktree_id: String,
    message: String,
    pool: tauri::State<'_, SqlitePool>,
    bridge: tauri::State<'_, crate::approvals::Bridge>,
) -> Result<String, String> {
    let wt = crate::core::db::store::worktrees::get(&*pool, &worktree_id).await
        .map_err(|e| e.to_string())?;

    let mut payload = op_payload(&pool, &wt).await;
    payload["message"] = serde_json::json!(message);

    bridge
        .post(&pool, crate::approvals::ops::GIT_COMMIT, payload, "ui", Some(&wt.session_id))
        .await
        .map_err(|e| e.to_string())
}

// ─── Staging ──────────────────────────────────────────────────────────────────

async fn worktree_path(worktree_id: &str, pool: &SqlitePool) -> Result<String, String> {
    let wt = crate::core::db::store::worktrees::get(pool, worktree_id).await
        .map_err(|e| e.to_string())?;
    Ok(wt.path)
}

/// Run `git <args>` in `path` off the async runtime, mapping any failure to a
/// user-facing string. Thin wrapper over the shared `run_git` for the command
/// layer (which returns `Result<(), String>`).
async fn run_git_in(path: &str, args: &[&str]) -> Result<(), String> {
    crate::core::git::run(path, args)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stage_file(
    worktree_id: String,
    file_path: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<(), String> {
    let path = worktree_path(&worktree_id, &pool).await?;
    run_git_in(&path, &["add", "--", &file_path]).await
}

#[tauri::command]
pub async fn unstage_file(
    worktree_id: String,
    file_path: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<(), String> {
    let path = worktree_path(&worktree_id, &pool).await?;
    run_git_in(&path, &["restore", "--staged", "--", &file_path]).await
}

#[tauri::command]
pub async fn stage_all(
    worktree_id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<(), String> {
    let path = worktree_path(&worktree_id, &pool).await?;
    run_git_in(&path, &["add", "-A"]).await
}

#[tauri::command]
pub async fn unstage_all(
    worktree_id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<(), String> {
    let path = worktree_path(&worktree_id, &pool).await?;
    run_git_in(&path, &["reset", "-q", "HEAD"]).await
}

// ─── Discard (destructive — gated by the confirmation bridge) ──────────────────

#[tauri::command]
pub async fn discard_file(
    worktree_id: String,
    file_path: String,
    pool: tauri::State<'_, SqlitePool>,
    bridge: tauri::State<'_, crate::approvals::Bridge>,
) -> Result<String, String> {
    let wt = crate::core::db::store::worktrees::get(&*pool, &worktree_id).await
        .map_err(|e| e.to_string())?;

    let payload = serde_json::json!({
        "worktree_id": worktree_id,
        "worktree_path": wt.path,
        "file_path": file_path,
    });
    bridge
        .post(&pool, crate::approvals::ops::GIT_DISCARD, payload, "ui", Some(&wt.session_id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn discard_all(
    worktree_id: String,
    pool: tauri::State<'_, SqlitePool>,
    bridge: tauri::State<'_, crate::approvals::Bridge>,
) -> Result<String, String> {
    let wt = crate::core::db::store::worktrees::get(&*pool, &worktree_id).await
        .map_err(|e| e.to_string())?;

    let payload = serde_json::json!({
        "worktree_id": worktree_id,
        "worktree_path": wt.path,
    });
    bridge
        .post(&pool, crate::approvals::ops::GIT_DISCARD_ALL, payload, "ui", Some(&wt.session_id))
        .await
        .map_err(|e| e.to_string())
}

/// Discard local changes for one file. Tracked/staged paths are restored from
/// HEAD (a staged-new file is removed); a purely untracked file is deleted.
pub async fn discard_impl(payload: serde_json::Value) -> anyhow::Result<()> {
    let path = required_str(&payload, "worktree_path")?;
    let file = required_str(&payload, "file_path")?;

    let in_index = git::output(path, &["ls-files", "--error-unmatch", "--", file])
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);

    if in_index {
        git::run(path, &["restore", "--source=HEAD", "--staged", "--worktree", "--", file]).await?;
    } else {
        git::run(path, &["clean", "-fd", "--", file]).await?;
    }
    Ok(())
}

fn required_str<'a>(payload: &'a serde_json::Value, key: &str) -> anyhow::Result<&'a str> {
    payload[key]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing {key}"))
}

/// Discard ALL local changes: revert tracked files to HEAD and remove untracked.
pub async fn discard_all_impl(payload: serde_json::Value) -> anyhow::Result<()> {
    let path = required_str(&payload, "worktree_path")?;
    git::run(path, &["reset", "-q", "--hard", "HEAD"]).await?;
    git::run(path, &["clean", "-fd"]).await?;
    git::cache::flush();
    Ok(())
}

#[tauri::command]
pub async fn push(
    worktree_id: String,
    pool: tauri::State<'_, SqlitePool>,
    bridge: tauri::State<'_, crate::approvals::Bridge>,
) -> Result<String, String> {
    let wt = crate::core::db::store::worktrees::get(&*pool, &worktree_id).await
        .map_err(|e| e.to_string())?;

    let payload = op_payload(&pool, &wt).await;

    bridge
        .post(&pool, crate::approvals::ops::GIT_PUSH, payload, "ui", Some(&wt.session_id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pull(
    worktree_id: String,
    pool: tauri::State<'_, SqlitePool>,
    bridge: tauri::State<'_, crate::approvals::Bridge>,
) -> Result<String, String> {
    let wt = crate::core::db::store::worktrees::get(&*pool, &worktree_id).await
        .map_err(|e| e.to_string())?;

    let payload = op_payload(&pool, &wt).await;

    bridge
        .post(&pool, crate::approvals::ops::GIT_PULL, payload, "ui", Some(&wt.session_id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rebase_on_main(
    worktree_id: String,
    default_branch: Option<String>,
    pool: tauri::State<'_, SqlitePool>,
    bridge: tauri::State<'_, crate::approvals::Bridge>,
) -> Result<String, String> {
    let wt = crate::core::db::store::worktrees::get(&*pool, &worktree_id).await
        .map_err(|e| e.to_string())?;

    let mut payload = op_payload(&pool, &wt).await;
    payload["default_branch"] = serde_json::json!(default_branch.unwrap_or_else(|| "main".to_string()));

    bridge
        .post(&pool, crate::approvals::ops::GIT_REBASE, payload, "ui", Some(&wt.session_id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rebase_continue(
    app: tauri::AppHandle,
    worktree_id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<(), String> {
    let wt = crate::core::db::store::worktrees::get(&*pool, &worktree_id).await
        .map_err(|e| e.to_string())?;

    // `-c core.editor=true` stands in for the old `GIT_EDITOR=true` env: it stops
    // `rebase --continue` from popping an editor for the commit message. run_git_output
    // keeps this (potentially slow) call off the tokio runtime.
    let output = crate::core::git::output(&wt.path, &["-c", "core.editor=true", "rebase", "--continue"])
        .await
        .map_err(|e| e.to_string())?;
    crate::core::git::cache::flush();

    if output.status.success() {
        app.emit(
            crate::core::events::REBASE_DONE,
            serde_json::json!({ "worktree_id": worktree_id }),
        )
        .map_err(|e| e.to_string())?;
    } else {
        let conflicts = get_conflict_files(&wt.path).await;
        app.emit(
            crate::core::events::REBASE_CONFLICT,
            serde_json::json!({ "worktree_id": worktree_id, "files": conflicts }),
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub async fn rebase_abort(
    app: tauri::AppHandle,
    worktree_id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<(), String> {
    let wt = crate::core::db::store::worktrees::get(&*pool, &worktree_id).await
        .map_err(|e| e.to_string())?;

    let output = crate::core::git::output(&wt.path, &["rebase", "--abort"])
        .await
        .map_err(|e| e.to_string())?;
    crate::core::git::cache::flush();
    if !output.status.success() {
        return Err(format!(
            "git rebase --abort failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    app.emit(
        crate::core::events::REBASE_DONE,
        serde_json::json!({ "worktree_id": worktree_id, "aborted": true }),
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

async fn get_conflict_files(path: &str) -> Vec<String> {
    crate::core::git::output(path, &["diff", "--name-only", "--diff-filter=U"])
        .await
        .ok()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|l| !l.is_empty())
                .map(|l| l.to_string())
                .collect()
        })
        .unwrap_or_default()
}

pub async fn commit_impl(payload: serde_json::Value, _pool: &SqlitePool) -> anyhow::Result<()> {
    let path = required_str(&payload, "worktree_path")?;
    let message = required_str(&payload, "message")?;

    // Stage-aware: if anything is staged, commit the index only; otherwise commit
    // all tracked changes (the simple "type a message → commit everything" flow).
    // `index_only` opts out of that second half — see the agent's path in
    // mcp_server/tools/write.rs.
    let index_only = payload["index_only"].as_bool().unwrap_or(false);
    let has_staged = git::output(path, &["diff", "--cached", "--quiet"])
        .await
        .map(|o| !o.status.success())
        .unwrap_or(false);

    if index_only && !has_staged {
        return Err(anyhow::anyhow!(
            "nothing staged. Stage exactly what this commit should contain first —              `git add <paths>` for some files, `git add -A` for everything — then commit."
        ));
    }

    let args: Vec<&str> = if index_only || has_staged {
        vec!["commit", "-m", message]
    } else {
        vec!["commit", "-a", "-m", message]
    };
    let output = git::output(path, &args).await?;

    if !output.status.success() {
        // `git commit` writes "nothing to commit" / "Untracked files present" to
        // STDOUT with a non-zero exit, so surfacing stderr alone yields an empty
        // "git commit failed:" message. Include both streams.
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = [stderr.trim(), stdout.trim()]
            .iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" — ");
        return Err(anyhow::anyhow!(
            "git commit failed: {}",
            if detail.is_empty() { "no output (nothing to commit?)".to_string() } else { detail }
        ));
    }
    git::cache::flush();
    Ok(())
}

pub async fn push_impl(payload: serde_json::Value) -> anyhow::Result<()> {
    let path = required_str(&payload, "worktree_path")?;
    let branch = required_str(&payload, "branch")?;

    git::run(path, &["push", "origin", &format!("{branch}:{branch}"), "--set-upstream"]).await?;
    git::cache::flush();
    Ok(())
}

pub async fn pull_impl(payload: serde_json::Value) -> anyhow::Result<()> {
    let path = required_str(&payload, "worktree_path")?;
    git::run(path, &["pull", "--rebase"]).await?;
    git::cache::flush();
    Ok(())
}

pub async fn rebase_impl(payload: serde_json::Value) -> anyhow::Result<serde_json::Value> {
    let path = payload["worktree_path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing worktree_path"))?
        .to_string();
    let default_branch = payload["default_branch"]
        .as_str()
        .unwrap_or("main")
        .to_string();
    let worktree_id = payload["worktree_id"].as_str().unwrap_or("").to_string();

    let _ = git::run(&path, &["fetch", "origin"]).await;
    git::cache::flush();

    // Rebase onto the base BRANCH, never a merge-base: the point of the rebase is
    // to move onto the branch tip.
    let base_ref = git::refs::upstream_base(&path, Some(&default_branch)).await?;

    let output = git::output(&path, &["rebase", &base_ref]).await?;
    git::cache::flush();

    if output.status.success() {
        return Ok(serde_json::json!({
            "status": "done", "worktree_id": worktree_id, "base": base_ref
        }));
    }

    let conflicts = get_conflict_files(&path).await;
    if conflicts.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(anyhow::anyhow!(
            "git rebase {base_ref} failed: {}",
            if stderr.is_empty() { "unknown error".to_string() } else { stderr }
        ));
    }
    Ok(serde_json::json!({
        "status": "conflict", "files": conflicts, "worktree_id": worktree_id, "base": base_ref
    }))
}
