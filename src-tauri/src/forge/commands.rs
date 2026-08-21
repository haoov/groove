use sqlx::SqlitePool;

use crate::core::db::models::{Mr, Repo, Worktree};
use crate::core::db::store;
use super::gitlab::fetch_and_upsert_mrs;
use super::client::make_client;

// ─── Shared lookups ───────────────────────────────────────────────────────────

/// Load the mr → worktree → repo chain for an MR id.
pub(super) async fn load_mr_context(
    mr_id: &str,
    pool: &SqlitePool,
) -> anyhow::Result<(Mr, Worktree, Repo)> {
    let mr = store::mrs::get(pool, mr_id).await?;
    let wt = store::worktrees::get(pool, &mr.worktree_id).await?;
    let repo = store::repos::get(pool, &wt.repo_id).await?;
    Ok((mr, wt, repo))
}

// ─── IPC commands ─────────────────────────────────────────────────────────────

/// Returns live MRs for the worktree's branch.
/// For GitLab repos, queries glab and upserts into DB.
/// For GitHub or on glab failure, falls back to DB.
#[tauri::command]
pub async fn get_mr(
    worktree_id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Vec<Mr>, String> {
    let wt = store::worktrees::get(&*pool, &worktree_id)
        .await
        .map_err(|e| e.to_string())?;
    let repo = store::repos::get(&*pool, &wt.repo_id)
        .await
        .map_err(|e| e.to_string())?;

    if !repo.host.contains("github") {
        match fetch_and_upsert_mrs(&wt, &repo, &pool).await {
            Ok(mrs) => return Ok(mrs),
            Err(e) => tracing::warn!("glab mr list failed: {e}"),
        }
    }

    store::mrs::for_worktree(&*pool, &worktree_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_mr_threads(
    mr_id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<serde_json::Value, String> {
    let (mr, _wt, repo) = load_mr_context(&mr_id, &pool)
        .await
        .map_err(|e| e.to_string())?;

    let client = make_client(&repo);
    client
        .get_mr_threads(&repo, &mr.remote_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_mr_ci(
    mr_id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<serde_json::Value, String> {
    let (mr, _wt, repo) = load_mr_context(&mr_id, &pool)
        .await
        .map_err(|e| e.to_string())?;

    let client = make_client(&repo);
    client
        .get_mr_ci(&repo, &mr.remote_id)
        .await
        .map_err(|e| e.to_string())
}

/// Rich MR/PR fields (title, description, author, branches, …) for the overview
/// page — live-fetched so it's always fresh; the local `mrs` row stays skeletal.
#[tauri::command]
pub async fn get_mr_details(
    mr_id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<serde_json::Value, String> {
    let (mr, _wt, repo) = load_mr_context(&mr_id, &pool)
        .await
        .map_err(|e| e.to_string())?;

    let client = make_client(&repo);
    let mut details = client
        .get_mr_details(&repo, &mr.remote_id)
        .await
        .map_err(|e| e.to_string())?;

    // Approval lives on a separate endpoint; fold it into the same payload so the
    // overview renders it without a second round trip of its own.
    if let Ok(approval) = client.get_mr_approval(&repo, &mr.remote_id).await {
        if let Some(obj) = details.as_object_mut() {
            obj.insert("approved".into(), approval["approved"].clone());
            obj.insert("approved_by_me".into(), approval["approved_by_me"].clone());
            obj.insert("approved_by".into(), approval["approved_by"].clone());
        }
    }
    Ok(details)
}

#[tauri::command]
pub async fn reply_to_thread(
    mr_id: String,
    thread_id: String,
    body: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<(), String> {
    let (mr, _wt, repo) = load_mr_context(&mr_id, &pool)
        .await
        .map_err(|e| e.to_string())?;

    let client = make_client(&repo);
    client
        .reply_to_thread(&repo, &mr.remote_id, &thread_id, &body)
        .await
        .map_err(|e| e.to_string())
}

/// Open the MR-create confirmation from the UI with the repo and branch already
/// filled in. Title and description are deliberately left EMPTY: the dialog
/// collects them and its edits become payload overrides at approve time, so the
/// text that lands on the MR is always the user's. Mirrors the MCP tool's path —
/// same op, same executor, same confirmation — only the author differs.
#[tauri::command]
pub async fn create_mr(
    worktree_id: String,
    pool: tauri::State<'_, SqlitePool>,
    bridge: tauri::State<'_, crate::approvals::Bridge>,
) -> Result<String, String> {
    let wt = store::worktrees::get(&*pool, &worktree_id)
        .await
        .map_err(|e| e.to_string())?;

    let payload = serde_json::json!({
        "worktree_id": worktree_id,
        // Shown read-only in the dialog; create_mr_impl re-derives what it needs.
        "worktree_path": wt.path,
        "branch": wt.branch,
        "title": "",
        "description": "",
    });

    bridge
        .post(&pool, crate::approvals::ops::MR_CREATE, payload, "ui", Some(&wt.session_id))
        .await
        .map_err(|e| e.to_string())
}

/// Rewrite the MR's title and/or description by hand.
///
/// Direct, not gated: you typed it and pressed save, the same rule the commit box
/// and the task composer follow. Agent-initiated updates still go through the
/// `mr.update` confirmation. Reuses `update_mr_impl`, so the Notion footer is
/// re-appended rather than lost on every edit.
#[tauri::command]
pub async fn edit_mr_text(
    mr_id: String,
    title: Option<String>,
    description: Option<String>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<(), String> {
    let payload = serde_json::json!({
        "mr_id": mr_id,
        "title": title,
        "description": description,
    });
    super::ops::update_mr_impl(payload, &pool)
        .await
        .map_err(|e| e.to_string())?;
    invalidate_mr_signals(&mr_id);
    Ok(())
}

/// Approve the MR as the current user. Direct UI invoke (non-destructive,
/// human-initiated) — like reply_to_thread, no confirmation-bridge round trip.
#[tauri::command]
pub async fn approve_mr(
    mr_id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<(), String> {
    let (mr, _wt, repo) = load_mr_context(&mr_id, &pool)
        .await
        .map_err(|e| e.to_string())?;

    let client = make_client(&repo);
    client
        .approve_mr(&repo, &mr.remote_id)
        .await
        .map_err(|e| e.to_string())?;
    invalidate_mr_signals(&mr.id);
    Ok(())
}

/// Post a comment on the MR: general note, or a positioned diff discussion when
/// `file_path` + `line` are given (new-side line on the MR head). Human-initiated
/// only — the agent drafts annotations, publishing them is a human click.
#[tauri::command]
pub async fn post_mr_comment(
    mr_id: String,
    body: String,
    file_path: Option<String>,
    line: Option<i64>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<(), String> {
    let (mr, _wt, repo) = load_mr_context(&mr_id, &pool)
        .await
        .map_err(|e| e.to_string())?;

    let client = make_client(&repo);
    let position = match (&file_path, line) {
        (Some(p), Some(l)) => Some((p.as_str(), l)),
        _ => None,
    };
    client
        .post_mr_comment(&repo, &mr.remote_id, &body, position)
        .await
        .map_err(|e| e.to_string())
}

// ─── MR signals for Home (cached: these are the only network calls there) ─────

/// Live-ish MR facts Home shows per repo. Everything else in the snapshot is
/// local git or SQLite; these two cost a forge round-trip each, so they are
/// cached per MR and refreshed on a TTL rather than on every Home render.
#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct MrSignals {
    /// Pipeline status ("success", "failed", …) — None when the MR has no pipeline.
    pub ci: Option<String>,
    #[ts(type = "number")]
    pub unresolved: i64,
    /// Carries at least one approval (from anyone).
    pub approved: bool,
    /// Live MR state ("open"/"merged"/"closed") — the stored row goes stale after
    /// a merge, so Home reads this instead. None when the fetch failed.
    pub state: Option<String>,
}

static MR_SIGNALS: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashMap<String, (std::time::Instant, MrSignals)>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

const MR_SIGNALS_TTL: std::time::Duration = std::time::Duration::from_secs(90);

/// Forget one MR's cached signals — called after approving so Home reflects it
/// on the next refresh instead of at the end of the TTL.
pub(crate) fn invalidate_mr_signals(mr_id: &str) {
    if let Ok(mut map) = MR_SIGNALS.lock() {
        map.remove(mr_id);
    }
}

/// Cached `(ci, unresolved, approved)` for one MR. `force` bypasses the TTL (manual refresh).
/// Never errors: a forge hiccup degrades to "unknown", it must not fail the snapshot.
pub(crate) async fn mr_signals(repo: &Repo, mr_id: &str, remote_id: &str, force: bool) -> MrSignals {
    if !force {
        if let Ok(map) = MR_SIGNALS.lock() {
            if let Some((at, sig)) = map.get(mr_id) {
                if at.elapsed() < MR_SIGNALS_TTL {
                    return sig.clone();
                }
            }
        }
    }

    let client = make_client(repo);

    let ci = client
        .get_mr_ci(repo, remote_id)
        .await
        .ok()
        .and_then(|v| v["status"].as_str().map(|s| s.to_string()));

    // Mirrors the frontend's countUnresolved: a thread counts when its first
    // note is resolvable and not yet resolved.
    let unresolved = client
        .get_mr_threads(repo, remote_id)
        .await
        .ok()
        .and_then(|v| v.as_array().cloned())
        .map(|threads| {
            threads
                .iter()
                .filter(|t| {
                    let first = &t["notes"][0];
                    first["resolvable"].as_bool() == Some(true)
                        && first["resolved"].as_bool() == Some(false)
                })
                .count() as i64
        })
        .unwrap_or(0);

    let approved = client
        .get_mr_approval(repo, remote_id)
        .await
        .ok()
        .and_then(|v| v["approved"].as_bool())
        .unwrap_or(false);

    // The stored mrs.state is only the discovery value; re-read the live state so
    // a merged/closed MR stops reading "open" on Home.
    let state = client
        .get_mr_details(repo, remote_id)
        .await
        .ok()
        .and_then(|v| v["state"].as_str().map(|s| s.to_string()));

    let sig = MrSignals { ci, unresolved, approved, state };
    if let Ok(mut map) = MR_SIGNALS.lock() {
        map.insert(mr_id.to_string(), (std::time::Instant::now(), sig.clone()));
    }
    sig
}

#[tauri::command]
pub async fn resolve_mr_thread(
    mr_id: String,
    thread_id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<(), String> {
    let (mr, _wt, repo) = load_mr_context(&mr_id, &pool)
        .await
        .map_err(|e| e.to_string())?;

    let client = make_client(&repo);
    client
        .resolve_mr_thread(&repo, &mr.remote_id, &thread_id)
        .await
        .map_err(|e| e.to_string())
}
