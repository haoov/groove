use groove_git::{Error as GitError, Git};
use groove_types::{Repo, Session, Timestamp, Worktree, WorktreeSpec, names_session};

use crate::naming::{default_branch, validate_branch_name};
use crate::{Error, Pool, Result};

/// The worktree, and what happened on the way that the user should hear.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provisioned {
    pub worktree: Worktree,
    /// A local branch of this name existed and the session continues on it.
    pub adopted: bool,
    pub notes: Vec<String>,
}

impl Pool {
    /// Fetch the clone, cut the branch when needed, add the worktree, record it.
    /// Idempotent: the session's existing worktree for the branch is aligned and returned.
    pub async fn provision(
        &self,
        session: &Session,
        repo: &Repo,
        spec: &WorktreeSpec,
        tag: Option<&str>,
    ) -> Result<Provisioned> {
        let branch = spec
            .branch
            .clone()
            .unwrap_or_else(|| default_branch(session, tag));
        validate_branch_name(&branch)?;
        let clone = Git::at(&repo.local_path);
        let mut notes = Vec::new();

        let existing = self
            .worktree_for_branch(&session.id, &repo.id, &branch)
            .await?;
        if let Some(existing) = existing.filter(|w| std::path::Path::new(&w.path).is_dir()) {
            align(&Git::at(&existing.path), &branch).await?;
            return Ok(Provisioned {
                worktree: existing,
                adopted: false,
                notes,
            });
        }
        if !clone.is_repository().await? {
            return Err(Error::NotFound {
                what: "repository",
                id: repo.local_path.clone(),
            });
        }
        clone.worktree_prune().await?;
        let default = refresh(&clone, &repo.project, &mut notes).await;
        let point = branch_point(&clone, spec.target.as_deref(), default.as_deref()).await?;

        let path = self
            .layout
            .worktree_dir(session.id.as_str(), &repo.project, &branch);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| Error::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let local = clone.ref_exists(&format!("refs/heads/{branch}")).await?;
        let mut adopted = false;
        match (&spec.track_remote, local) {
            (Some(remote), false) => clone.worktree_add_tracking(&path, &branch, remote).await?,
            (_, true) => {
                if spec.branch.is_none() && !names_session(&branch, session, tag) {
                    return Err(Error::ForeignBranch {
                        repo: repo.project.clone(),
                        branch,
                    });
                }
                adopted = true;
                notes.push(format!(
                    "{}: continuing on the existing branch {branch}",
                    repo.project
                ));
                add_or_align(&clone, &path, &branch).await?;
            }
            (None, false) => {
                clone.branch_create(&branch, &point).await?;
                add_or_align(&clone, &path, &branch).await?;
            }
        }

        let worktree = self
            .upsert_worktree(
                &session.id,
                &repo.id,
                &branch,
                &path.to_string_lossy(),
                spec.target.as_deref(),
                Timestamp::now(),
            )
            .await?;
        Ok(Provisioned {
            worktree,
            adopted,
            notes,
        })
    }
}

/// `origin/<target>` when given and present, else the repo's default, then main and master.
async fn branch_point(clone: &Git, target: Option<&str>, default: Option<&str>) -> Result<String> {
    if let Some(target) = target {
        let point = format!("origin/{target}");
        if clone.ref_exists(&point).await? {
            return Ok(point);
        }
        return Err(Error::NoTarget {
            target: target.to_string(),
            available: clone.remote_heads().await.unwrap_or_default(),
        });
    }
    Ok(clone.base_ref(default).await?)
}

/// Fetches origin and moves the clone's default branch forward. Every failure is a note,
/// not an error: a new branch still comes from `origin/<default>`.
async fn refresh(clone: &Git, label: &str, notes: &mut Vec<String>) -> Option<String> {
    if let Err(e) = clone.fetch(&[]).await {
        notes.push(format!(
            "{label}: could not fetch origin, the worktree may start from stale history: {e}"
        ));
    }
    let default = clone.default_branch().await.ok().flatten()?;
    let current = clone.current_branch().await.unwrap_or_default();
    let moved = if current == default {
        clone.pull().await
    } else {
        clone.fetch(&[&format!("{default}:{default}")]).await
    };
    if let Err(e) = moved {
        notes.push(format!(
            "{label}: {default} in the clone could not fast-forward: {e}"
        ));
    }
    Some(default)
}

/// `worktree add`, or align the checkout when the directory already holds one.
async fn add_or_align(clone: &Git, path: &std::path::Path, branch: &str) -> Result<()> {
    match clone.worktree_add(path, branch).await {
        Ok(()) => Ok(()),
        Err(GitError::AlreadyExists { .. }) => align(&Git::at(path), branch).await,
        Err(e) => Err(e.into()),
    }
}

/// Switches a checkout to `branch` when it sits on another one.
async fn align(worktree: &Git, branch: &str) -> Result<()> {
    if worktree.current_branch().await? == branch {
        return Ok(());
    }
    if worktree.switch(branch, false).await.is_ok() {
        return Ok(());
    }
    Ok(worktree.switch(branch, true).await?)
}
