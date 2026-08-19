use sqlx::SqlitePool;
use tauri::Emitter;

#[tauri::command]
pub async fn commit(
    worktree_id: String,
    message: String,
    pool: tauri::State<'_, SqlitePool>,
    bridge: tauri::State<'_, crate::confirmation_bridge::Bridge>,
) -> Result<String, String> {
    let wt = crate::core::db::store::worktrees::get(&*pool, &worktree_id).await
        .map_err(|e| e.to_string())?;

    let payload = serde_json::json!({
        "worktree_id": worktree_id,
        "worktree_path": wt.path,
        "branch": wt.branch,
        "message": message,
    });

    bridge
        .post(&pool, crate::ops::GIT_COMMIT, payload, "ui", Some(&wt.session_id))
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
    super::run_git(path, args)
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
    bridge: tauri::State<'_, crate::confirmation_bridge::Bridge>,
) -> Result<String, String> {
    let wt = crate::core::db::store::worktrees::get(&*pool, &worktree_id).await
        .map_err(|e| e.to_string())?;

    let payload = serde_json::json!({
        "worktree_id": worktree_id,
        "worktree_path": wt.path,
        "file_path": file_path,
    });
    bridge
        .post(&pool, crate::ops::GIT_DISCARD, payload, "ui", Some(&wt.session_id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn discard_all(
    worktree_id: String,
    pool: tauri::State<'_, SqlitePool>,
    bridge: tauri::State<'_, crate::confirmation_bridge::Bridge>,
) -> Result<String, String> {
    let wt = crate::core::db::store::worktrees::get(&*pool, &worktree_id).await
        .map_err(|e| e.to_string())?;

    let payload = serde_json::json!({
        "worktree_id": worktree_id,
        "worktree_path": wt.path,
    });
    bridge
        .post(&pool, crate::ops::GIT_DISCARD_ALL, payload, "ui", Some(&wt.session_id))
        .await
        .map_err(|e| e.to_string())
}

/// Discard local changes for one file. Tracked/staged paths are restored from
/// HEAD (a staged-new file is removed); a purely untracked file is deleted.
pub async fn discard_impl(payload: serde_json::Value) -> anyhow::Result<()> {
    let path = payload["worktree_path"].as_str().ok_or_else(|| anyhow::anyhow!("missing worktree_path"))?.to_string();
    let file = payload["file_path"].as_str().ok_or_else(|| anyhow::anyhow!("missing file_path"))?.to_string();

    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        let in_index = std::process::Command::new("git")
            .args(["ls-files", "--error-unmatch", "--", &file])
            .current_dir(&path)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        let args: Vec<&str> = if in_index {
            vec!["restore", "--source=HEAD", "--staged", "--worktree", "--", &file]
        } else {
            vec!["clean", "-fd", "--", &file]
        };
        let out = std::process::Command::new("git")
            .args(&args)
            .current_dir(&path)
            .output()?;
        if !out.status.success() {
            return Err(anyhow::anyhow!("git {} failed: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim()));
        }
        Ok(())
    })
    .await?
}

/// Discard ALL local changes: revert tracked files to HEAD and remove untracked.
pub async fn discard_all_impl(payload: serde_json::Value) -> anyhow::Result<()> {
    let path = payload["worktree_path"].as_str().ok_or_else(|| anyhow::anyhow!("missing worktree_path"))?.to_string();

    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        for args in [["reset", "-q", "--hard", "HEAD"].as_slice(), ["clean", "-fd"].as_slice()] {
            let out = std::process::Command::new("git")
                .args(args)
                .current_dir(&path)
                .output()?;
            if !out.status.success() {
                return Err(anyhow::anyhow!("git {} failed: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim()));
            }
        }
        Ok(())
    })
    .await?
}

#[tauri::command]
pub async fn push(
    worktree_id: String,
    pool: tauri::State<'_, SqlitePool>,
    bridge: tauri::State<'_, crate::confirmation_bridge::Bridge>,
) -> Result<String, String> {
    let wt = crate::core::db::store::worktrees::get(&*pool, &worktree_id).await
        .map_err(|e| e.to_string())?;

    let payload = serde_json::json!({
        "worktree_id": worktree_id,
        "worktree_path": wt.path,
        "branch": wt.branch,
    });

    bridge
        .post(&pool, crate::ops::GIT_PUSH, payload, "ui", Some(&wt.session_id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pull(
    worktree_id: String,
    pool: tauri::State<'_, SqlitePool>,
    bridge: tauri::State<'_, crate::confirmation_bridge::Bridge>,
) -> Result<String, String> {
    let wt = crate::core::db::store::worktrees::get(&*pool, &worktree_id).await
        .map_err(|e| e.to_string())?;

    let payload = serde_json::json!({
        "worktree_id": worktree_id,
        "worktree_path": wt.path,
        "branch": wt.branch,
    });

    bridge
        .post(&pool, crate::ops::GIT_PULL, payload, "ui", Some(&wt.session_id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rebase_on_main(
    worktree_id: String,
    default_branch: Option<String>,
    pool: tauri::State<'_, SqlitePool>,
    bridge: tauri::State<'_, crate::confirmation_bridge::Bridge>,
) -> Result<String, String> {
    let wt = crate::core::db::store::worktrees::get(&*pool, &worktree_id).await
        .map_err(|e| e.to_string())?;

    let payload = serde_json::json!({
        "worktree_id": worktree_id,
        "worktree_path": wt.path,
        "branch": wt.branch,
        "default_branch": default_branch.unwrap_or_else(|| "main".to_string()),
    });

    bridge
        .post(&pool, crate::ops::GIT_REBASE, payload, "ui", Some(&wt.session_id))
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
    let output = super::run_git_output(&wt.path, &["-c", "core.editor=true", "rebase", "--continue"])
        .await
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        app.emit(
            crate::events::REBASE_DONE,
            serde_json::json!({ "worktree_id": worktree_id }),
        )
        .map_err(|e| e.to_string())?;
    } else {
        let conflicts = get_conflict_files(&wt.path).await;
        app.emit(
            crate::events::REBASE_CONFLICT,
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

    let output = super::run_git_output(&wt.path, &["rebase", "--abort"])
        .await
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "git rebase --abort failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    app.emit(
        crate::events::REBASE_DONE,
        serde_json::json!({ "worktree_id": worktree_id, "aborted": true }),
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

async fn get_conflict_files(path: &str) -> Vec<String> {
    super::run_git_output(path, &["diff", "--name-only", "--diff-filter=U"])
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
    let path = payload["worktree_path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing worktree_path"))?
        .to_string();
    let message = payload["message"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing message"))?
        .to_string();

    // Stage-aware: if anything is staged, commit the index only; otherwise commit
    // all tracked changes (the simple "type a message → commit everything" flow).
    let has_staged = {
        let path = path.clone();
        tokio::task::spawn_blocking(move || {
            std::process::Command::new("git")
                .args(["diff", "--cached", "--quiet"])
                .current_dir(&path)
                .status()
                .map(|s| !s.success())
                .unwrap_or(false)
        })
        .await?
    };

    let output = tokio::task::spawn_blocking(move || {
        let mut args = vec!["commit"];
        if !has_staged {
            args.push("-a");
        }
        args.push("-m");
        args.push(&message);
        std::process::Command::new("git")
            .args(&args)
            .current_dir(&path)
            .output()
    })
    .await??;

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
    Ok(())
}

pub async fn push_impl(payload: serde_json::Value) -> anyhow::Result<()> {
    let path = payload["worktree_path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing worktree_path"))?
        .to_string();
    let branch = payload["branch"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing branch"))?
        .to_string();

    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new("git")
            .args(["push", "origin", &format!("{branch}:{branch}"), "--set-upstream"])
            .current_dir(&path)
            .output()
    })
    .await??;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "git push failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

pub async fn pull_impl(payload: serde_json::Value) -> anyhow::Result<()> {
    let path = payload["worktree_path"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing worktree_path"))?
        .to_string();

    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new("git")
            .args(["pull", "--rebase"])
            .current_dir(&path)
            .output()
    })
    .await??;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "git pull failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
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

    let path2 = path.clone();
    let _ = tokio::task::spawn_blocking(move || {
        std::process::Command::new("git")
            .args(["fetch", "origin"])
            .current_dir(&path2)
            .status()
    })
    .await;

    // Rebase onto the base BRANCH, never a merge-base: the point of the rebase is
    // to move onto the branch tip.
    let base_ref = super::upstream_base(&path, Some(&default_branch)).await?;

    let path3 = path.clone();
    let base_for_cmd = base_ref.clone();
    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new("git")
            .args(["rebase", &base_for_cmd])
            .current_dir(&path3)
            .output()
    })
    .await??;

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
