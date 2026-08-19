use std::path::PathBuf;
use sqlx::SqlitePool;
use tauri::Emitter;

use crate::core::db::models::{Repo, Worktree};
use crate::core::db::store;
use crate::core::git;
use super::types::BranchSpec;

/// Whether `branch` already exists as a head on the repo's `origin` remote.
/// Used to block creating a worktree on a branch name that's already taken remotely.
#[tauri::command]
pub async fn remote_branch_exists(
    repo_id: String,
    branch: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<bool, String> {
    let repo = store::repos::get(&*pool, &repo_id).await.map_err(|e| e.to_string())?;

    let out = crate::core::git::output(&repo.local_path, &["ls-remote", "--heads", "origin", &branch])
        .await
        .map_err(|e| e.to_string())?;

    let target = format!("refs/heads/{branch}");
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split('\t').nth(1))
        .any(|r| r == target))
}

#[tauri::command]
pub async fn register_repo(
    local_path: String,
    remote_url: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Repo, String> {
    register_repo_impl(local_path, remote_url, &pool)
        .await
        .map_err(|e| e.to_string())
}

pub(crate) async fn register_repo_impl(
    local_path: String,
    remote_url: String,
    pool: &SqlitePool,
) -> anyhow::Result<Repo> {
    let (host, group_path, project) = git::parse_git_url(&remote_url)?;
    let id = format!("{host}/{group_path}/{project}");

    if !local_path.is_empty() && !git::run::is_repository(&local_path).await {
        return Err(anyhow::anyhow!("Not a git repository at {local_path}"));
    }

    let repo = Repo { id, host, group_path, project, local_path };
    store::repos::upsert(pool, &repo).await?;
    Ok(repo)
}

#[tauri::command]
pub async fn provision_worktrees(
    task_id: String,
    branches: Vec<BranchSpec>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Vec<Worktree>, String> {
    let branch_name = crate::task_manager::derive_branch(&task_id);
    provision_worktrees_impl(&task_id, &branches, &branch_name, &pool)
        .await
        .map_err(|e| e.to_string())
}

pub(crate) async fn provision_worktrees_impl(
    session_id: &str,
    branches: &[BranchSpec],
    default_branch: &str,
    pool: &SqlitePool,
) -> anyhow::Result<Vec<Worktree>> {
    let mut created = vec![];

    for spec in branches {
        let repo = store::repos::get(pool, &spec.repo_id).await?;

        let branch = spec
            .branch_name
            .clone()
            .unwrap_or_else(|| default_branch.to_string());

        let wt_path = session_dir(session_id).join(&repo.project);

        std::fs::create_dir_all(&wt_path)?;

        if !git::run::is_repository(&repo.local_path).await {
            return Err(anyhow::anyhow!("Cannot open repo {}", repo.local_path));
        }

        // Fetch + fast-forward the MAIN clone before creating the branch, and use
        // the default branch it resolved — the same notion of "base" throughout.
        let default_branch = refresh_main_clone(&repo.local_path, &repo.project, session_id).await;
        ensure_branch(&branch, &repo.local_path, default_branch.as_deref()).await?;

        let wt_path_str = wt_path.to_string_lossy().to_string();
        let output =
            crate::core::git::output(&repo.local_path, &["worktree", "add", &wt_path_str, &branch]).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("already exists") {
                return Err(anyhow::anyhow!("git worktree add failed: {stderr}"));
            }
            align_checkout(&wt_path_str, &branch).await?;
        }

        git::cache::flush();
        let wt = store::worktrees::upsert(pool, session_id, &repo.id, &branch, &wt_path_str).await?;
        created.push(wt);
    }

    Ok(created)
}

/// A re-provisioned worktree may sit on a different branch than the one asked
/// for; upserting the new branch while the checkout keeps the old one would
/// desync DB and disk, so align the checkout first.
async fn align_checkout(wt_path: &str, branch: &str) -> anyhow::Result<()> {
    let current = crate::core::git::run(wt_path, &["rev-parse", "--abbrev-ref", "HEAD"])
        .await?
        .trim()
        .to_string();
    if current == branch {
        return Ok(());
    }
    // Plain switch when the branch exists locally, -c otherwise.
    let switch = crate::core::git::output(wt_path, &["switch", branch]).await?;
    if switch.status.success() {
        return Ok(());
    }
    let create = crate::core::git::output(wt_path, &["switch", "-c", branch]).await?;
    if !create.status.success() {
        return Err(anyhow::anyhow!(
            "worktree at {wt_path} is on branch '{current}' and switching to '{branch}' failed: {}",
            String::from_utf8_lossy(&create.stderr).trim()
        ));
    }
    Ok(())
}

/// Provision **detached** worktrees for an explorer session: each repo is checked
/// out at `origin/main` (or `origin/master`) with no branch, so exploration never
/// creates throwaway branches. Conversion to a task later runs `git switch -c`.
#[tauri::command]
pub async fn provision_explorer_worktrees(
    task_id: String,
    repo_ids: Vec<String>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Vec<Worktree>, String> {
    provision_explorer_worktrees_impl(&task_id, &repo_ids, &pool)
        .await
        .map_err(|e| e.to_string())
}

pub(crate) async fn provision_explorer_worktrees_impl(
    session_id: &str,
    repo_ids: &[String],
    pool: &SqlitePool,
) -> anyhow::Result<Vec<Worktree>> {
    let mut created = vec![];

    for repo_id in repo_ids {
        let repo = store::repos::get(pool, repo_id).await?;

        let wt_path = session_dir(session_id).join(&repo.project);
        std::fs::create_dir_all(&wt_path)?;

        let default_branch = refresh_main_clone(&repo.local_path, &repo.project, session_id).await;

        // Prefer the branch the refresh resolved; fall back to probing.
        let mut base = default_branch.map(|b| format!("origin/{b}"));
        for candidate in ["origin/main", "origin/master"] {
            if base.is_some() {
                break;
            }
            if crate::core::git::refs::ref_exists(&repo.local_path, candidate).await {
                base = Some(candidate.to_string());
            }
        }
        let base = base.ok_or_else(|| {
            anyhow::anyhow!("no default branch, origin/main or origin/master in {}", repo.project)
        })?;

        let wt_path_str = wt_path.to_string_lossy().to_string();
        let output = crate::core::git::output(
            &repo.local_path,
            &["worktree", "add", "--detach", &wt_path_str, &base],
        )
        .await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("already exists") {
                return Err(anyhow::anyhow!("git worktree add --detach failed: {stderr}"));
            }
        }

        let wt = store::worktrees::upsert(pool, session_id, &repo.id, "(detached)", &wt_path_str).await?;
        created.push(wt);
    }

    Ok(created)
}

/// Provision the worktree for a review session: check out the MR's source
/// branch (tracking origin) at `<root>/<session_id>/<project>` and pin the MR's
/// target branch as the diff/log base via `worktrees.base_ref`.
pub(crate) async fn provision_review_worktree(
    session_id: &str,
    repo: &Repo,
    source_branch: &str,
    target_branch: &str,
    pool: &SqlitePool,
) -> anyhow::Result<Worktree> {
    refresh_main_clone(&repo.local_path, &repo.project, session_id).await;
    // The default-branch fetch above may not cover the MR branches — fetch both.
    let out = crate::core::git::output(
        &repo.local_path,
        &["fetch", "origin", source_branch, target_branch],
    )
    .await?;
    git::cache::flush();
    if !out.status.success() {
        return Err(anyhow::anyhow!(
            "git fetch origin {source_branch} {target_branch} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }

    // Drop registrations whose directories are gone: reopening a review whose
    // folder was deleted would otherwise fail with "missing but already
    // registered worktree" instead of simply recreating it.
    let _ = crate::core::git::run(&repo.local_path, &["worktree", "prune"]).await;

    let wt_path = session_dir(session_id).join(&repo.project);
    std::fs::create_dir_all(&wt_path)?;
    let wt_path_str = wt_path.to_string_lossy().to_string();

    let local_exists =
        crate::core::git::refs::ref_exists(&repo.local_path, &format!("refs/heads/{source_branch}")).await;

    let output = if local_exists {
        crate::core::git::output(&repo.local_path, &["worktree", "add", &wt_path_str, source_branch]).await?
    } else {
        let track = format!("origin/{source_branch}");
        crate::core::git::output(
            &repo.local_path,
            &["worktree", "add", "--track", "-b", source_branch, &wt_path_str, &track],
        )
        .await?
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Reopening an existing review session lands here — the worktree is fine,
        // but it may sit on a different branch. Align, or the DB row below would
        // claim a branch the worktree isn't on.
        if !stderr.contains("already exists") {
            return Err(anyhow::anyhow!("git worktree add failed: {stderr}"));
        }
        align_checkout(&wt_path_str, source_branch).await?;
    }

    let wt = store::worktrees::upsert(pool, session_id, &repo.id, source_branch, &wt_path_str).await?;
    store::worktrees::set_base_ref(pool, &wt.id, target_branch).await?;
    Ok(Worktree { base_ref: Some(target_branch.to_string()), ..wt })
}

/// Create `branch` if it doesn't exist yet, based on the repo's default branch.
///
/// `base_branch` is what `refresh_main_clone` resolved; main/master are only a
/// fallback for when it couldn't say. Guessing that list alone used to fail on any
/// repo whose default is something else, e.g. `develop`.
async fn ensure_branch(
    branch: &str,
    repo_path: &str,
    base_branch: Option<&str>,
) -> anyhow::Result<()> {
    if crate::core::git::refs::ref_exists(repo_path, &format!("refs/heads/{branch}")).await {
        return Ok(());
    }

    let mut candidates: Vec<String> = vec![];
    if let Some(base) = base_branch {
        candidates.push(format!("origin/{base}"));
    }
    candidates.extend(["origin/main".to_string(), "origin/master".to_string()]);

    for base in &candidates {
        if let Ok(out) = crate::core::git::output(repo_path, &["branch", branch, base]).await {
            if out.status.success() {
                return Ok(());
            }
        }
    }

    Err(anyhow::anyhow!(
        "Cannot create branch {branch}: none of {} resolve in {repo_path}",
        candidates.join(", ")
    ))
}

pub fn resolve_worktree_root() -> PathBuf {
    // Honor the configured root (the agent cwd already does) — tilde-expanded,
    // since the config stores it unexpanded.
    if let Some(cfg) = crate::core::config::get() {
        let raw = cfg.git.worktree_root;
        if !raw.trim().is_empty() {
            return PathBuf::from(crate::agent_manager::expand_tilde(&raw));
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join("worktrees")
}

/// Where the primary clones live. This directory IS the repo pool — the pickers
/// list it, `clone_repo` fills it.
pub(crate) fn main_root() -> PathBuf {
    resolve_worktree_root().join("main")
}

/// A clone's place in the pool: `<root>/main/<host>/<group>/<project>`.
///
/// The host is a level of its own, so two forges — or two GitLab instances — can
/// hold the same group and project without colliding, and the pool says where a
/// repo came from without anyone opening its remote.
pub(super) fn repo_dir(host: &str, group_path: &str, project: &str) -> PathBuf {
    main_root().join(host).join(group_path).join(project)
}

/// Every worktree of one session: `<root>/worktrees/<session id>`.
pub(crate) fn session_dir(session_id: &str) -> PathBuf {
    resolve_worktree_root().join("worktrees").join(session_id)
}

/// Delete a stray local branch literally named `HEAD` — it makes every `HEAD`
/// reference ambiguous, breaking switch/rev-parse in the clone AND all of its
/// worktrees (refs are shared). Older builds created one via `fetch HEAD:HEAD`.
/// Best-effort; safe to call from a worktree path.
pub(crate) async fn repair_head_branch(repo_or_wt_path: &str) {
    if crate::core::git::refs::ref_exists(repo_or_wt_path, "refs/heads/HEAD").await {
        tracing::warn!("[git] deleting stray local branch 'HEAD' at {repo_or_wt_path}");
        let _ = crate::core::git::run(repo_or_wt_path, &["update-ref", "-d", "refs/heads/HEAD"]).await;
    }
}

/// Bring the MAIN clone up to date before branching off it, and return the default
/// branch so the caller branches from the ref that was just fetched.
///
/// Failures here are REPORTED, not swallowed. A silent fetch failure (off VPN,
/// expired credentials) would hand back a worktree quietly based on last week's
/// main, with nothing to explain why — which is worse than the delay of noticing.
async fn refresh_main_clone(repo_path: &str, repo_label: &str, session_id: &str) -> Option<String> {
    let fetched = crate::core::git::run(repo_path, &["fetch", "origin"]).await;
    git::cache::flush();
    if let Err(e) = fetched {
        crate::events::notice(
            "error",
            "git",
            format!("Could not fetch {repo_label} — its worktree may be based on stale history"),
            Some(e.to_string()),
            Some(session_id),
        );
        // Carry on: branching from whatever is on disk still beats failing outright,
        // now that the staleness has been named.
    }
    repair_head_branch(repo_path).await;

    let default_branch = git::refs::default_branch(repo_path).await?;

    let current = crate::core::git::run(repo_path, &["rev-parse", "--abbrev-ref", "HEAD"])
        .await
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    let result = if current == default_branch {
        crate::core::git::run(repo_path, &["pull", "--ff-only"]).await
    } else {
        // Advance the local branch ref without touching MAIN's checkout.
        crate::core::git::run(repo_path, &["fetch", "origin", &format!("{default_branch}:{default_branch}")]).await
    };
    if let Err(e) = result {
        // The common cause is local commits or a dirty tree in the MAIN clone, which
        // the user has to resolve there — so say which repo and which branch.
        crate::events::notice(
            "attention",
            "git",
            format!("{repo_label}: {default_branch} in MAIN could not fast-forward"),
            Some(format!("{e}\n\nNew branches still come from origin/{default_branch}, so this only affects MAIN's own checkout.")),
            Some(session_id),
        );
    }
    Some(default_branch)
}

/// All git clones in the pool (searched a few levels deep, since a host level sits
/// above group paths that nest; never descends INTO a repo). No pool → empty list.
#[tauri::command]
pub async fn list_main_repos() -> Result<Vec<super::types::MainRepo>, String> {
    let root = main_root();
    // Walk on a blocking thread — it's pure filesystem.
    let found: Vec<PathBuf> = tokio::task::spawn_blocking(move || {
        fn walk(dir: &std::path::Path, depth: u32, acc: &mut Vec<PathBuf>) {
            if depth > 6 {
                return;
            }
            let Ok(entries) = std::fs::read_dir(dir) else { return };
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                if path.join(".git").exists() {
                    acc.push(path); // a repo — don't descend into it
                } else {
                    walk(&path, depth + 1, acc);
                }
            }
        }
        let mut acc = vec![];
        walk(&root, 1, &mut acc);
        acc
    })
    .await
    .map_err(|e| e.to_string())?;

    let root = main_root();
    // One `git remote get-url` per clone, all at once. In a row they cost 124 ms on a
    // 35-repo pool, and this runs on every review-queue poll and every time a repo
    // picker opens — the answer is the same either way, so it may as well be prompt.
    let mut repos: Vec<super::types::MainRepo> =
        futures_util::future::join_all(found.into_iter().map(|path| {
            let root = root.clone();
            async move {
                let local_path = path.to_string_lossy().to_string();
                let slug = path
                    .strip_prefix(&root)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| local_path.clone());
                // A clone without an origin remote can't be registered — skip it.
                let url = crate::core::git::run(&local_path, &["remote", "get-url", "origin"]).await.ok()?;
                Some(super::types::MainRepo { url: url.trim().to_string(), local_path, slug })
            }
        }))
        .await
        .into_iter()
        .flatten()
        .collect();
    repos.sort_by(|a, b| a.slug.to_lowercase().cmp(&b.slug.to_lowercase()));
    Ok(repos)
}

/// Clone a repo into the pool and return it. The clone can
/// be slow — it runs off the async runtime; the UI shows a pending state.
#[tauri::command]
pub async fn clone_repo(url: String) -> Result<super::types::MainRepo, String> {
    let (host, group_path, project) = git::parse_git_url(&url).map_err(|e| e.to_string())?;
    let dest = repo_dir(&host, &group_path, &project);
    if dest.exists() {
        return Err(format!("{} already exists", dest.display()));
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let dest_str = dest.to_string_lossy().to_string();
    let parent = dest.parent().unwrap_or(&dest).to_string_lossy().to_string();
    crate::core::git::run(&parent, &["clone", &url, &dest_str])
        .await
        .map_err(|e| e.to_string())?;

    Ok(super::types::MainRepo {
        url,
        local_path: dest_str,
        slug: format!("{host}/{group_path}/{project}"),
    })
}

/// Delete a worktree's directory and prune its clone's registration.
/// Disk only — DB rows are the store's job.
async fn remove_worktree_dir(wt_path: String, repo_local_path: Option<String>) {
    let _ = tokio::task::spawn_blocking(move || std::fs::remove_dir_all(wt_path)).await;
    if let Some(local_path) = repo_local_path {
        let _ = git::run(&local_path, &["worktree", "prune"]).await;
    }
}

/// Remove every worktree directory of a session and the session directory
/// itself. The DB rows cascade when the session row is deleted.
pub async fn cleanup_session_worktrees(session_id: &str, pool: &SqlitePool) -> anyhow::Result<()> {
    for wt in store::worktrees::for_session(pool, session_id).await? {
        let repo = store::repos::get_opt(pool, &wt.repo_id).await?;
        remove_worktree_dir(wt.path, repo.map(|r| r.local_path)).await;
    }

    let dir = session_dir(session_id);
    let _ = tokio::task::spawn_blocking(move || {
        let _ = std::fs::remove_dir_all(&dir);
    })
    .await;

    Ok(())
}

#[tauri::command]
pub async fn close_worktree(
    app: tauri::AppHandle,
    worktree_id: String,
    force: Option<bool>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<(), String> {
    close_worktree_impl(&app, &worktree_id, force, &pool)
        .await
        .map_err(|e| e.to_string())
}

async fn close_worktree_impl(
    app: &tauri::AppHandle,
    worktree_id: &str,
    force: Option<bool>,
    pool: &SqlitePool,
) -> anyhow::Result<()> {
    let wt = store::worktrees::get(pool, worktree_id).await?;

    // Guard against destroying uncommitted work: unless forced, refuse to close a
    // worktree with a dirty working tree (the frontend re-invokes with force after
    // a confirm).
    if force != Some(true) {
        let dirty = crate::core::git::output(&wt.path, &["status", "--porcelain"])
            .await
            .map(|o| o.status.success() && !o.stdout.is_empty())
            .unwrap_or(false);
        if dirty {
            return Err(anyhow::anyhow!(
                "worktree has uncommitted changes — commit or discard first, or force close"
            ));
        }
    }

    let repo = store::repos::get_opt(pool, &wt.repo_id).await?;
    remove_worktree_dir(wt.path.clone(), repo.map(|r| r.local_path)).await;
    git::cache::flush();

    let closed = store::worktrees::close(pool, worktree_id).await?;

    app.emit(
        crate::events::WORKTREE_CLOSED,
        serde_json::json!({
            "worktree_id": worktree_id,
            "session_id": closed.session_id,
            "repo_id": closed.repo_id,
        }),
    )?;

    Ok(())
}
