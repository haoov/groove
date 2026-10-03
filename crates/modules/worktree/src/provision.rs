//! Cutting a worktree for a session: its branch, its directory, its row.

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
    /// Fetch, cut the branch if needed, add and record the worktree; an existing one is aligned.
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

        if let Some(worktree) = self.standing(session, repo, &branch).await? {
            return Ok(Provisioned {
                worktree,
                adopted: false,
                notes,
            });
        }
        let point = fetched(&clone, repo, spec, &mut notes).await?;
        let path = self
            .layout
            .worktree_dir(session.id.as_str(), &repo.project, &branch);
        make_parent(&path)?;
        let adopted = cut(
            &clone,
            &path,
            Cutting {
                session,
                repo,
                spec,
                tag,
                branch: &branch,
                point: &point,
            },
            &mut notes,
        )
        .await?;
        let worktree = self
            .recorded(session, repo, &branch, &path, spec.target.as_deref())
            .await?;
        Ok(Provisioned {
            worktree,
            adopted,
            notes,
        })
    }

    /// The worktree as the database now holds it.
    async fn recorded(
        &self,
        session: &Session,
        repo: &Repo,
        branch: &str,
        path: &std::path::Path,
        target: Option<&str>,
    ) -> Result<Worktree> {
        self.upsert_worktree(
            &session.id,
            &repo.id,
            branch,
            &path.to_string_lossy(),
            target,
            Timestamp::now(),
        )
        .await
    }

    /// The session's worktree for this branch, when it is still on disk, aligned.
    async fn standing(
        &self,
        session: &Session,
        repo: &Repo,
        branch: &str,
    ) -> Result<Option<Worktree>> {
        let existing = self
            .worktree_for_branch(&session.id, &repo.id, branch)
            .await?;
        let Some(existing) = existing.filter(|w| std::path::Path::new(&w.path).is_dir()) else {
            return Ok(None);
        };
        align(&Git::at(&existing.path), branch).await?;
        Ok(Some(existing))
    }
}

/// The clone brought up to date, and the commit a new branch would start from.
async fn fetched(
    clone: &Git,
    repo: &Repo,
    spec: &WorktreeSpec,
    notes: &mut Vec<String>,
) -> Result<String> {
    if !clone.is_repository().await? {
        return Err(Error::NotFound {
            what: "repository",
            id: repo.local_path.clone(),
        });
    }
    clone.worktree_prune().await?;
    let default = refresh(clone, &repo.project, notes).await;
    branch_point(clone, spec.target.as_deref(), default.as_deref()).await
}

/// What cutting a worktree needs to know about the branch it stands on.
struct Cutting<'a> {
    session: &'a Session,
    repo: &'a Repo,
    spec: &'a WorktreeSpec,
    tag: Option<&'a str>,
    branch: &'a str,
    point: &'a str,
}

fn make_parent(path: &std::path::Path) -> Result<()> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    std::fs::create_dir_all(parent).map_err(|source| Error::Io {
        path: parent.to_path_buf(),
        source,
    })
}

/// Adds the worktree, on a branch tracked, adopted or newly cut. True when adopted.
async fn cut(
    clone: &Git,
    path: &std::path::Path,
    at: Cutting<'_>,
    notes: &mut Vec<String>,
) -> Result<bool> {
    let Cutting {
        session,
        repo,
        spec,
        tag,
        branch,
        point,
    } = at;
    let local = clone.ref_exists(&format!("refs/heads/{branch}")).await?;
    match (&spec.track_remote, local) {
        (Some(remote), false) => {
            clone.worktree_add_tracking(path, branch, remote).await?;
            Ok(false)
        }
        (_, true) => {
            if spec.branch.is_none() && !names_session(branch, session, tag) {
                return Err(Error::ForeignBranch {
                    repo: repo.project.clone(),
                    branch: branch.to_string(),
                });
            }
            notes.push(format!(
                "{}: continuing on the existing branch {branch}",
                repo.project
            ));
            add_or_align(clone, path, branch).await?;
            Ok(true)
        }
        (None, false) => {
            clone.branch_create(branch, point).await?;
            add_or_align(clone, path, branch).await?;
            Ok(false)
        }
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

/// Every remote-tracking ref, in one fetch.
const TRACKING: &str = "+refs/heads/*:refs/remotes/origin/*";

/// Reads origin once and moves the clone's default branch forward with it. A failure is a note.
async fn refresh(clone: &Git, label: &str, notes: &mut Vec<String>) -> Option<String> {
    let default = clone.default_branch().await.ok().flatten();
    let current = clone.current_branch().await.unwrap_or_default();
    let read = match &default {
        Some(name) if *name != current => clone.fetch(&[TRACKING, &format!("{name}:{name}")]).await,
        Some(_) => clone.pull().await,
        None => clone.fetch(&[]).await,
    };
    if let Err(e) = read {
        notes.push(format!(
            "{label}: could not read origin, the worktree may start from stale history: {e}"
        ));
    }
    default
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
    let made = worktree.ref_exists(&format!("refs/heads/{branch}")).await?;
    Ok(worktree.switch(branch, !made).await?)
}
