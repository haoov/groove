//! One provisioning path for every session kind.
//!
//! A task worktree sits on `<type>/<id>-<slug>` (user-overridable), an
//! explorer's on `explorer/<name-slug>` — both created off the repo's default
//! branch. A review worktree checks out the MR's source branch (tracking
//! origin) with the MR's target pinned as the diff/log base. All of them live
//! at `<session>/<project>@<branch-slug>`.

use serde::Deserialize;
use sqlx::SqlitePool;

use crate::core::db::models::{Repo, Session, Worktree};
use crate::core::db::store;
use crate::core::git;
use super::naming;
use super::pool::session_dir;

#[derive(Debug, Clone, Deserialize)]
pub struct BranchSpec {
    pub repo_id: String,
    /// None → the session-kind default from `naming::default_branch`.
    pub branch_name: Option<String>,
}

#[tauri::command]
pub async fn provision_worktrees(
    task_id: String,
    branches: Vec<BranchSpec>,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Vec<Worktree>, String> {
    provision_worktrees_impl(&task_id, &branches, &pool)
        .await
        .map_err(|e| e.to_string())
}

/// Provision one worktree per spec, concurrently — each repo's fetch is
/// network-bound and independent.
pub(crate) async fn provision_worktrees_impl(
    session_id: &str,
    branches: &[BranchSpec],
    pool: &SqlitePool,
) -> anyhow::Result<Vec<Worktree>> {
    let session = store::sessions::get(pool, session_id).await?;

    let tag = store::provider_tasks::get_by_short_id(pool, &session.id)
        .await
        .ok()
        .flatten()
        .and_then(|t| t.branch_tag);

    futures_util::future::try_join_all(branches.iter().map(|spec| {
        let session = &session;
        let tag = tag.as_deref();
        async move {
            let repo = store::repos::get(pool, &spec.repo_id).await?;
            let branch = spec
                .branch_name
                .clone()
                .filter(|b| !b.trim().is_empty())
                .unwrap_or_else(|| naming::default_branch(session, tag));
            provision_one(pool, session, &repo, &branch, None).await
        }
    }))
    .await
}

/// Provision the worktree for a review session: check out the MR's source
/// branch and pin the MR's target as the diff/log base via `base_ref`.
pub(crate) async fn provision_review_worktree(
    session_id: &str,
    repo: &Repo,
    source_branch: &str,
    target_branch: &str,
    pool: &SqlitePool,
) -> anyhow::Result<Worktree> {
    let session = store::sessions::get(pool, session_id).await?;

    // The default-branch fetch inside provisioning may not cover the MR
    // branches — fetch both explicitly first.
    let out = git::output(
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

    let wt = provision_one(pool, &session, repo, source_branch, Some(source_branch)).await?;
    store::worktrees::set_base_ref(pool, &wt.id, target_branch).await?;
    Ok(Worktree { base_ref: Some(target_branch.to_string()), ..wt })
}

/// The shared core: freshen the clone, make sure the branch exists, add the
/// worktree at its place, record it. Idempotent — a worktree this session
/// already has for this branch is reused wherever its directory sits (which
/// also keeps pre-rework directory names working).
async fn provision_one(
    pool: &SqlitePool,
    session: &Session,
    repo: &Repo,
    branch: &str,
    track_remote: Option<&str>,
) -> anyhow::Result<Worktree> {
    if let Some(existing) = existing_worktree(pool, session, repo, branch).await? {
        align_checkout(&existing.path, branch).await?;
        return Ok(existing);
    }

    if !git::run::is_repository(&repo.local_path).await {
        return Err(anyhow::anyhow!("Cannot open repo {}", repo.local_path));
    }

    // Reopening a review whose folder was deleted would otherwise fail with
    // "missing but already registered worktree" instead of recreating it.
    let _ = git::run(&repo.local_path, &["worktree", "prune"]).await;

    let default_branch = refresh_main_clone(&repo.local_path, &repo.project, &session.id).await;

    let wt_path = session_dir(&session.id).join(naming::worktree_dir(&repo.project, branch));
    std::fs::create_dir_all(&wt_path)?;
    let wt_path_str = wt_path.to_string_lossy().to_string();

    let local_exists = git::refs::ref_exists(&repo.local_path, &format!("refs/heads/{branch}")).await;
    let output = match track_remote {
        // Review: a branch we don't have yet tracks its origin counterpart.
        Some(remote_branch) if !local_exists => {
            let track = format!("origin/{remote_branch}");
            git::output(
                &repo.local_path,
                &["worktree", "add", "--track", "-b", branch, &wt_path_str, &track],
            )
            .await?
        }
        _ => {
            if !local_exists {
                create_branch(branch, &repo.local_path, default_branch.as_deref()).await?;
            }
            git::output(&repo.local_path, &["worktree", "add", &wt_path_str, branch]).await?
        }
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("already exists") {
            return Err(anyhow::anyhow!("git worktree add failed: {stderr}"));
        }
        align_checkout(&wt_path_str, branch).await?;
    }

    git::cache::flush();
    Ok(store::worktrees::upsert(pool, &session.id, &repo.id, branch, &wt_path_str).await?)
}

/// The session's existing worktree for this (repo, branch), when its directory
/// is still on disk.
async fn existing_worktree(
    pool: &SqlitePool,
    session: &Session,
    repo: &Repo,
    branch: &str,
) -> anyhow::Result<Option<Worktree>> {
    Ok(store::worktrees::for_repo(pool, &session.id, &repo.id)
        .await?
        .into_iter()
        .find(|wt| wt.branch == branch && std::path::Path::new(&wt.path).is_dir()))
}

/// A re-provisioned worktree may sit on a different branch than the one asked
/// for; recording the new branch while the checkout keeps the old one would
/// desync DB and disk, so align the checkout first.
async fn align_checkout(wt_path: &str, branch: &str) -> anyhow::Result<()> {
    let current = git::run(wt_path, &["rev-parse", "--abbrev-ref", "HEAD"])
        .await?
        .trim()
        .to_string();
    if current == branch {
        return Ok(());
    }
    // Plain switch when the branch exists locally, -c otherwise.
    let switch = git::output(wt_path, &["switch", branch]).await?;
    if switch.status.success() {
        git::cache::flush();
        return Ok(());
    }
    let create = git::output(wt_path, &["switch", "-c", branch]).await?;
    if !create.status.success() {
        return Err(anyhow::anyhow!(
            "worktree at {wt_path} is on branch '{current}' and switching to '{branch}' failed: {}",
            String::from_utf8_lossy(&create.stderr).trim()
        ));
    }
    git::cache::flush();
    Ok(())
}

/// Create `branch` off the repo's default branch.
///
/// `base_branch` is what `refresh_main_clone` resolved; main/master are only a
/// fallback for when it couldn't say. Guessing that list alone used to fail on
/// any repo whose default is something else, e.g. `develop`.
async fn create_branch(
    branch: &str,
    repo_path: &str,
    base_branch: Option<&str>,
) -> anyhow::Result<()> {
    let mut candidates: Vec<String> = vec![];
    if let Some(base) = base_branch {
        candidates.push(format!("origin/{base}"));
    }
    candidates.extend(["origin/main".to_string(), "origin/master".to_string()]);

    for base in &candidates {
        if let Ok(out) = git::output(repo_path, &["branch", branch, base]).await {
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

/// Delete a stray local branch literally named `HEAD` — it makes every `HEAD`
/// reference ambiguous, breaking switch/rev-parse in the clone AND all of its
/// worktrees (refs are shared). Older builds created one via `fetch HEAD:HEAD`.
/// Best-effort; safe to call from a worktree path.
pub(crate) async fn repair_head_branch(repo_or_wt_path: &str) {
    if git::refs::ref_exists(repo_or_wt_path, "refs/heads/HEAD").await {
        tracing::warn!("[git] deleting stray local branch 'HEAD' at {repo_or_wt_path}");
        let _ = git::run(repo_or_wt_path, &["update-ref", "-d", "refs/heads/HEAD"]).await;
    }
}

/// Bring the MAIN clone up to date before branching off it, and return the
/// default branch so the caller branches from the ref that was just fetched.
///
/// Failures here are REPORTED, not swallowed. A silent fetch failure (off VPN,
/// expired credentials) would hand back a worktree quietly based on last week's
/// main, with nothing to explain why.
async fn refresh_main_clone(repo_path: &str, repo_label: &str, session_id: &str) -> Option<String> {
    let fetched = git::run(repo_path, &["fetch", "origin"]).await;
    git::cache::flush();
    if let Err(e) = fetched {
        crate::core::events::notice(
            "error",
            "git",
            format!("Could not fetch {repo_label} — its worktree may be based on stale history"),
            Some(e.to_string()),
            Some(session_id),
        );
        // Carry on: branching from whatever is on disk still beats failing
        // outright, now that the staleness has been named.
    }
    repair_head_branch(repo_path).await;

    let default_branch = git::refs::default_branch(repo_path).await?;

    let current = git::run(repo_path, &["rev-parse", "--abbrev-ref", "HEAD"])
        .await
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    let result = if current == default_branch {
        git::run(repo_path, &["pull", "--ff-only"]).await
    } else {
        // Advance the local branch ref without touching MAIN's checkout.
        git::run(repo_path, &["fetch", "origin", &format!("{default_branch}:{default_branch}")]).await
    };
    if let Err(e) = result {
        // The common cause is local commits or a dirty tree in the MAIN clone,
        // which the user has to resolve there — so say which repo and branch.
        crate::core::events::notice(
            "attention",
            "git",
            format!("{repo_label}: {default_branch} in MAIN could not fast-forward"),
            Some(format!("{e}\n\nNew branches still come from origin/{default_branch}, so this only affects MAIN's own checkout.")),
            Some(session_id),
        );
    }
    Some(default_branch)
}
