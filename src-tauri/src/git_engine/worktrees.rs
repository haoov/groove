use std::path::PathBuf;
use sqlx::SqlitePool;
use tauri::Emitter;
use crate::db::schema::{Repo, Worktree};
use super::types::BranchSpec;

pub(crate) fn parse_git_url(url: &str) -> anyhow::Result<(String, String, String)> {
    let url = url.trim_end_matches(".git");

    // SSH: git@host:group/project
    if let Some(at) = url.find('@') {
        let rest = &url[at + 1..];
        if let Some(colon) = rest.find(':') {
            let host = &rest[..colon];
            let path = &rest[colon + 1..];
            if let Some(slash) = path.rfind('/') {
                return Ok((
                    host.to_string(),
                    path[..slash].to_string(),
                    path[slash + 1..].to_string(),
                ));
            }
        }
    }

    // HTTPS: https://host/group/project
    let stripped = url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    if let Some(slash) = stripped.find('/') {
        let host = &stripped[..slash];
        let path = &stripped[slash + 1..];
        if let Some(last_slash) = path.rfind('/') {
            return Ok((
                host.to_string(),
                path[..last_slash].to_string(),
                path[last_slash + 1..].to_string(),
            ));
        }
    }

    Err(anyhow::anyhow!("Cannot parse git URL: {url}"))
}

/// Whether `branch` already exists as a head on the repo's `origin` remote.
/// Used to block creating a worktree on a branch name that's already taken remotely.
#[tauri::command]
pub async fn remote_branch_exists(
    repo_id: String,
    branch: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<bool, String> {
    let repo = crate::db::load::repo(&pool, &repo_id).await
        .map_err(|e| e.to_string())?;

    let local_path = repo.local_path;
    let branch_q = branch.clone();
    let out = tokio::task::spawn_blocking(move || {
        std::process::Command::new("git")
            .args(["ls-remote", "--heads", "origin", &branch_q])
            .current_dir(&local_path)
            .output()
    })
    .await
    .map_err(|e| e.to_string())?
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
    let (host, group_path, project) = parse_git_url(&remote_url)?;
    let id = format!("{host}/{group_path}/{project}");

    if !local_path.is_empty() {
        git2::Repository::open(&local_path)
            .map_err(|e| anyhow::anyhow!("Not a git repository at {local_path}: {e}"))?;
    }

    let repo = Repo {
        id: id.clone(),
        host,
        group_path,
        project,
        local_path: local_path.clone(),
    };

    sqlx::query(
        "INSERT INTO repos (id, host, group_path, project, local_path)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET local_path = excluded.local_path",
    )
    .bind(&repo.id)
    .bind(&repo.host)
    .bind(&repo.group_path)
    .bind(&repo.project)
    .bind(&repo.local_path)
    .execute(pool)
    .await?;

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
    task_id: &str,
    branches: &[BranchSpec],
    default_branch: &str,
    pool: &SqlitePool,
) -> anyhow::Result<Vec<Worktree>> {
    let mut created = vec![];

    for spec in branches {
        let repo = crate::db::load::repo(pool, &spec.repo_id).await?;

        let branch = spec
            .branch_name
            .clone()
            .unwrap_or_else(|| default_branch.to_string());

        let wt_path = task_dir(task_id).join(&repo.project);

        std::fs::create_dir_all(&wt_path)?;

        {
            let local_path = repo.local_path.clone();
            tokio::task::spawn_blocking(move || {
                git2::Repository::open(&local_path)
                    .map(|_| ())
                    .map_err(|e| anyhow::anyhow!("Cannot open repo {local_path}: {e}"))
            })
            .await??;
        }

        // Fetch + fast-forward the MAIN clone before creating the branch, and use
        // the default branch it resolved — the same notion of "base" throughout.
        let default_branch = refresh_main_clone(&repo.local_path, &repo.project, task_id).await;
        ensure_branch(&branch, &repo.local_path, default_branch.as_deref()).await?;

        let wt_path_str = wt_path.to_string_lossy().to_string();
        let output =
            super::run_git_output(&repo.local_path, &["worktree", "add", &wt_path_str, &branch]).await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("already exists") {
                return Err(anyhow::anyhow!("git worktree add failed: {stderr}"));
            }
            // The worktree already exists — but possibly on a different branch
            // (e.g. re-provisioned with a new name). Upserting the new branch
            // while the checkout keeps the old one would desync DB and disk,
            // so align the checkout first.
            let current = super::run_git(&wt_path_str, &["rev-parse", "--abbrev-ref", "HEAD"])
                .await?
                .trim()
                .to_string();
            if current != branch {
                // Plain switch when the branch exists locally, -c otherwise.
                let sw = super::run_git_output(&wt_path_str, &["switch", &branch]).await?;
                if !sw.status.success() {
                    let sw_c = super::run_git_output(&wt_path_str, &["switch", "-c", &branch]).await?;
                    if !sw_c.status.success() {
                        return Err(anyhow::anyhow!(
                            "worktree at {wt_path_str} is on branch '{current}' and switching to '{branch}' failed: {}",
                            String::from_utf8_lossy(&sw_c.stderr).trim()
                        ));
                    }
                }
            }
        }

        let wt = super::upsert_worktree(task_id, &repo.id, &branch, &wt_path_str, pool).await?;
        created.push(wt);
    }

    Ok(created)
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
    task_id: &str,
    repo_ids: &[String],
    pool: &SqlitePool,
) -> anyhow::Result<Vec<Worktree>> {
    let mut created = vec![];

    for repo_id in repo_ids {
        let repo = crate::db::load::repo(pool, repo_id).await?;

        let wt_path = task_dir(task_id).join(&repo.project);
        std::fs::create_dir_all(&wt_path)?;

        let default_branch = refresh_main_clone(&repo.local_path, &repo.project, task_id).await;

        // Prefer the branch the refresh resolved; fall back to probing.
        let mut base = default_branch.map(|b| format!("origin/{b}"));
        for r in ["origin/main", "origin/master"] {
            if base.is_some() {
                break;
            }
            let ok = super::run_git_output(&repo.local_path, &["rev-parse", "--verify", "--quiet", r])
                .await
                .map(|o| o.status.success())
                .unwrap_or(false);
            if ok {
                base = Some(r.to_string());
                break;
            }
        }
        let base = base.ok_or_else(|| {
            anyhow::anyhow!("no default branch, origin/main or origin/master in {}", repo.project)
        })?;

        let wt_path_str = wt_path.to_string_lossy().to_string();
        let output = super::run_git_output(
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

        let wt = super::upsert_worktree(task_id, &repo.id, "(detached)", &wt_path_str, pool).await?;
        created.push(wt);
    }

    Ok(created)
}

/// Provision the worktree for a review session: check out the MR's source
/// branch (tracking origin) at `<root>/<task_id>/<project>` and pin the MR's
/// target branch as the diff/log base via `worktrees.base_ref`.
pub(crate) async fn provision_review_worktree(
    task_id: &str,
    repo: &Repo,
    source_branch: &str,
    target_branch: &str,
    pool: &SqlitePool,
) -> anyhow::Result<Worktree> {
    refresh_main_clone(&repo.local_path, &repo.project, task_id).await;
    // The default-branch fetch above may not cover the MR branches — fetch both.
    let out = super::run_git_output(
        &repo.local_path,
        &["fetch", "origin", source_branch, target_branch],
    )
    .await?;
    if !out.status.success() {
        return Err(anyhow::anyhow!(
            "git fetch origin {source_branch} {target_branch} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }

    // Drop registrations whose directories are gone: reopening a review whose
    // folder was deleted would otherwise fail with "missing but already
    // registered worktree" instead of simply recreating it.
    let _ = super::run_git(&repo.local_path, &["worktree", "prune"]).await;

    let wt_path = task_dir(task_id).join(&repo.project);
    std::fs::create_dir_all(&wt_path)?;
    let wt_path_str = wt_path.to_string_lossy().to_string();

    let local_exists = super::run_git_output(
        &repo.local_path,
        &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{source_branch}")],
    )
    .await?
    .status
    .success();

    let output = if local_exists {
        super::run_git_output(&repo.local_path, &["worktree", "add", &wt_path_str, source_branch]).await?
    } else {
        let track = format!("origin/{source_branch}");
        super::run_git_output(
            &repo.local_path,
            &["worktree", "add", "--track", "-b", source_branch, &wt_path_str, &track],
        )
        .await?
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Reopening an existing review session lands here — the worktree is fine.
        if !stderr.contains("already exists") {
            return Err(anyhow::anyhow!("git worktree add failed: {stderr}"));
        }
        // …but it may sit on a different branch (the reviewer checked something
        // out, or an older session left it behind). Align the checkout, or the DB
        // row below would claim a branch the worktree isn't on.
        let current = super::run_git(&wt_path_str, &["rev-parse", "--abbrev-ref", "HEAD"])
            .await
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        if current != source_branch {
            let sw = super::run_git_output(&wt_path_str, &["switch", source_branch]).await?;
            if !sw.status.success() {
                return Err(anyhow::anyhow!(
                    "worktree at {wt_path_str} is on '{current}' and switching to '{source_branch}' failed: {}",
                    String::from_utf8_lossy(&sw.stderr).trim()
                ));
            }
        }
    }

    let wt = super::upsert_worktree(task_id, &repo.id, source_branch, &wt_path_str, pool).await?;
    sqlx::query("UPDATE worktrees SET base_ref = ? WHERE id = ?")
        .bind(target_branch)
        .bind(&wt.id)
        .execute(pool)
        .await?;
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
    let exists = super::run_git_output(
        repo_path,
        &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{branch}")],
    )
    .await?
    .status
    .success();
    if exists {
        return Ok(());
    }

    let mut candidates: Vec<String> = vec![];
    if let Some(b) = base_branch {
        candidates.push(format!("origin/{b}"));
    }
    candidates.extend(["origin/main".to_string(), "origin/master".to_string()]);

    for base in &candidates {
        let output = super::run_git_output(repo_path, &["branch", branch, base]).await;
        if let Ok(out) = output {
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
    if let Some(cfg) = crate::task_manager::global_config() {
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

/// Every worktree of one task: `<root>/worktrees/<task or explorer id>`.
///
/// A sibling of the clone pool rather than the root itself, which used to collect a
/// directory per task anyone had ever opened, beside MAIN.
pub(crate) fn task_dir(task_id: &str) -> PathBuf {
    resolve_worktree_root().join("worktrees").join(task_id)
}

/// Delete a stray local branch literally named `HEAD` — it makes every `HEAD`
/// reference ambiguous, breaking switch/rev-parse in the clone AND all of its
/// worktrees (refs are shared). Older builds created one via `fetch HEAD:HEAD`.
/// Best-effort; safe to call from a worktree path.
pub(crate) async fn repair_head_branch(repo_or_wt_path: &str) {
    let poisoned =
        super::run_git_output(repo_or_wt_path, &["rev-parse", "--verify", "--quiet", "refs/heads/HEAD"])
            .await
            .map(|o| o.status.success())
            .unwrap_or(false);
    if poisoned {
        tracing::warn!("[git] deleting stray local branch 'HEAD' at {repo_or_wt_path}");
        let _ = super::run_git(repo_or_wt_path, &["update-ref", "-d", "refs/heads/HEAD"]).await;
    }
}

/// Freshen a MAIN clone during provisioning: fetch, then fast-forward its local
/// default branch (pull when it's checked out, ref-update otherwise). All
/// best-effort — worktrees are created from origin/<default> regardless.
/// The repo's real default branch.
///
/// Resolved from `refs/remotes/origin/HEAD`, and NEVER by stripping the
/// "origin/HEAD" shorthand — that yields "HEAD", and `fetch HEAD:HEAD` is exactly
/// what creates the poisoned branch `repair_head_branch` cleans up.
///
/// When the symref is missing (a `--single-branch` clone, or one made before the
/// remote had a default), this asks the remote and writes it, so the answer is
/// deterministic next time instead of a guess between main and master.
async fn resolve_default_branch(repo_path: &str) -> Option<String> {
    async fn read_symref(repo_path: &str) -> Option<String> {
        let out = super::run_git(repo_path, &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"])
            .await
            .ok()?;
        match out.trim().strip_prefix("origin/") {
            Some(name) if !name.is_empty() && name != "HEAD" => Some(name.to_string()),
            _ => None,
        }
    }

    if let Some(name) = read_symref(repo_path).await {
        return Some(name);
    }

    // Ask the remote and record the answer.
    let _ = super::run_git(repo_path, &["remote", "set-head", "origin", "-a"]).await;
    if let Some(name) = read_symref(repo_path).await {
        return Some(name);
    }

    // Offline, or a remote that won't say: fall back to whichever exists.
    for name in ["main", "master"] {
        let ok = super::run_git_output(
            repo_path,
            &["rev-parse", "--verify", "--quiet", &format!("refs/remotes/origin/{name}")],
        )
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);
        if ok {
            return Some(name.to_string());
        }
    }
    None
}

/// Bring the MAIN clone up to date before branching off it, and return the default
/// branch so the caller branches from the ref that was just fetched.
///
/// Failures here are REPORTED, not swallowed. A silent fetch failure (off VPN,
/// expired credentials) would hand back a worktree quietly based on last week's
/// main, with nothing to explain why — which is worse than the delay of noticing.
async fn refresh_main_clone(repo_path: &str, repo_label: &str, task_id: &str) -> Option<String> {
    if let Err(e) = super::run_git(repo_path, &["fetch", "origin"]).await {
        crate::events::notice(
            "error",
            "git",
            format!("Could not fetch {repo_label} — its worktree may be based on stale history"),
            Some(e.to_string()),
            Some(task_id),
        );
        // Carry on: branching from whatever is on disk still beats failing outright,
        // now that the staleness has been named.
    }
    repair_head_branch(repo_path).await;

    let default_branch = resolve_default_branch(repo_path).await?;

    let current = super::run_git(repo_path, &["rev-parse", "--abbrev-ref", "HEAD"])
        .await
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    let result = if current == default_branch {
        super::run_git(repo_path, &["pull", "--ff-only"]).await
    } else {
        // Advance the local branch ref without touching MAIN's checkout.
        super::run_git(repo_path, &["fetch", "origin", &format!("{default_branch}:{default_branch}")]).await
    };
    if let Err(e) = result {
        // The common cause is local commits or a dirty tree in the MAIN clone, which
        // the user has to resolve there — so say which repo and which branch.
        crate::events::notice(
            "attention",
            "git",
            format!("{repo_label}: {default_branch} in MAIN could not fast-forward"),
            Some(format!("{e}\n\nNew branches still come from origin/{default_branch}, so this only affects MAIN's own checkout.")),
            Some(task_id),
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
    let mut repos = vec![];
    for path in found {
        let local_path = path.to_string_lossy().to_string();
        let slug = path
            .strip_prefix(&root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| local_path.clone());
        // A clone without an origin remote can't be registered — skip it.
        let Ok(url) = super::run_git(&local_path, &["remote", "get-url", "origin"]).await else {
            continue;
        };
        repos.push(super::types::MainRepo { url: url.trim().to_string(), local_path, slug });
    }
    repos.sort_by(|a, b| a.slug.to_lowercase().cmp(&b.slug.to_lowercase()));
    Ok(repos)
}

/// Clone a repo into the pool and return it. The clone can
/// be slow — it runs off the async runtime; the UI shows a pending state.
#[tauri::command]
pub async fn clone_repo(url: String) -> Result<super::types::MainRepo, String> {
    let (host, group_path, project) = parse_git_url(&url).map_err(|e| e.to_string())?;
    let dest = repo_dir(&host, &group_path, &project);
    if dest.exists() {
        return Err(format!("{} already exists", dest.display()));
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let dest_str = dest.to_string_lossy().to_string();
    let parent = dest.parent().unwrap_or(&dest).to_string_lossy().to_string();
    super::run_git(&parent, &["clone", &url, &dest_str])
        .await
        .map_err(|e| e.to_string())?;

    Ok(super::types::MainRepo {
        url,
        local_path: dest_str,
        slug: format!("{host}/{group_path}/{project}"),
    })
}

/// Remove all worktrees for a task: delete their directories and prune git references.
/// Also removes the task's worktree root directory.
pub async fn cleanup_task_worktrees(task_id: &str, pool: &SqlitePool) -> anyhow::Result<()> {
    let worktrees: Vec<Worktree> = sqlx::query_as(
        "SELECT w.* FROM worktrees w WHERE w.task_id = ?",
    )
    .bind(task_id)
    .fetch_all(pool)
    .await?;

    for wt in &worktrees {
        let wt_path = wt.path.clone();
        let repo_id = wt.repo_id.clone();

        let repo = crate::db::load::repo_opt(pool, &repo_id).await?;

        let repo_local_path = repo.map(|r| r.local_path);

        let _ = tokio::task::spawn_blocking(move || {
            let _ = std::fs::remove_dir_all(&wt_path);
            if let Some(local_path) = repo_local_path {
                let _ = std::process::Command::new("git")
                    .args(["worktree", "prune"])
                    .current_dir(&local_path)
                    .status();
            }
        })
        .await;
    }

    let dir = task_dir(task_id);
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
    let wt = crate::db::load::worktree(pool, worktree_id).await?;

    // Guard against destroying uncommitted work: unless forced, refuse to close a
    // worktree with a dirty working tree (the frontend re-invokes with force after
    // a confirm).
    if force != Some(true) {
        let dirty = super::run_git_output(&wt.path, &["status", "--porcelain"])
            .await
            .map(|o| o.status.success() && !o.stdout.is_empty())
            .unwrap_or(false);
        if dirty {
            return Err(anyhow::anyhow!(
                "worktree has uncommitted changes — commit or discard first, or force close"
            ));
        }
    }

    let wt_path = wt.path.clone();
    let repo = crate::db::load::repo_opt(pool, &wt.repo_id).await?;
    let repo_local_path = repo.map(|r| r.local_path);

    let _ = tokio::task::spawn_blocking(move || {
        let _ = std::fs::remove_dir_all(&wt_path);
        if let Some(local_path) = repo_local_path {
            let _ = std::process::Command::new("git")
                .args(["worktree", "prune"])
                .current_dir(&local_path)
                .status();
        }
    })
    .await;

    sqlx::query("DELETE FROM mrs WHERE worktree_id = ?")
        .bind(worktree_id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM worktrees WHERE id = ?")
        .bind(worktree_id)
        .execute(pool)
        .await?;

    let remaining: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM worktrees WHERE task_id = ? AND repo_id = ?")
            .bind(&wt.task_id)
            .bind(&wt.repo_id)
            .fetch_one(pool)
            .await?;

    if remaining == 0 {
        sqlx::query("DELETE FROM task_repos WHERE task_id = ? AND repo_id = ?")
            .bind(&wt.task_id)
            .bind(&wt.repo_id)
            .execute(pool)
            .await?;
    }

    app.emit(
        crate::events::WORKTREE_CLOSED,
        serde_json::json!({
            "worktree_id": worktree_id,
            "task_id": wt.task_id,
            "repo_id": wt.repo_id,
        }),
    )?;

    Ok(())
}
