//! The clone pool: where primary clones live, how they are found and added.
//!
//! The pool layout IS the identity: `<root>/main/<host>/<group>/<project>`.
//! Listing it is a pure directory walk — no git call per repo. The one
//! `remote get-url origin` check happens when a repo is attached to a session.

use std::path::PathBuf;

use serde::Serialize;
use sqlx::SqlitePool;

use crate::core::db::models::Repo;
use crate::core::db::store;
use crate::core::git;

/// A clone living under `<worktree_root>/main` — what the pickers list.
#[derive(Debug, Clone, Serialize)]
pub struct MainRepo {
    pub local_path: String,
    /// Path relative to the pool, host first — both the display name and the
    /// identity: `<host>/<group…>/<project>`.
    pub slug: String,
}

/// `(host, group_path, project)` read off a pool slug.
pub(crate) fn slug_parts(slug: &str) -> anyhow::Result<(String, String, String)> {
    let mut segments: Vec<&str> = slug.split('/').filter(|s| !s.is_empty()).collect();
    if segments.len() < 3 {
        return Err(anyhow::anyhow!("'{slug}' is not a <host>/<group>/<project> pool path"));
    }
    let project = segments.pop().unwrap().to_string();
    let host = segments.remove(0).to_string();
    Ok((host, segments.join("/"), project))
}

pub fn resolve_worktree_root() -> PathBuf {
    // Honor the configured root (the agent cwd already does) — tilde-expanded,
    // since the config stores it unexpanded.
    if let Some(cfg) = crate::core::config::get() {
        let raw = cfg.git.worktree_root;
        if !raw.trim().is_empty() {
            return PathBuf::from(crate::core::fs::expand_tilde(&raw));
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
    slug: String,
    local_path: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<Repo, String> {
    register_repo_impl(&slug, local_path, &pool)
        .await
        .map_err(|e| e.to_string())
}

/// Record a pool clone in the DB so a session can use it. The identity comes
/// from its place in the pool; the one git call verifies it has an `origin`
/// (every forge feature needs one) and happens here, at attach — never during
/// a listing.
pub(crate) async fn register_repo_impl(
    slug: &str,
    local_path: String,
    pool: &SqlitePool,
) -> anyhow::Result<Repo> {
    let (host, group_path, project) = slug_parts(slug)?;

    git::run(&local_path, &["remote", "get-url", "origin"])
        .await
        .map_err(|_| anyhow::anyhow!("{local_path} has no `origin` remote — forge features need one"))?;

    let repo = Repo {
        id: format!("{host}/{group_path}/{project}"),
        host,
        group_path,
        project,
        local_path,
    };
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

/// The branches a worktree can be based on, plus which one the repo defaults to.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct OriginBranches {
    pub branches: Vec<String>,
    /// None when origin never set a default; the caller then preselects nothing.
    pub default_branch: Option<String>,
}

/// Origin's branch heads, for the base-branch pickers.
#[tauri::command]
pub async fn list_origin_branches(
    repo_id: String,
    pool: tauri::State<'_, SqlitePool>,
) -> Result<OriginBranches, String> {
    let repo = store::repos::get(&*pool, &repo_id).await.map_err(|e| e.to_string())?;
    let branches = git::refs::origin_branches(&repo.local_path)
        .await
        .map_err(|e| e.to_string())?;
    let default_branch = git::refs::default_branch(&repo.local_path).await;
    Ok(OriginBranches { branches, default_branch })
}

/// Every clone in the pool: a pure directory walk, no git calls (searched a
/// few levels deep, since a host level sits above group paths that nest; never
/// descends INTO a repo). No pool → empty list.
#[tauri::command]
pub async fn list_main_repos() -> Result<Vec<MainRepo>, String> {
    let root = main_root();
    // On a blocking thread — it's pure filesystem.
    let mut repos: Vec<MainRepo> = tokio::task::spawn_blocking(move || {
        fn walk(dir: &std::path::Path, root: &std::path::Path, depth: u32, acc: &mut Vec<MainRepo>) {
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
                    let Ok(slug) = path.strip_prefix(root) else { continue };
                    acc.push(MainRepo {
                        local_path: path.to_string_lossy().to_string(),
                        slug: slug.to_string_lossy().to_string(),
                    });
                } else {
                    walk(&path, root, depth + 1, acc);
                }
            }
        }
        let mut acc = vec![];
        walk(&root, &root, 1, &mut acc);
        acc
    })
    .await
    .map_err(|e| e.to_string())?;
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

    Ok(MainRepo {
        local_path: dest_str,
        slug: format!("{host}/{group_path}/{project}"),
    })
}

#[cfg(test)]
mod tests {
    use super::slug_parts;

    #[test]
    fn slugs_split_into_host_group_and_project() {
        assert_eq!(
            slug_parts("gitlab.example.com/wiremind/devops/mayo").unwrap(),
            ("gitlab.example.com".into(), "wiremind/devops".into(), "mayo".into())
        );
        assert_eq!(
            slug_parts("github.com/owner/proj").unwrap(),
            ("github.com".into(), "owner".into(), "proj".into())
        );
        assert!(slug_parts("just/two").is_err());
    }
}
