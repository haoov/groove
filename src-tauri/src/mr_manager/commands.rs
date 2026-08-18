use sqlx::SqlitePool;

use crate::db::schema::{Mr, Repo, Worktree};
use super::gitlab::fetch_and_upsert_mrs;
use super::platform::make_client;

// ─── Shared lookups ───────────────────────────────────────────────────────────

/// Load the mr → worktree → repo chain for an MR id.
pub(super) async fn load_mr_context(
    mr_id: &str,
    pool: &SqlitePool,
) -> anyhow::Result<(Mr, Worktree, Repo)> {
    let mr: Mr = sqlx::query_as("SELECT * FROM mrs WHERE id = ?")
        .bind(mr_id)
        .fetch_one(pool)
        .await?;

    let wt = crate::db::load::worktree(pool, &mr.worktree_id).await?;

    let repo = crate::db::load::repo(pool, &wt.repo_id).await?;

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
    let wt = crate::db::load::worktree(&pool, &worktree_id).await
        .map_err(|e| e.to_string())?;

    let repo = crate::db::load::repo(&pool, &wt.repo_id).await
        .map_err(|e| e.to_string())?;

    if !repo.host.contains("github") {
        match fetch_and_upsert_mrs(&wt, &repo, &pool).await {
            Ok(mrs) => return Ok(mrs),
            Err(e) => tracing::warn!("glab mr list failed: {e}"),
        }
    }

    sqlx::query_as("SELECT * FROM mrs WHERE worktree_id = ?")
        .bind(&worktree_id)
        .fetch_all(&*pool)
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
    bridge: tauri::State<'_, crate::confirmation_bridge::Bridge>,
) -> Result<String, String> {
    let wt = crate::db::load::worktree(&pool, &worktree_id).await
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
        .post(&pool, crate::ops::MR_CREATE, payload, "ui", Some(&wt.task_id))
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
#[derive(Debug, Clone, serde::Serialize)]
pub struct MrSignals {
    /// Pipeline status ("success", "failed", …) — None when the MR has no pipeline.
    pub ci: Option<String>,
    pub unresolved: i64,
    /// Carries at least one approval (from anyone).
    pub approved: bool,
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

    let sig = MrSignals { ci, unresolved, approved };
    if let Ok(mut map) = MR_SIGNALS.lock() {
        map.insert(mr_id.to_string(), (std::time::Instant::now(), sig.clone()));
    }
    sig
}

// ─── Review queue ─────────────────────────────────────────────────────────────

/// An open MR where the current user is a reviewer, matched (by origin URL) to
/// its MAIN clone when one exists. TS mirror in types/ipc.ts.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReviewMr {
    /// Which forge this row came from: "gitlab" or "github". The UI reads it for
    /// the reference sigil (`!42` vs `#42`).
    pub platform: String,
    /// Full project path on the forge, e.g. "wiremind/devops/gitlab-ci-common".
    pub project_full: String,
    pub iid: u64,
    pub title: String,
    pub author: String,
    pub source_branch: String,
    pub target_branch: String,
    pub draft: bool,
    pub web_url: String,
    pub updated_at: String,
    /// MAIN clone path when the project is already cloned locally.
    pub local_path: Option<String>,
    /// Already approved (by anyone) but not merged — shown as a pill in Up next.
    pub approved: bool,
}

/// glab usernames are stable for a run, and differ per instance: the same person is
/// one login on gitlab.com and another on a company's GitLab. Keyed by host, so a
/// second instance is not answered with the first one's name.
static GITLAB_USERNAMES: std::sync::Mutex<Vec<(String, String)>> = std::sync::Mutex::new(Vec::new());

fn cached_username(host: &str) -> Option<String> {
    let cache = GITLAB_USERNAMES.lock().ok()?;
    cache.iter().find(|(h, _)| h == host).map(|(_, u)| u.clone())
}

fn cache_username(host: &str, username: &str) {
    if let Ok(mut cache) = GITLAB_USERNAMES.lock() {
        cache.push((host.to_string(), username.to_string()));
    }
}

/// Open MRs and PRs where the current user is a requested reviewer.
///
/// Every forge host in the pool is asked, and each query runs inside a clone that
/// belongs to THAT host: both CLIs read the instance out of the repository they run
/// in, so one shared working directory silently asked the wrong one — `glab` in a
/// GitHub clone answers for gitlab.com, as a different user, with no error. Which
/// half of the pool sorted first was all that stood between that and working.
///
/// Asking per host also means a self-managed GitLab and gitlab.com both report,
/// rather than whichever happened to come first.
///
/// One host failing (CLI missing, not logged in) must not blank the others, so each
/// is best-effort and only a total failure surfaces.
#[tauri::command]
pub async fn list_review_mrs() -> Result<Vec<ReviewMr>, String> {
    let main_repos = crate::git_engine::list_main_repos().await.map_err(|e| e.to_string())?;

    // Pool directory names do not mirror forge group paths — match by origin URL.
    let mut clone_by_host: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();
    let mut clone_by_project: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for r in &main_repos {
        if let Ok((host, group, project)) = crate::git_engine::parse_git_url(&r.url) {
            clone_by_host.entry(host.clone()).or_insert_with(|| r.local_path.clone());
            clone_by_project.insert(format!("{host}/{group}/{project}"), r.local_path.clone());
        }
    }
    if clone_by_host.is_empty() {
        return Err("no repos in the pool — clone one first".to_string());
    }

    let results = futures_util::future::join_all(clone_by_host.iter().map(|(host, cwd)| {
        let clones = &clone_by_project;
        async move {
            // Same rule as platform::make_client, so the queue and the MR views
            // cannot disagree about which forge a host is.
            let mrs = if host.contains("github") {
                list_github_reviews(host, cwd, clones).await
            } else {
                list_gitlab_reviews(host, cwd, clones).await
            };
            (host.clone(), mrs)
        }
    }))
    .await;

    let mut out = vec![];
    let mut errors = vec![];
    for (host, result) in results {
        match result {
            Ok(mut mrs) => out.append(&mut mrs),
            // No CLI for that forge means no repos on it — not something to report.
            Err(e) if super::platform::is_cli_missing(&e) => {
                tracing::debug!("{host} review queue skipped: {e}");
            }
            Err(e) => errors.push(format!("{host}: {e}")),
        }
    }
    if out.is_empty() && !errors.is_empty() {
        return Err(errors.join(" · "));
    }
    // Most recently touched first, across every host.
    out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(out)
}

/// Open PRs where the user is a requested reviewer. `reviewDecision` rides along in
/// the same query, so approval needs no extra call.
async fn list_github_reviews(
    host: &str,
    cwd: &str,
    clone_by_project: &std::collections::HashMap<String, String>,
) -> anyhow::Result<Vec<ReviewMr>> {
    let nodes = super::github::review_requested_prs(cwd).await?;
    Ok(nodes
        .iter()
        .filter_map(|pr| {
            let project_full = pr["repository"]["nameWithOwner"].as_str()?.to_string();
            Some(ReviewMr {
                local_path: clone_by_project.get(&format!("{host}/{project_full}")).cloned(),
                iid: pr["number"].as_u64()?,
                title: pr["title"].as_str().unwrap_or("").to_string(),
                author: pr["author"]["login"].as_str().unwrap_or("").to_string(),
                source_branch: pr["headRefName"].as_str().unwrap_or("").to_string(),
                target_branch: pr["baseRefName"].as_str().unwrap_or("").to_string(),
                draft: pr["isDraft"].as_bool().unwrap_or(false),
                web_url: pr["url"].as_str().unwrap_or("").to_string(),
                updated_at: pr["updatedAt"].as_str().unwrap_or("").to_string(),
                approved: pr["reviewDecision"].as_str() == Some("APPROVED"),
                platform: "github".into(),
                project_full,
            })
        })
        .collect())
}

async fn list_gitlab_reviews(
    host: &str,
    cwd: &str,
    clone_by_project: &std::collections::HashMap<String, String>,
) -> anyhow::Result<Vec<ReviewMr>> {
    let cwd = cwd.to_string();

    let username = match cached_username(host) {
        Some(u) => u,
        None => {
            let out = super::gitlab::glab_run(cwd.clone(), vec!["api".into(), "user".into()]).await?;
            let v: serde_json::Value = serde_json::from_str(&out)?;
            let u = v["username"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("no username in `glab api user` response"))?
                .to_string();
            cache_username(host, &u);
            u
        }
    };

    let endpoint =
        format!("merge_requests?reviewer_username={username}&state=opened&scope=all&per_page=50");
    let out = super::gitlab::glab_run(cwd.clone(), vec!["api".into(), endpoint]).await?;
    let items: Vec<serde_json::Value> = serde_json::from_str(&out)?;

    let mut result = vec![];
    for item in items {
        // references.full = "<group/project>!<iid>"
        let full = item["references"]["full"].as_str().unwrap_or("");
        let project_full = full.split('!').next().unwrap_or("").to_string();
        if project_full.is_empty() {
            continue;
        }
        result.push(ReviewMr {
            local_path: clone_by_project.get(&format!("{host}/{project_full}")).cloned(),
            iid: item["iid"].as_u64().unwrap_or(0),
            title: item["title"].as_str().unwrap_or("").to_string(),
            author: item["author"]["username"]
                .as_str()
                .or(item["author"]["name"].as_str())
                .unwrap_or("")
                .to_string(),
            source_branch: item["source_branch"].as_str().unwrap_or("").to_string(),
            target_branch: item["target_branch"].as_str().unwrap_or("").to_string(),
            draft: item["draft"].as_bool().or(item["work_in_progress"].as_bool()).unwrap_or(false),
            web_url: item["web_url"].as_str().unwrap_or("").to_string(),
            updated_at: item["updated_at"].as_str().unwrap_or("").to_string(),
            // Filled in concurrently below — one approvals call per MR.
            approved: false,
            platform: "gitlab".into(),
            project_full,
        });
    }

    // Approval isn't in the list payload, so fetch it per MR. Concurrent, so the
    // queue costs one extra round-trip in wall time rather than N.
    let flags = futures_util::future::join_all(
        result
            .iter()
            .map(|mr| super::gitlab::mr_approved(&cwd, &mr.project_full, mr.iid)),
    )
    .await;
    for (mr, approved) in result.iter_mut().zip(flags) {
        mr.approved = approved;
    }

    Ok(result)
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
