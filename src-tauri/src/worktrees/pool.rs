//! The clone pool: where primary clones live, how they are found and added.

use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use sqlx::SqlitePool;

use crate::core::db::models::Repo;
use crate::core::db::store;
use crate::core::git;

/// A clone living under `<worktree_root>/main` — what the pickers list.
#[derive(Debug, Clone, Serialize)]
pub struct MainRepo {
    pub url: String,
    pub local_path: String,
    /// Path relative to the pool, host first — the picker's display name.
    pub slug: String,
}

/// The pool listing costs a filesystem walk plus one `git remote get-url` per
/// clone, and the review queue asks for it on every poll — cache it.
const POOL_TTL: Duration = Duration::from_secs(60);

type PoolSnapshot = Option<(Instant, Vec<MainRepo>)>;

static POOL: LazyLock<Mutex<PoolSnapshot>> =
    LazyLock::new(|| Mutex::new(None));

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
fn repo_dir(host: &str, group_path: &str, project: &str) -> PathBuf {
    main_root().join(host).join(group_path).join(project)
}

/// Every worktree of one session: `<root>/worktrees/<session id>`.
pub(crate) fn session_dir(session_id: &str) -> PathBuf {
    resolve_worktree_root().join("worktrees").join(session_id)
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

/// Whether `branch` already exists as a head on the repo's `origin` remote.
/// Used to block creating a worktree on a branch name that's already taken remotely.
#[tauri::command]
pub async fn remote_branch_exists(
    repo_id: String,
    branch: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<bool, String> {
    let repo = store::repos::get(&*pool, &repo_id).await.map_err(|e| e.to_string())?;

    let out = git::output(&repo.local_path, &["ls-remote", "--heads", "origin", &branch])
        .await
        .map_err(|e| e.to_string())?;

    let target = format!("refs/heads/{branch}");
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split('\t').nth(1))
        .any(|r| r == target))
}

/// All git clones in the pool, cached for `POOL_TTL` (searched a few levels
/// deep, since a host level sits above group paths that nest; never descends
/// INTO a repo). No pool → empty list.
#[tauri::command]
pub async fn list_main_repos() -> Result<Vec<MainRepo>, String> {
    if let Ok(guard) = POOL.lock() {
        if let Some((at, repos)) = guard.as_ref() {
            if at.elapsed() < POOL_TTL {
                return Ok(repos.clone());
            }
        }
    }
    let repos = scan_pool().await?;
    if let Ok(mut guard) = POOL.lock() {
        *guard = Some((Instant::now(), repos.clone()));
    }
    Ok(repos)
}

fn invalidate_pool() {
    if let Ok(mut guard) = POOL.lock() {
        *guard = None;
    }
}

async fn scan_pool() -> Result<Vec<MainRepo>, String> {
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
    // One `git remote get-url` per clone, all at once.
    let mut repos: Vec<MainRepo> =
        futures_util::future::join_all(found.into_iter().map(|path| {
            let root = root.clone();
            async move {
                let local_path = path.to_string_lossy().to_string();
                let slug = path
                    .strip_prefix(&root)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| local_path.clone());
                // A clone without an origin remote can't be registered — skip it.
                let url = git::run(&local_path, &["remote", "get-url", "origin"]).await.ok()?;
                Some(MainRepo { url: url.trim().to_string(), local_path, slug })
            }
        }))
        .await
        .into_iter()
        .flatten()
        .collect();
    repos.sort_by(|a, b| a.slug.to_lowercase().cmp(&b.slug.to_lowercase()));
    Ok(repos)
}

/// Clone a repo into the pool and return it. The clone can be slow — it runs
/// off the async runtime; the UI shows a pending state.
#[tauri::command]
pub async fn clone_repo(url: String) -> Result<MainRepo, String> {
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
    git::run(&parent, &["clone", &url, &dest_str])
        .await
        .map_err(|e| e.to_string())?;

    invalidate_pool();
    Ok(MainRepo {
        url,
        local_path: dest_str,
        slug: format!("{host}/{group_path}/{project}"),
    })
}
