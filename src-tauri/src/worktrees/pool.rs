//! The clone pool: `<root>/main/<host>/<group>/<project>`, listed by a directory walk.

use std::path::PathBuf;

use serde::Serialize;
use sqlx::SqlitePool;

use crate::core::db::models::Repo;
use crate::core::db::store;
use crate::core::error::{AppError, AppResult, ErrorKind};
use crate::core::git;

/// A clone living under `<worktree_root>/main` — what the pickers list.
#[derive(Debug, Clone, Serialize)]
pub struct MainRepo {
    pub local_path: String,
    /// Path relative to the pool, host first: `<host>/<group…>/<project>`.
    pub slug: String,
}

/// `(host, group_path, project)` read off a pool slug.
pub(crate) fn slug_parts(slug: &str) -> anyhow::Result<(String, String, String)> {
    let mut segments: Vec<&str> = slug.split('/').filter(|s| !s.is_empty()).collect();
    if segments.len() < 3 {
        return Err(anyhow::anyhow!(
            "'{slug}' is not a <host>/<group>/<project> pool path"
        ));
    }
    let project = segments.pop().unwrap().to_string();
    let host = segments.remove(0).to_string();
    Ok((host, segments.join("/"), project))
}

pub fn resolve_worktree_root() -> PathBuf {
    // The config stores the root unexpanded.
    if let Some(cfg) = crate::core::config::get() {
        let raw = cfg.git.worktree_root;
        if !raw.trim().is_empty() {
            return PathBuf::from(crate::core::fs::expand_tilde(&raw));
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join("worktrees")
}

/// Where the primary clones live.
pub(crate) fn main_root() -> PathBuf {
    resolve_worktree_root().join("main")
}

/// A clone's place in the pool: `<root>/main/<host>/<group>/<project>`.
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
) -> AppResult<Repo> {
    Ok(register_repo_impl(&slug, local_path, &pool).await?)
}

/// Record a pool clone in the DB. The one git call checks that it has an `origin`.
#[tracing::instrument(skip_all, fields(repo = slug))]
pub(crate) async fn register_repo_impl(
    slug: &str,
    local_path: String,
    pool: &SqlitePool,
) -> anyhow::Result<Repo> {
    let (host, group_path, project) = slug_parts(slug)?;

    git::run(&local_path, &["remote", "get-url", "origin"])
        .await
        .map_err(|_| {
            anyhow::anyhow!("{local_path} has no `origin` remote — forge features need one")
        })?;

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

/// Whether `branch` exists as a head on the repo's `origin`.
#[tauri::command]
pub async fn remote_branch_exists(
    repo_id: String,
    branch: String,
    pool: tauri::State<'_, SqlitePool>,
) -> AppResult<bool> {
    let repo = store::repos::get(&*pool, &repo_id).await?;

    let out = git::output(
        &repo.local_path,
        &["ls-remote", "--heads", "origin", &branch],
    )
    .await
    .map_err(|e| AppError::from(e).with_kind(ErrorKind::Git))?;

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
) -> AppResult<OriginBranches> {
    let repo = store::repos::get(&*pool, &repo_id).await?;
    let branches = git::refs::origin_branches(&repo.local_path)
        .await
        .map_err(|e| AppError::from(e).with_kind(ErrorKind::Git))?;
    let default_branch = git::refs::default_branch(&repo.local_path).await;
    Ok(OriginBranches {
        branches,
        default_branch,
    })
}

/// Every clone in the pool, by directory walk; no pool gives an empty list.
#[tauri::command]
pub async fn list_main_repos() -> AppResult<Vec<MainRepo>> {
    let root = main_root();
    let mut repos: Vec<MainRepo> = tokio::task::spawn_blocking(move || {
        fn walk(
            dir: &std::path::Path,
            root: &std::path::Path,
            depth: u32,
            acc: &mut Vec<MainRepo>,
        ) {
            if depth > 6 {
                return;
            }
            let Ok(entries) = std::fs::read_dir(dir) else {
                return;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                if path.join(".git").exists() {
                    let Ok(slug) = path.strip_prefix(root) else {
                        continue;
                    };
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
    .map_err(|e| AppError::internal(e.to_string()))?;
    repos.sort_by(|a, b| a.slug.to_lowercase().cmp(&b.slug.to_lowercase()));
    Ok(repos)
}

/// Clone a repo into the pool and return it.
#[tauri::command]
pub async fn clone_repo(url: String) -> AppResult<MainRepo> {
    let (host, group_path, project) =
        git::parse_git_url(&url).map_err(|e| AppError::from(e).with_kind(ErrorKind::Invalid))?;
    let dest = repo_dir(&host, &group_path, &project);
    if dest.exists() {
        return Err(AppError::conflict(format!(
            "{} already exists",
            dest.display()
        )));
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let dest_str = dest.to_string_lossy().to_string();
    let parent = dest.parent().unwrap_or(&dest).to_string_lossy().to_string();
    git::run(&parent, &["clone", &url, &dest_str])
        .await
        .map_err(|e| AppError::from(e).with_kind(ErrorKind::Git))?;

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
            (
                "gitlab.example.com".into(),
                "wiremind/devops".into(),
                "mayo".into()
            )
        );
        assert_eq!(
            slug_parts("github.com/owner/proj").unwrap(),
            ("github.com".into(), "owner".into(), "proj".into())
        );
        assert!(slug_parts("just/two").is_err());
    }
}
